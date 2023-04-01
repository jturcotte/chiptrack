// Copyright © SixtyFPS GmbH <info@slint-ui.com>
// SPDX-License-Identifier: GPL-3.0-only OR LicenseRef-Slint-commercial

extern crate alloc;

use i_slint_core::model::ModelChangeListenerContainer;
use slint::Model;
use slint::Global;
use crate::GlobalEngine;
use crate::MainWindow;
use crate::sound_renderer::SoundRenderer;
use slint::Brush::SolidColor;
use i_slint_core::renderer::Renderer;
use core::pin::Pin;
use core::cell::Cell;
use slint::Window;
use slint::platform::software_renderer::SoftwareRenderer;
use alloc::rc::Weak;
use alloc::boxed::Box;
use alloc::rc::Rc;

use embedded_alloc::Heap;
use core::cell::RefCell;

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

// This is a type alias for the enabled `restore-state-*` feature.
// For example, it is `bool` if you enable `restore-state-bool`.
use critical_section::RawRestoreState;

struct GbaCriticalSection;
critical_section::set_impl!(GbaCriticalSection);

unsafe impl critical_section::Impl for GbaCriticalSection {
    unsafe fn acquire() -> RawRestoreState { true }
    unsafe fn release(_token: RawRestoreState) { }
}

pub struct MinimalGbaWindow {
    window: i_slint_core::api::Window,
    renderer: SoftwareRenderer<0>,
    needs_redraw: Cell<bool>,
    sequencer_steps_tracker: Pin<Box<i_slint_core::model::ModelChangeListenerContainer<ModelDirtinessTracker>>>,
    sequencer_pattern_instruments_tracker: Pin<Box<i_slint_core::model::ModelChangeListenerContainer<ModelDirtinessTracker>>>,
}

struct ModelDirtinessTracker {
    is_dirty: RefCell<bool>,
}

impl Default for ModelDirtinessTracker {
    fn default() -> Self {
        ModelDirtinessTracker { is_dirty: RefCell::new(true) }
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
        DISPCNT.write(DisplayControl::new()
            // .with_video_mode(VideoMode::_0)
            .with_show_bg0(true));

        Rc::new_cyclic(|w: &Weak<Self>| Self {
            window: Window::new(w.clone()),
            renderer: SoftwareRenderer::new(w.clone()),
            needs_redraw: Default::default(),
            sequencer_steps_tracker: Box::pin(ModelChangeListenerContainer::<ModelDirtinessTracker>::default()),
            sequencer_pattern_instruments_tracker: Box::pin(ModelChangeListenerContainer::<ModelDirtinessTracker>::default()),
        })
    }

    fn attach_trackers(&self) {
        let handle = unsafe { WINDOW.as_ref().unwrap().upgrade().unwrap() };
        GlobalEngine::get(&handle).get_sequencer_steps().model_tracker().attach_peer(Pin::as_ref(&self.sequencer_steps_tracker).model_peer());
        GlobalEngine::get(&handle).get_sequencer_pattern_instruments().model_tracker().attach_peer(Pin::as_ref(&self.sequencer_pattern_instruments_tracker).model_peer());
    }

    pub fn draw_if_needed(
        &self,
        render_callback: impl FnOnce(&SoftwareRenderer<0>),
    ) -> bool {
        // FIXME: Check if this could be casted from the component of self somehow
        let handle = unsafe { WINDOW.as_ref().unwrap().upgrade().unwrap() };
        let global_engine = GlobalEngine::get(&handle);
        let current_instrument = global_engine.get_current_instrument() as usize;
        let tsb = TextScreenblockAddress::new(31);

        if self.sequencer_steps_tracker.take_dirtiness() {
            let current_instrument_id = global_engine.get_instruments().row_data(current_instrument).unwrap().id;
            tsb.row_col(0, 6 + 1).write(TextEntry::new());
            tsb.row_col(0, 6 + 2).write(TextEntry::new());
            for (j, c) in current_instrument_id.chars().enumerate() {
                tsb.row_col(0, 6 + j).write(TextEntry::new().with_tile(c as u16));
            }

            let sequencer_steps = global_engine.get_sequencer_steps();
            for i in 0 .. sequencer_steps.row_count() {
                let row_data = sequencer_steps.row_data(i).unwrap();
                for (j, c) in row_data.note_name.chars().enumerate() {
                    let tile_index = if row_data.press {
                        c as u16
                    } else {
                        0
                    };
                    tsb.row_col(i + 1, j + 6).write(TextEntry::new().with_tile(tile_index));
                }
                tsb.row_col(i + 1, 4).write(TextEntry::new().with_tile(row_data.active as u16 * 7));
                tsb.row_col(i + 1, 5).write(TextEntry::new().with_tile(row_data.press as u16 * '[' as u16));
                tsb.row_col(i + 1, 9).write(TextEntry::new().with_tile(row_data.release as u16 * ']' as u16));
            }
        }
        if self.sequencer_pattern_instruments_tracker.take_dirtiness() {
            let sequencer_pattern_instruments = global_engine.get_sequencer_pattern_instruments();
            for i in 0 .. 6 {
                if i < sequencer_pattern_instruments.row_count() {
                    let row_data = sequencer_pattern_instruments.row_data(i).unwrap();

                    tsb.row_col(0, i * 3 + 11 + 1).write(TextEntry::new());
                    tsb.row_col(0, i * 3 + 11 + 2).write(TextEntry::new());
                    for (j, c) in row_data.id.chars().enumerate() {
                        tsb.row_col(0, i * 3 + 11 + j).write(TextEntry::new().with_tile(c as u16));
                    }

                    let steps_empty = row_data.steps_empty;
                    for j in 0 .. steps_empty.row_count() {
                        let empty = steps_empty.row_data(j).unwrap();
                        tsb.row_col(j + 1, i * 3 + 11).write(TextEntry::new().with_tile(!empty as u16 * 7));
                    }                    
                } else {
                    tsb.row_col(0, i * 3 + 11).write(TextEntry::new());
                    tsb.row_col(0, i * 3 + 11 + 1).write(TextEntry::new());
                    tsb.row_col(0, i * 3 + 11 + 2).write(TextEntry::new());
                    for j in 1 .. 17 {
                        tsb.row_col(j, i * 3 + 11).write(TextEntry::new());
                    }                    
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

impl i_slint_core::window::WindowAdapterSealed for MinimalGbaWindow
{
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
    DISPCNT.write(DisplayControl::new()
        .with_video_mode(VideoMode::_3)
        .with_show_bg2(true));

    let window = MinimalGbaWindow::new();
    slint::platform::set_platform(Box::new(GbaPlatform{window: window.clone()}))
        .expect("backend already initialized");

}

pub fn set_sound_renderer(sound_renderer: Rc<RefCell<SoundRenderer>>) {
    unsafe {
        SOUND_RENDERER = Some(sound_renderer)
    }
}

// FIXME: Move as a platform method and attach here.
pub fn set_main_window(main_window: slint::Weak<MainWindow>) {
    unsafe {
        WINDOW = Some(main_window)
    }
}

impl slint::platform::Platform for GbaPlatform {
    fn create_window_adapter(&self) -> Rc<dyn slint::platform::WindowAdapter> {
        self.window.clone()
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

    fn run_event_loop(&self) -> () {
        // FIXME: Those take iwram space by being put on the stack and could probably be used for something better.
        let slint_key_a = ' ';
        let slint_key_b = slint::platform::Key::Shift.into();
        let slint_key_select = slint::platform::Key::Escape.into();
        let slint_key_start = slint::platform::Key::Return.into();
        let slint_key_right = slint::platform::Key::RightArrow.into();
        let slint_key_left = slint::platform::Key::LeftArrow.into();
        let slint_key_up = slint::platform::Key::UpArrow.into();
        let slint_key_down = slint::platform::Key::DownArrow.into();
        let slint_key_r = slint::platform::Key::PageDown.into();
        let slint_key_l = slint::platform::Key::PageUp.into();

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

            let cps = 16*1024*1024 / 1024;
            TIMER0_CONTROL.write(TimerControl::new().with_enabled(false));
            TIMER0_RELOAD.write(0);
            TIMER0_CONTROL.write(TimerControl::new().with_scale(TimerScale::_1024).with_enabled(true));
            unsafe {
                SOUND_RENDERER.as_ref().unwrap().borrow_mut().sound_engine.advance_frame();
            }
            let time = TIMER0_COUNT.read() as u32 * 1000 / cps;
            if time > 0 {
                log!("--- sound_engine.advance_frame(ms) {}", time);
            }

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
            if prev_used != ALLOCATOR.used() {
                log!("--- Memory used: {}kb", ALLOCATOR.used());
                prev_used = ALLOCATOR.used();
            }

            let switched_keys = keys ^ prev_keys;
            if switched_keys != 0 {
                log!("{:#b}, {:#b}, {:#b}", prev_keys, keys, switched_keys);
                let process_key = |key_mask: u16, out_key: char| {
                    if switched_keys & key_mask != 0 {
                        if keys & key_mask != 0 {
                            window.dispatch_event(WindowEvent::KeyReleased { text: out_key });
                        } else {
                            window.dispatch_event(WindowEvent::KeyPressed { text: out_key });
                        }
                    }
                };

                process_key(KEY_A, slint_key_a);
                process_key(KEY_B, slint_key_b);
                process_key(KEY_SELECT, slint_key_select);
                process_key(KEY_START, slint_key_start);
                process_key(KEY_RIGHT, slint_key_right);
                process_key(KEY_LEFT, slint_key_left);
                process_key(KEY_UP, slint_key_up);
                process_key(KEY_DOWN, slint_key_down);
                process_key(KEY_R, slint_key_r);
                process_key(KEY_L, slint_key_l);
                prev_keys = keys;
            }
        }
    }
}
