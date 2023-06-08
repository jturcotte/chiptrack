// Copyright © SixtyFPS GmbH <info@slint-ui.com>
// SPDX-License-Identifier: GPL-3.0-only OR LicenseRef-Slint-commercial

extern crate alloc;

use crate::log;
use crate::sound_renderer::SoundRenderer;
use crate::GlobalEngine;
use crate::GlobalUI;
use crate::MainWindow;
use crate::MidiNote;
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::rc::Weak;
use core::cell::Cell;
use core::pin::Pin;
use i_slint_core::model::ModelChangeListenerContainer;
use i_slint_core::renderer::Renderer;
use slint::platform::software_renderer::RepaintBufferType;
use slint::platform::software_renderer::SoftwareRenderer;
use slint::Brush::SolidColor;
use slint::Global;
use slint::Model;
use slint::PlatformError;
use slint::SharedString;
use slint::Window;

use core::cell::RefCell;
use embedded_alloc::Heap;

use slint::platform::WindowEvent;

use core::fmt::Write;
use gba::{
    mgba::{MgbaBufferedLogger, MgbaMessageLevel},
    prelude::*,
};

#[cfg(feature = "panic-probe")]
use panic_probe as _;

#[alloc_error_handler]
fn oom(layout: core::alloc::Layout) -> ! {
    panic!("Out of memory {:?}", layout);
}

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    if let Ok(mut logger) = MgbaBufferedLogger::try_new(MgbaMessageLevel::Fatal) {
        write!(logger, "{info}").ok();
    }
    loop {}
}

const HEAP_SIZE: usize = 256 * 1024;
// static mut HEAP: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: Heap = Heap::empty();

const DISPLAY_SIZE: slint::PhysicalSize = slint::PhysicalSize::new(240, 160);

const KEY_A: u16 = 0b00_00000001;
const KEY_B: u16 = 0b00_00000010;
const KEY_SELECT: u16 = 0b00_00000100;
const KEY_START: u16 = 0b00_00001000;
const KEY_RIGHT: u16 = 0b00_00010000;
const KEY_LEFT: u16 = 0b00_00100000;
const KEY_UP: u16 = 0b00_01000000;
const KEY_DOWN: u16 = 0b00_10000000;
const KEY_R: u16 = 0b01_00000000;
const KEY_L: u16 = 0b10_00000000;

const NORMAL_TEXT: u16 = 0;
const FADED_TEXT: u16 = 1;

// This is a type alias for the enabled `restore-state-*` feature.
// For example, it is `bool` if you enable `restore-state-bool`.
use critical_section::RawRestoreState;

struct GbaCriticalSection;
critical_section::set_impl!(GbaCriticalSection);

unsafe impl critical_section::Impl for GbaCriticalSection {
    unsafe fn acquire() -> RawRestoreState {
        true
    }
    unsafe fn release(_token: RawRestoreState) {}
}

pub struct MinimalGbaWindow {
    window: i_slint_core::api::Window,
    renderer: SoftwareRenderer,
    needs_redraw: Cell<bool>,
    sequencer_patterns_tracker: Pin<Box<i_slint_core::model::ModelChangeListenerContainer<ModelDirtinessTracker>>>,
    sequencer_song_patterns_tracker: Pin<Box<i_slint_core::model::ModelChangeListenerContainer<ModelDirtinessTracker>>>,
    sequencer_steps_tracker: Pin<Box<i_slint_core::model::ModelChangeListenerContainer<ModelDirtinessTracker>>>,
    sequencer_pattern_instruments_tracker:
        Pin<Box<i_slint_core::model::ModelChangeListenerContainer<ModelDirtinessTracker>>>,
    instruments_tracker: Pin<Box<i_slint_core::model::ModelChangeListenerContainer<ModelDirtinessTracker>>>,
    was_in_song_mode: RefCell<bool>,
    was_in_instruments_grid: RefCell<bool>,
    sequencer_song_pattern_active_previous: RefCell<usize>,
    sequencer_pattern_active_previous: RefCell<usize>,
    sequencer_step_active_previous: RefCell<usize>,
    current_instrument_previous: RefCell<usize>,
}

struct ModelDirtinessTracker {
    is_dirty: RefCell<bool>,
}

impl Default for ModelDirtinessTracker {
    fn default() -> Self {
        ModelDirtinessTracker {
            is_dirty: RefCell::new(true),
        }
    }
}

impl ModelDirtinessTracker {
    fn take_dirtiness(&self) -> bool {
        self.is_dirty.replace(false)
    }
}

impl i_slint_core::model::ModelChangeListener for ModelDirtinessTracker {
    fn row_changed(&self, _row: usize) {
        *self.is_dirty.borrow_mut() = true;
    }

    fn row_added(&self, _index: usize, _count: usize) {
        *self.is_dirty.borrow_mut() = true;
    }

    fn row_removed(&self, _index: usize, _count: usize) {
        *self.is_dirty.borrow_mut() = true;
    }
    fn reset(&self) {
        *self.is_dirty.borrow_mut() = true;
    }
}

impl MinimalGbaWindow {
    /// Instantiate a new MinimalWindowAdaptor
    pub fn new() -> Rc<Self> {
        {
            // get our tile data into memory.
            Cga8x8Thick.bitunpack_4bpp(CHARBLOCK0_4BPP.as_region(), 0);
        }

        BG0CNT.write(BackgroundControl::new().with_screenblock(31));
        DISPCNT.write(
            DisplayControl::new()
                // .with_video_mode(VideoMode::_0)
                .with_show_bg0(true),
        );
        bg_palbank(NORMAL_TEXT as usize).index(1).write(Color::BLACK);
        bg_palbank(FADED_TEXT as usize).index(1).write(Color(0b0_11010_11010_11010));

        Rc::new_cyclic(|w: &Weak<Self>| Self {
            window: Window::new(w.clone()),
            renderer: SoftwareRenderer::new(RepaintBufferType::NewBuffer, w.clone()),
            needs_redraw: Default::default(),
            sequencer_patterns_tracker: Box::pin(ModelChangeListenerContainer::<ModelDirtinessTracker>::default()),
            sequencer_song_patterns_tracker: Box::pin(ModelChangeListenerContainer::<ModelDirtinessTracker>::default()),
            sequencer_steps_tracker: Box::pin(ModelChangeListenerContainer::<ModelDirtinessTracker>::default()),
            sequencer_pattern_instruments_tracker: Box::pin(
                ModelChangeListenerContainer::<ModelDirtinessTracker>::default(),
            ),
            instruments_tracker: Box::pin(ModelChangeListenerContainer::<ModelDirtinessTracker>::default()),
            was_in_song_mode: RefCell::new(false),
            was_in_instruments_grid: RefCell::new(false),
            sequencer_song_pattern_active_previous: RefCell::new(0),
            sequencer_pattern_active_previous: RefCell::new(0),
            sequencer_step_active_previous: RefCell::new(0),
            current_instrument_previous: RefCell::new(0),
        })
    }

    fn attach_trackers(&self) {
        let handle = unsafe { WINDOW.as_ref().unwrap().upgrade().unwrap() };
        GlobalEngine::get(&handle)
            .get_sequencer_patterns()
            .model_tracker()
            .attach_peer(Pin::as_ref(&self.sequencer_patterns_tracker).model_peer());
        GlobalEngine::get(&handle)
            .get_sequencer_song_patterns()
            .model_tracker()
            .attach_peer(Pin::as_ref(&self.sequencer_song_patterns_tracker).model_peer());
        GlobalEngine::get(&handle)
            .get_sequencer_steps()
            .model_tracker()
            .attach_peer(Pin::as_ref(&self.sequencer_steps_tracker).model_peer());
        GlobalEngine::get(&handle)
            .get_sequencer_pattern_instruments()
            .model_tracker()
            .attach_peer(Pin::as_ref(&self.sequencer_pattern_instruments_tracker).model_peer());
        GlobalEngine::get(&handle)
            .get_instruments()
            .model_tracker()
            .attach_peer(Pin::as_ref(&self.instruments_tracker).model_peer());
        // Start dirty
        *self.was_in_song_mode.borrow_mut() = !GlobalUI::get(&handle).get_song_mode();
        *self.was_in_instruments_grid.borrow_mut() = !GlobalUI::get(&handle).get_instruments_grid();
    }

    pub fn draw_if_needed(&self, render_callback: impl FnOnce(&SoftwareRenderer)) -> bool {
        // FIXME: Check if this could be casted from the component of self somehow
        let handle = unsafe { WINDOW.as_ref().unwrap().upgrade().unwrap() };
        let global_engine = GlobalEngine::get(&handle);
        let global_ui = GlobalUI::get(&handle);
        let song_mode = global_ui.get_song_mode();
        let song_mode_dirty = self.was_in_song_mode.replace(song_mode) != song_mode;
        let instruments_grid = global_ui.get_instruments_grid();
        let instruments_grid_dirty = self.was_in_instruments_grid.replace(instruments_grid) != instruments_grid;
        let sequencer_song_pattern_active = global_engine.get_sequencer_song_pattern_active() as usize;
        let sequencer_song_pattern_active_dirty = self.sequencer_song_pattern_active_previous.replace(sequencer_song_pattern_active) != sequencer_song_pattern_active;
        let sequencer_pattern_active = global_engine.get_sequencer_pattern_active() as usize;
        let sequencer_pattern_active_dirty = self.sequencer_pattern_active_previous.replace(sequencer_pattern_active) != sequencer_pattern_active;
        let sequencer_step_active = global_engine.get_sequencer_step_active() as usize;
        let sequencer_step_active_dirty = self.sequencer_step_active_previous.replace(sequencer_step_active) != sequencer_step_active;
        let current_instrument = global_engine.get_current_instrument() as usize;
        let current_instrument_dirty = self.current_instrument_previous.replace(current_instrument) != current_instrument;

        let tsb = TEXT_SCREENBLOCKS.get_frame(31).unwrap();

        let status_vid_row = tsb.get_row(18).unwrap();
        status_vid_row
            .index(0)
            .write(TextEntry::new().with_tile(handle.get_patterns_have_focus() as u16 * 7));
        status_vid_row
            .index(6)
            .write(TextEntry::new().with_tile(handle.get_steps_have_focus() as u16 * 7));

        if song_mode_dirty {
            let vid_row = tsb.get_row(0).unwrap();
            let s = if song_mode { "Song" } else { "Patt" };
            vid_row
                .iter()
                .zip(s.chars())
                .for_each(|(row, c)| row.write(TextEntry::new().with_tile(c as u16)));
        }
        let dirty_pattern_model = if !song_mode && (song_mode_dirty || sequencer_pattern_active_dirty || self.sequencer_patterns_tracker.take_dirtiness())
        {
            Some((global_engine.get_sequencer_patterns(), sequencer_pattern_active))
        } else if song_mode && (song_mode_dirty || sequencer_song_pattern_active_dirty || self.sequencer_song_patterns_tracker.take_dirtiness()) {
            Some((global_engine.get_sequencer_song_patterns(), sequencer_song_pattern_active))
        } else {
            None
        };
        if let Some((pattern_model, active_index)) = dirty_pattern_model {
            let scroll_pos = active_index.max(8).min(pattern_model.row_count() - 8) - 8;
            for i in scroll_pos..(pattern_model.row_count().min(scroll_pos + 16)) {
                let vid_row = tsb.get_row(i - scroll_pos + 1).unwrap();
                let row_data = pattern_model.row_data(i).unwrap();
                let number = row_data.number + 1;
                let c1 = (number / 10) as u8 + '0' as u8;
                vid_row.index(1).write(TextEntry::new().with_tile(c1 as u16));
                let c2 = (number % 10) as u8 + '0' as u8;
                vid_row.index(2).write(TextEntry::new().with_tile(c2 as u16));
                vid_row
                    .index(0)
                    .write(TextEntry::new().with_tile((i == active_index) as u16 * 7));
            }
        }

        if self.sequencer_steps_tracker.take_dirtiness() || sequencer_step_active_dirty {
            let current_instrument = global_engine.get_current_instrument() as usize;
            let vid_row = tsb.get_row(0).unwrap();
            let current_instrument_id = global_engine.get_instruments().row_data(current_instrument).unwrap().id;
            vid_row.iter_range(6..6+3)
                .zip(current_instrument_id.chars().chain(core::iter::repeat(' ')))
                .for_each(|(row, c)| row.write(TextEntry::new().with_tile(c as u16)));

            let sequencer_steps = global_engine.get_sequencer_steps();
            for i in 0..sequencer_steps.row_count() {
                let vid_row = tsb.get_row(i + 1).unwrap();
                let row_data = sequencer_steps.row_data(i).unwrap();
                for (j, &c) in MidiNote(row_data.note).char_desc().iter().enumerate() {
                    let t = if row_data.press {
                        TextEntry::new().with_tile(c as u16)
                    } else {
                        TextEntry::new().with_tile('-' as u16).with_palbank(FADED_TEXT)
                    };
                    vid_row.index(j + 6).write(t);
                }
                vid_row
                    .index(4)
                    .write(TextEntry::new().with_tile((i == sequencer_step_active) as u16 * 7));
                vid_row
                    .index(5)
                    .write(TextEntry::new().with_tile(row_data.press as u16 * '[' as u16));
                vid_row
                    .index(9)
                    .write(TextEntry::new().with_tile(row_data.release as u16 * ']' as u16));
            }
        }

        if instruments_grid_dirty {
            for i in 0..17 {
                tsb.get_row(i)
                    .unwrap()
                    .iter_range(11..)
                    .for_each(|a| a.write(TextEntry::new()));
            }
            if instruments_grid {
                tsb.get_row(0)
                    .unwrap()
                    .iter_range(11..)
                    .zip("Instruments".chars())
                    .for_each(|(row, c)| row.write(TextEntry::new().with_tile(c as u16)));
            }
        }
        if !instruments_grid && (instruments_grid_dirty || self.sequencer_pattern_instruments_tracker.take_dirtiness())
        {
            let top_vid_row = tsb.get_row(0).unwrap();
            let sequencer_pattern_instruments_len = global_engine.get_sequencer_pattern_instruments_len() as usize;
            let sequencer_pattern_instruments = global_engine.get_sequencer_pattern_instruments();
            let placeholder = TextEntry::new().with_tile('-' as u16).with_palbank(FADED_TEXT);
            for i in 0..6 {
                if i < sequencer_pattern_instruments_len {
                    let row_data = sequencer_pattern_instruments.row_data(i).unwrap();

                    top_vid_row.index(i * 3 + 11 + 1).write(TextEntry::new());
                    top_vid_row.index(i * 3 + 11 + 2).write(TextEntry::new());
                    for (ci, c) in row_data.id.chars().enumerate() {
                        top_vid_row
                            .index(i * 3 + 11 + ci)
                            .write(TextEntry::new().with_tile(c as u16));
                    }

                    let notes = row_data.notes;
                    for j in 0..notes.row_count() {
                        let note = notes.row_data(j).unwrap();
                        let vid_row = tsb.get_row(j + 1).unwrap();
                        vid_row.index(i * 3 + 11).write(placeholder);
                        vid_row.index(i * 3 + 11 + 1).write(placeholder);

                        if note != -1 {
                            let midi_note = MidiNote(note);
                            vid_row.index(i * 3 + 11).write(TextEntry::new().with_tile(midi_note.base_note_name() as u16));
                            if midi_note.is_black() {
                                vid_row.index(i * 3 + 11 + 1).write(TextEntry::new().with_tile('#' as u16));
                            }
                        };
                    }
                } else {
                    top_vid_row.index(i * 3 + 11).write(placeholder);
                    top_vid_row.index(i * 3 + 11 + 1).write(placeholder);
                    top_vid_row.index(i * 3 + 11 + 2).write(TextEntry::new());
                    for j in 1..17 {
                        tsb.get_row(j).unwrap().index(i * 3 + 11).write(placeholder);
                        tsb.get_row(j).unwrap().index(i * 3 + 11 + 1).write(placeholder);
                    }
                }
            }
        }
        if instruments_grid && (instruments_grid_dirty || current_instrument_dirty || self.instruments_tracker.take_dirtiness()) {
            let instruments = global_engine.get_instruments();
            let scroll_pos = (current_instrument / 4).max(2).min(instruments.row_count() / 4 - 2) - 2;
            for y in scroll_pos..scroll_pos+4 {
                let vid_row = tsb.get_row((y - scroll_pos) * 4 + 2).unwrap();
                let sel_vid_row = tsb.get_row((y - scroll_pos) * 4 + 3).unwrap();
                for x in 0..4 {
                    let instrument_idx = y * 4 + x;
                    let instrument = instruments.row_data(instrument_idx).unwrap();
                    vid_row
                        .index(x * 4 + 11)
                        .write(TextEntry::new().with_tile(instrument.active as u16 * 7));
                    let col = x * 4 + 11 + 1;
                    vid_row
                        .iter_range(col..col+4)
                        .zip(instrument.id.chars().chain(core::iter::repeat(' ')))
                        .for_each(|(row, c)| row.write(TextEntry::new().with_tile(c as u16)));
                    let sel_char = TextEntry::new().with_tile((instrument_idx == current_instrument) as u16 * '-' as u16);
                    sel_vid_row
                        .iter_range(x * 4 + 11..x * 4 + 11 + 4)
                        .for_each(|a| a.write(sel_char));
                }
            }
        }
        if self.needs_redraw.replace(false) {
            render_callback(&self.renderer);
            true
        } else {
            false
        }
    }
}

impl i_slint_core::window::WindowAdapterSealed for MinimalGbaWindow {
    fn request_redraw(&self) {
        self.needs_redraw.set(true);
    }
    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }

    fn apply_window_properties(&self, window_item: Pin<&i_slint_core::items::WindowItem>) {
        i_slint_core::debug_log!("window_item.background: {:?}", window_item.background());
        if let SolidColor(color) = window_item.background() {
            let bgr555 = ((color.red() as u16) >> 3)
                | (((color.green() as u16) & 0xf8) << 2)
                | (((color.blue() as u16) & 0xf8) << 7);

            BACKDROP_COLOR.write(Color(bgr555));
        }
    }
}

impl i_slint_core::window::WindowAdapter for MinimalGbaWindow {
    fn window(&self) -> &i_slint_core::api::Window {
        &self.window
    }
}

impl core::ops::Deref for MinimalGbaWindow {
    type Target = Window;
    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

struct GbaPlatform {
    window: Rc<MinimalGbaWindow>,
}

static mut LAST_TIMER3_READ: u16 = 0;
static mut BASE_MILLIS_SINCE_START: u32 = 0;
// FIXME: Use GbaCell just to avoid unsafe?
static mut SOUND_RENDERER: Option<Rc<RefCell<SoundRenderer>>> = None;
static mut WINDOW: Option<slint::Weak<MainWindow>> = None;

// /// A 16bit pixel that has 5 blue bits, 5 green bits and  5 red bits
// #[repr(transparent)]
// #[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
// pub struct Bgr555Pixel(pub u16);

// impl Bgr555Pixel {
//     const B_MASK: u16 = 0b01111100_00000000;
//     const G_MASK: u16 = 0b00000011_11100000;
//     const R_MASK: u16 = 0b00000000_00011111;
// }

// impl TargetPixel for Bgr555Pixel {
//     #[link_section = ".iwram"]
//     fn blend(&mut self, color: PremultipliedRgbaColor) {
//         let a = (u8::MAX - color.alpha) as u32;
//         // convert to 5 bits
//         let a = (a + 4) >> 3;

//         // 0000000g_gggg0000_0bbbbb00_000rrrrr
//         let expanded = (self.0 & (Self::R_MASK | Self::B_MASK)) as u32
//             | (((self.0 & Self::G_MASK) as u32) << 15);

//         // 00gggggg_gg00bbbb_bbbb00rr_rrrrrr00
//         let c =
//             ((color.blue as u32) << 12) | ((color.green as u32) << 22) | ((color.red as u32) << 2);
//         // 00ggggg0_0000bbbb_b00000rr_rrr00000
//         let c = c & 0b00111110_00001111_10000011_11100000;

//         let res = expanded * a + c;

//         self.0 = ((res >> 20) as u16 & Self::G_MASK)
//             | ((res >> 5) as u16 & (Self::R_MASK | Self::B_MASK));
//     }

//     fn from_rgb(r: u8, g: u8, b: u8) -> Self {
//         Self(((b as u16 & 0b11111000) << 7) | ((g as u16 & 0b11111000) << 2) | (r as u16 >> 3))
//     }
// }
// struct FrameBuffer<'a>{ frame_buffer: &'a mut [Bgr555Pixel] }
// impl<'a> slint::platform::software_renderer::LineBufferProvider for FrameBuffer<'a> {
//  type TargetPixel = Bgr555Pixel;
//  fn process_line(
//      &mut self,
//      line: usize,
//      range: core::ops::Range<usize>,
//      render_fn: impl FnOnce(&mut [Self::TargetPixel]),
//  ) { unsafe {
//      let line_begin = line * 240;
//      render_fn(&mut self.frame_buffer[line_begin..][range]);
//      // The line has been rendered and there could be code here to
//      // send the pixel to the display
//  } }
// }

pub fn init() {
    unsafe { ALLOCATOR.init(0x02000000, HEAP_SIZE) }

    DISPSTAT.write(DisplayStatus::new().with_irq_vblank(true));
    IE.write(IrqBits::VBLANK);
    IME.write(true);

    // 16.78 MHz / (16*1024) = 1024 overflows per second
    // This means that each overflow will increment the cascaded TIMER3 each ~1ms.
    TIMER2_RELOAD.write(0xffff - 16);
    TIMER2_CONTROL.write(TimerControl::new().with_enabled(true).with_scale(TimerScale::_1024));
    TIMER3_CONTROL.write(TimerControl::new().with_enabled(true).with_cascade(true));

    BG0CNT.write(BackgroundControl::new().with_screenblock(31));
    DISPCNT.write(DisplayControl::new().with_video_mode(VideoMode::_3).with_show_bg2(true));

    let window = MinimalGbaWindow::new();
    slint::platform::set_platform(Box::new(GbaPlatform { window: window.clone() }))
        .expect("backend already initialized");
}

pub fn set_sound_renderer(sound_renderer: Rc<RefCell<SoundRenderer>>) {
    unsafe { SOUND_RENDERER = Some(sound_renderer) }
}

// FIXME: Move as a platform method and attach here.
pub fn set_main_window(main_window: slint::Weak<MainWindow>) {
    unsafe { WINDOW = Some(main_window) }
}

impl slint::platform::Platform for GbaPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn slint::platform::WindowAdapter>, PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> core::time::Duration {
        let timer3_read = TIMER3_COUNT.read();
        // FIXME: Don't use static mut
        let ms = unsafe {
            if timer3_read < LAST_TIMER3_READ {
                BASE_MILLIS_SINCE_START += 0xffff;
            }
            LAST_TIMER3_READ = timer3_read;
            BASE_MILLIS_SINCE_START + LAST_TIMER3_READ as u32
        };

        core::time::Duration::new(0, ms.wrapping_mul(1_000_000))
    }

    fn debug_log(&self, arguments: core::fmt::Arguments) {
        if let Ok(mut logger) = MgbaBufferedLogger::try_new(MgbaMessageLevel::Info) {
            write!(logger, "{}", arguments).ok();
        }
    }

    fn run_event_loop(&self) -> Result<(), PlatformError> {
        // FIXME: Those take iwram space by being put on the stack and could probably be used for something better.
        let slint_key_a: SharedString = slint::platform::Key::Control.into();
        let slint_key_b: SharedString = slint::platform::Key::Shift.into();
        let slint_key_select: SharedString = ' '.into();
        let slint_key_start: SharedString = slint::platform::Key::Return.into();
        let slint_key_right: SharedString = slint::platform::Key::RightArrow.into();
        let slint_key_left: SharedString = slint::platform::Key::LeftArrow.into();
        let slint_key_up: SharedString = slint::platform::Key::UpArrow.into();
        let slint_key_down: SharedString = slint::platform::Key::DownArrow.into();
        let slint_key_r: SharedString = slint::platform::Key::PageDown.into();
        let slint_key_l: SharedString = slint::platform::Key::Tab.into();

        let window = self.window.clone();
        window.set_size(DISPLAY_SIZE);
        window.attach_trackers();

        // let frame_buffer = unsafe {
        //     core::slice::from_raw_parts_mut(0x0600_0000 as *mut Bgr555Pixel,
        //     (DISPLAY_SIZE.width * DISPLAY_SIZE.height) as usize)
        // };
        log!("--- Memory used before loop: {}kb", ALLOCATOR.used());

        let mut prev_keys = 0u16;
        let mut prev_used = 0;
        loop {
            VBlankIntrWait();
            let keys = KEYINPUT.read().to_u16();

            let cps = 16 * 1024 * 1024 / 1024;
            slint::platform::update_timers_and_animations();

            TIMER0_CONTROL.write(TimerControl::new().with_enabled(false));
            TIMER0_RELOAD.write(0);
            TIMER0_CONTROL.write(TimerControl::new().with_scale(TimerScale::_1024).with_enabled(true));
            window.draw_if_needed(|_renderer| {
                // renderer.render(frame_buffer, DISPLAY_SIZE.width as usize);
            });
            let time = TIMER0_COUNT.read() as u32 * 1000 / cps;
            if time > 0 {
                log!("--- window.draw_if_needed(ms) {}", time);
            }

            TIMER0_CONTROL.write(TimerControl::new().with_enabled(false));
            TIMER0_RELOAD.write(0);
            TIMER0_CONTROL.write(TimerControl::new().with_scale(TimerScale::_1024).with_enabled(true));
            unsafe {
                SOUND_RENDERER
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .sound_engine
                    .advance_frame();
            }
            let time = TIMER0_COUNT.read() as u32 * 1000 / cps;
            if time > 0 {
                log!("--- sound_engine.advance_frame(ms) {}", time);
            }

            if prev_used != ALLOCATOR.used() {
                log!("--- Memory used: {}kb", ALLOCATOR.used());
                prev_used = ALLOCATOR.used();
            }

            let switched_keys = keys ^ prev_keys;
            if switched_keys != 0 {
                log!("{:#b}, {:#b}, {:#b}", prev_keys, keys, switched_keys);
                let process_key = |key_mask: u16, out_key: &SharedString| {
                    if switched_keys & key_mask != 0 {
                        if keys & key_mask != 0 {
                            log!("PRESS {}", out_key.chars().next().unwrap() as u8);
                            window.dispatch_event(WindowEvent::KeyReleased { text: out_key.clone() });
                        } else {
                            log!("RELEASE {}", out_key.chars().next().unwrap() as u8);
                            window.dispatch_event(WindowEvent::KeyPressed { text: out_key.clone() });
                        }
                    }
                };

                process_key(KEY_A, &slint_key_a);
                process_key(KEY_B, &slint_key_b);
                process_key(KEY_SELECT, &slint_key_select);
                process_key(KEY_START, &slint_key_start);
                process_key(KEY_RIGHT, &slint_key_right);
                process_key(KEY_LEFT, &slint_key_left);
                process_key(KEY_UP, &slint_key_up);
                process_key(KEY_DOWN, &slint_key_down);
                process_key(KEY_R, &slint_key_r);
                process_key(KEY_L, &slint_key_l);
                prev_keys = keys;
            }
        }
    }
}
