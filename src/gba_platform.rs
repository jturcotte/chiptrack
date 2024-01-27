// Copyright © SixtyFPS GmbH <info@slint-ui.com>
// SPDX-License-Identifier: GPL-3.0-only OR LicenseRef-Slint-commercial

extern crate alloc;

use crate::elog;
use crate::log;
use crate::sound_renderer::SoundRenderer;
use crate::FocusedPanel;
use crate::GlobalEngine;
use crate::GlobalUI;
use crate::MainWindow;
use crate::MidiNote;

use alloc::boxed::Box;
use alloc::rc::Rc;
use core::cell::RefCell;
use core::fmt::Write;
use core::iter::repeat;
use core::pin::Pin;

use embedded_alloc::Heap;
use gba::{
    mgba::{MgbaBufferedLogger, MgbaMessageLevel},
    prelude::*,
};
use i_slint_core::model::ModelChangeListenerContainer;
use slint::platform::software_renderer::MinimalSoftwareWindow;
use slint::platform::WindowEvent;
use slint::Global;
use slint::Model;
use slint::PlatformError;
use slint::SharedString;
use voladdress::{Safe, VolBlock};

#[alloc_error_handler]
fn oom(layout: core::alloc::Layout) -> ! {
    panic!("Out of memory {:?}", layout);
}

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    if let Ok(mut logger) = MgbaBufferedLogger::try_new(MgbaMessageLevel::Fatal) {
        write!(logger, "{info}").ok();
    } else {
        elog!("{info}");
    }

    loop {}
}

const HEAP_SIZE: usize = 256 * 1024;

#[global_allocator]
static ALLOCATOR: Heap = Heap::empty();

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
const KEYS_ALL: u16 = 0b11_11111111;
const KEYS_REPEATABLE: u16 = 0b00_11110000;

// Default palette
const NORMAL_TEXT: u16 = 0;
// Those types can be ORed into mixed palettes
const FADED_TEXT: u16 = 0b001;
const ALT_COL_TEXT: u16 = 0b10;
// Selected eats any mixed-in type
const SELECTED_TEXT: u16 = 0b111;
// Must not be mixed
const ERROR_TEXT: u16 = 0b101;

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

pub struct MainScreen {
    sequencer_song_patterns_tracker: Pin<Box<i_slint_core::model::ModelChangeListenerContainer<ModelDirtinessTracker>>>,
    sequencer_steps_tracker: Pin<Box<i_slint_core::model::ModelChangeListenerContainer<ModelDirtinessTracker>>>,
    sequencer_pattern_instruments_tracker:
        Pin<Box<i_slint_core::model::ModelChangeListenerContainer<ModelDirtinessTracker>>>,
    instruments_tracker: Pin<Box<i_slint_core::model::ModelChangeListenerContainer<ModelDirtinessTracker>>>,
    focused_panel_previous: RefCell<FocusedPanel>,
    selected_column_previous: RefCell<i32>,
    sequencer_pattern_instruments_len_previous: RefCell<usize>,
    sequencer_song_pattern_active_previous: RefCell<usize>,
    sequencer_step_active_previous: RefCell<usize>,
    selected_instrument_previous: RefCell<usize>,
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
    fn row_changed(self: Pin<&Self>, _row: usize) {
        *self.is_dirty.borrow_mut() = true;
    }

    fn row_added(self: Pin<&Self>, _index: usize, _count: usize) {
        *self.is_dirty.borrow_mut() = true;
    }

    fn row_removed(self: Pin<&Self>, _index: usize, _count: usize) {
        *self.is_dirty.borrow_mut() = true;
    }
    fn reset(self: Pin<&Self>) {
        *self.is_dirty.borrow_mut() = true;
    }
}

fn to_hex(v: u8) -> [u8; 2] {
    let l = v & 0xf;
    let h = v >> 4;
    let c1 = if h < 0xa { b'0' + h } else { b'A' + h - 0xa };
    let c2 = if l < 0xa { b'0' + l } else { b'A' + l - 0xa };
    [c1, c2]
}

fn to_dec(v: u8) -> [u8; 2] {
    debug_assert!(v < 100);
    let c1 = (v / 10) as u8 + '0' as u8;
    let c2 = (v % 10) as u8 + '0' as u8;
    [c1, c2]
}

fn draw_ascii_byte<const C: usize>(vid_row: VolBlock<TextEntry, Safe, Safe, C>, index: usize, char: u8, palbank: u16) {
    vid_row
        .index(index)
        .write(TextEntry::new().with_tile(char as u16).with_palbank(palbank));
}

fn draw_ascii<RB, U, const C: usize>(vid_row: VolBlock<TextEntry, Safe, Safe, C>, range: RB, chars: U, palbank: u16)
where
    RB: core::ops::RangeBounds<usize>,
    U: IntoIterator<Item = u8>,
{
    vid_row
        .iter_range(range)
        .zip(chars)
        .for_each(|(row, c)| row.write(TextEntry::new().with_tile(c as u16).with_palbank(palbank)));
}

fn draw_ascii_ref<RB, const C: usize>(
    vid_row: VolBlock<TextEntry, Safe, Safe, C>,
    range: RB,
    chars: &[u8],
    palbank: u16,
) where
    RB: core::ops::RangeBounds<usize>,
{
    draw_ascii(vid_row, range, chars.iter().map(|c| *c), palbank)
}

fn draw_ascii_chars<RB, U, const C: usize>(
    vid_row: VolBlock<TextEntry, Safe, Safe, C>,
    range: RB,
    chars: U,
    palbank: u16,
) where
    RB: core::ops::RangeBounds<usize>,
    U: Iterator<Item = char>,
{
    draw_ascii(vid_row, range, chars.map(|c| c as u8), palbank)
}

pub fn draw_error_text(t: &str) {
    let tsb = TEXT_SCREENBLOCKS.get_frame(31).unwrap();
    draw_ascii_chars(tsb.get_row(19).unwrap(), 0.., t.chars(), ERROR_TEXT);
}

impl MainScreen {
    pub fn new() -> Self {
        // Copy text data into the first tile indices, the tile index is then the ASCII code.
        // Set the offset to 1 (including transparent pixels) since I want the
        // background color to be set by the palette as well.
        Cga8x8Thick.bitunpack_4bpp(CHARBLOCK0_4BPP.as_region(), 0x80000001);

        BG0CNT.write(BackgroundControl::new().with_screenblock(31));
        BACKDROP_COLOR.write(Color::WHITE);
        DISPCNT.write(DisplayControl::new().with_show_bg0(true));

        fn set_palette(bank: u16, colors: [Color; 2]) {
            bg_palbank(bank as usize)
                .iter()
                .skip(1)
                .zip(colors)
                .for_each(|(i, c)| i.write(c));
        }
        set_palette(NORMAL_TEXT, [Color::WHITE, Color::BLACK]);
        set_palette(ALT_COL_TEXT, [Color(0b0_11100_11100_11100), Color::BLACK]);
        set_palette(FADED_TEXT, [Color::WHITE, Color(0b0_11010_11010_11010)]);
        set_palette(
            SELECTED_TEXT,
            [Color(0b0_11000_11000_11000), Color(0b0_11110_10010_00100)],
        );
        set_palette(
            FADED_TEXT | ALT_COL_TEXT,
            [Color(0b0_11100_11100_11100), Color(0b0_11010_11010_11010)],
        );
        set_palette(ERROR_TEXT, [Color::WHITE, Color::RED]);

        Self {
            sequencer_song_patterns_tracker: Box::pin(ModelChangeListenerContainer::<ModelDirtinessTracker>::default()),
            sequencer_steps_tracker: Box::pin(ModelChangeListenerContainer::<ModelDirtinessTracker>::default()),
            sequencer_pattern_instruments_tracker: Box::pin(
                ModelChangeListenerContainer::<ModelDirtinessTracker>::default(),
            ),
            instruments_tracker: Box::pin(ModelChangeListenerContainer::<ModelDirtinessTracker>::default()),
            focused_panel_previous: RefCell::new(FocusedPanel::Steps),
            selected_column_previous: RefCell::new(0),
            sequencer_pattern_instruments_len_previous: RefCell::new(0),
            sequencer_song_pattern_active_previous: RefCell::new(0),
            sequencer_step_active_previous: RefCell::new(0),
            selected_instrument_previous: RefCell::new(0),
        }
    }

    fn attach_trackers(&self) {
        let handle = unsafe { WINDOW.as_ref().unwrap().upgrade().unwrap() };
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
    }

    pub fn draw(&self) {
        let handle = unsafe { WINDOW.as_ref().unwrap().upgrade().unwrap() };
        let global_engine = GlobalEngine::get(&handle);
        let global_ui = GlobalUI::get(&handle);
        let focused_panel = handle.get_focused_panel();
        let focused_panel_previous = self.focused_panel_previous.replace(focused_panel);
        let focused_panel_dirty = focused_panel_previous != focused_panel;
        let selected_column = global_ui.get_selected_column();
        let selected_column_dirty = self.selected_column_previous.replace(selected_column) != selected_column;
        let sequencer_pattern_instruments_len = global_engine.get_sequencer_pattern_instruments_len() as usize;
        let sequencer_pattern_instruments_len_dirty = self
            .sequencer_pattern_instruments_len_previous
            .replace(sequencer_pattern_instruments_len)
            != sequencer_pattern_instruments_len;
        let sequencer_song_pattern_active = global_engine.get_sequencer_song_pattern_active() as usize;
        let sequencer_song_pattern_active_dirty = self
            .sequencer_song_pattern_active_previous
            .replace(sequencer_song_pattern_active)
            != sequencer_song_pattern_active;
        let sequencer_step_active = global_engine.get_sequencer_step_active() as usize;
        let sequencer_step_active_dirty =
            self.sequencer_step_active_previous.replace(sequencer_step_active) != sequencer_step_active;
        let selected_instrument = global_engine.get_selected_instrument() as usize;
        let selected_instrument_dirty =
            self.selected_instrument_previous.replace(selected_instrument) != selected_instrument;

        let sequencer_song_patterns_dirty = self.sequencer_song_patterns_tracker.take_dirtiness();
        let sequencer_steps_dirty = self.sequencer_steps_tracker.take_dirtiness();
        let sequencer_pattern_instruments_dirty = self.sequencer_pattern_instruments_tracker.take_dirtiness();
        let instruments_dirty = self.instruments_tracker.take_dirtiness();

        let tsb = TEXT_SCREENBLOCKS.get_frame(31).unwrap();
        const PARAMS_START_X: usize = 4;
        const STEPS_START_X: usize = 10;
        const INSTR_START_X: usize = 14;

        let status_vid_row = tsb.get_row(18).unwrap();
        if focused_panel_dirty {
            draw_ascii_byte(
                status_vid_row,
                0,
                (focused_panel == FocusedPanel::Patterns) as u8 * Cga8x8Thick::BULLET,
                NORMAL_TEXT,
            );
            draw_ascii_byte(
                status_vid_row,
                PARAMS_START_X - 1,
                (focused_panel == FocusedPanel::Steps) as u8 * Cga8x8Thick::BULLET,
                NORMAL_TEXT,
            );
            draw_ascii_byte(
                status_vid_row,
                INSTR_START_X - 1,
                (focused_panel == FocusedPanel::Instruments) as u8 * Cga8x8Thick::BULLET,
                NORMAL_TEXT,
            );
        }

        if sequencer_song_pattern_active_dirty || sequencer_song_patterns_dirty {
            let pattern_model = global_engine.get_sequencer_song_patterns();
            draw_ascii_ref(tsb.get_row(0).unwrap(), 0.., b"Song", NORMAL_TEXT);

            // sequencer_song_patterns_tracker will be dirty and trigger a redraw any time this is changed.
            let sequencer_song_pattern_selected = global_engine.get_sequencer_song_pattern_selected() as usize;
            let scroll_pos = sequencer_song_pattern_selected
                .max(8)
                .min(pattern_model.row_count() - 8)
                - 8;
            for i in scroll_pos..(pattern_model.row_count().min(scroll_pos + 16)) {
                let vid_row = tsb.get_row(i - scroll_pos + 1).unwrap();
                let row_data = pattern_model.row_data(i).unwrap();
                let palbank = if row_data.selected { SELECTED_TEXT } else { NORMAL_TEXT };
                if row_data.number > -1 {
                    draw_ascii(vid_row, 1.., to_dec(row_data.number as u8 + 1), palbank);
                } else {
                    draw_ascii(vid_row, 1..3, repeat(b'-'), palbank | FADED_TEXT);
                }
                draw_ascii_byte(
                    vid_row,
                    0,
                    (i == sequencer_song_pattern_active) as u8 * Cga8x8Thick::BULLET,
                    NORMAL_TEXT,
                )
            }
        }

        if sequencer_steps_dirty || sequencer_step_active_dirty || selected_column_dirty || focused_panel_dirty {
            let selected_instrument = global_engine.get_selected_instrument() as usize;
            let vid_row = tsb.get_row(0).unwrap();
            let selected_instrument_id = global_engine
                .get_instruments()
                .row_data(selected_instrument)
                .unwrap()
                .id;
            draw_ascii_chars(
                vid_row,
                STEPS_START_X..STEPS_START_X + 3,
                selected_instrument_id.chars().chain(repeat(' ')),
                NORMAL_TEXT,
            );

            let sequencer_steps = global_engine.get_sequencer_steps();
            for i in 0..sequencer_steps.row_count() {
                let vid_row = tsb.get_row(i + 1).unwrap();
                let row_data = sequencer_steps.row_data(i).unwrap();
                let selected = row_data.selected && focused_panel == FocusedPanel::Steps;
                let param0_bank = if selected && selected_column == 0 {
                    SELECTED_TEXT
                } else {
                    NORMAL_TEXT
                };
                let param1_bank = if selected && selected_column == 1 {
                    SELECTED_TEXT
                } else {
                    NORMAL_TEXT
                };
                let press_bank = if selected && selected_column == 2 {
                    SELECTED_TEXT
                } else {
                    NORMAL_TEXT
                };
                let release_bank = if selected && selected_column >= 2 {
                    SELECTED_TEXT
                } else {
                    NORMAL_TEXT
                };

                // Draw params
                if row_data.param0_set {
                    draw_ascii(
                        vid_row,
                        PARAMS_START_X..,
                        to_hex(row_data.param0_val as u8),
                        param0_bank,
                    );
                } else {
                    draw_ascii(
                        vid_row,
                        PARAMS_START_X..PARAMS_START_X + 2,
                        repeat(b'-'),
                        param0_bank | FADED_TEXT,
                    );
                }
                draw_ascii_byte(vid_row, PARAMS_START_X + 2, b'/', FADED_TEXT);
                if row_data.param1_set {
                    draw_ascii(
                        vid_row,
                        PARAMS_START_X + 3..,
                        to_hex(row_data.param1_val as u8),
                        param1_bank,
                    );
                } else {
                    draw_ascii(
                        vid_row,
                        PARAMS_START_X + 3..PARAMS_START_X + 5,
                        repeat(b'-'),
                        param1_bank | FADED_TEXT,
                    );
                }

                if row_data.press {
                    draw_ascii(
                        vid_row,
                        STEPS_START_X..,
                        MidiNote(row_data.note).char_desc(),
                        press_bank,
                    );
                } else {
                    draw_ascii(
                        vid_row,
                        STEPS_START_X..STEPS_START_X + 3,
                        repeat(b'-'),
                        press_bank | FADED_TEXT,
                    );
                }

                draw_ascii_byte(
                    vid_row,
                    PARAMS_START_X - 1,
                    (i == sequencer_step_active) as u8 * Cga8x8Thick::BULLET,
                    NORMAL_TEXT,
                );
                draw_ascii_byte(vid_row, STEPS_START_X - 1, row_data.press as u8 * b'[', press_bank);
                draw_ascii_byte(vid_row, STEPS_START_X + 3, row_data.release as u8 * b']', release_bank);
            }
        }

        let toggled_instruments = focused_panel_dirty
            && (focused_panel == FocusedPanel::Instruments || focused_panel_previous == FocusedPanel::Instruments);
        if toggled_instruments {
            for i in 0..17 {
                draw_ascii(tsb.get_row(i).unwrap(), INSTR_START_X.., repeat(0), NORMAL_TEXT);
            }
            if focused_panel == FocusedPanel::Instruments {
                draw_ascii_ref(tsb.get_row(0).unwrap(), INSTR_START_X.., b"Instruments", NORMAL_TEXT);
            }
        }
        if focused_panel != FocusedPanel::Instruments
            && (toggled_instruments || sequencer_pattern_instruments_len_dirty || sequencer_pattern_instruments_dirty)
        {
            let top_vid_row = tsb.get_row(0).unwrap();
            let sequencer_pattern_instruments = global_engine.get_sequencer_pattern_instruments();
            for i in 0..5 {
                let x = i * 3 + INSTR_START_X;
                let col_bank = if i % 2 == 0 { ALT_COL_TEXT } else { NORMAL_TEXT };
                let normal_bank = col_bank | NORMAL_TEXT;
                let faded_bank = col_bank | FADED_TEXT;

                if i < sequencer_pattern_instruments_len {
                    let row_data = sequencer_pattern_instruments.row_data(i).unwrap();

                    draw_ascii_chars(
                        top_vid_row,
                        x..x + 3,
                        row_data.id.chars().chain(repeat(' ')),
                        normal_bank,
                    );

                    let notes = row_data.notes;
                    for j in 0..notes.row_count() {
                        let note = notes.row_data(j).unwrap();
                        let vid_row = tsb.get_row(j + 1).unwrap();

                        if note != -1 {
                            let midi_note = MidiNote(note);
                            if midi_note.is_black() {
                                draw_ascii(vid_row, x.., midi_note.char_desc(), normal_bank);
                            } else {
                                draw_ascii(vid_row, x.., midi_note.short_char_desc(), normal_bank);
                                draw_ascii_byte(vid_row, x + 2, b' ', faded_bank);
                            }
                        } else {
                            draw_ascii_ref(vid_row, x.., b"-- ", faded_bank);
                        }
                    }
                } else {
                    for j in 0..17 {
                        draw_ascii_ref(tsb.get_row(j).unwrap(), x.., b"-- ", faded_bank);
                    }
                }
            }
        }
        if focused_panel == FocusedPanel::Instruments
            && (toggled_instruments || selected_instrument_dirty || instruments_dirty)
        {
            let instruments = global_engine.get_instruments();
            let scroll_pos =
                // 4 instruments per row
                (selected_instrument / 4)
                // Keep the cursor on the middle row even if the first row is selected
                    .max(4)
                // With the cursor in the middle of the screen, keep the last row at the bottom of the screen (4 rows more)
                    .min(instruments.row_count() / 4 - 4)
                // Work with the pos of the top of the screen.
                    - 4;
            for row in scroll_pos..scroll_pos + 8 {
                let vid_row = tsb.get_row((row - scroll_pos) * 2 + 2).unwrap();
                for col in 0..4 {
                    let x = col * 4 + INSTR_START_X;
                    let instrument_idx = row * 4 + col;
                    let instrument = instruments.row_data(instrument_idx).unwrap();
                    let palbank = if instrument_idx == selected_instrument {
                        SELECTED_TEXT
                    } else {
                        NORMAL_TEXT
                    };
                    // Active indicator
                    draw_ascii_byte(vid_row, x, instrument.active as u8 * Cga8x8Thick::BULLET, palbank);
                    // Instrument ID
                    draw_ascii_chars(vid_row, x + 1..x + 5, instrument.id.chars().chain(repeat(' ')), palbank);
                }
            }
        }
    }
}

struct GbaPlatform {
    main_screen: MainScreen,
    window: Rc<MinimalSoftwareWindow>,
}

static mut LAST_TIMER3_READ: u16 = 0;
static mut BASE_MILLIS_SINCE_START: u32 = 0;
// FIXME: Use GbaCell just to avoid unsafe?
static mut SOUND_RENDERER: Option<Rc<RefCell<SoundRenderer>>> = None;
static mut WINDOW: Option<slint::Weak<MainWindow>> = None;

static TRIGGERED_IRQS: GbaCell<u16> = GbaCell::new(0);

#[link_section = ".iwram"]
extern "C" fn irq_handler(b: IrqBits) {
    // IntrWait won't tell us which interrupts made it return from sleep,
    // so gather this information in the interrupt handler where this is told to us.
    TRIGGERED_IRQS.write(TRIGGERED_IRQS.read() | b.to_u16());
}

pub fn init() {
    unsafe { ALLOCATOR.init(0x02000000, HEAP_SIZE) }

    RUST_IRQ_HANDLER.write(Some(irq_handler));
    DISPSTAT.write(DisplayStatus::new().with_irq_vblank(true));
    KEYCNT.write(
        KeyControl::new()
            .with_a(true)
            .with_b(true)
            .with_select(true)
            .with_start(true)
            .with_right(true)
            .with_left(true)
            .with_up(true)
            .with_down(true)
            .with_r(true)
            .with_l(true)
            .with_irq_enabled(true),
    );
    IE.write(IrqBits::VBLANK.with_keypad(true));
    IME.write(true);

    // 16.78 MHz / (16*1024) = 1024 overflows per second
    // This means that each overflow will increment the cascaded TIMER3 each ~1ms.
    TIMER2_RELOAD.write(0xffff - 16);
    TIMER2_CONTROL.write(TimerControl::new().with_enabled(true).with_scale(TimerScale::_1024));
    TIMER3_CONTROL.write(TimerControl::new().with_enabled(true).with_cascade(true));

    let main_screen = MainScreen::new();
    let window = MinimalSoftwareWindow::new(Default::default());
    slint::platform::set_platform(Box::new(GbaPlatform { main_screen, window })).expect("backend already initialized");
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
        let slint_key_r: SharedString = slint::platform::Key::Alt.into();
        let slint_key_l: SharedString = slint::platform::Key::Tab.into();

        let main_screen = &self.main_screen;
        let window = self.window.clone();
        main_screen.attach_trackers();

        log!("--- Memory used before loop: {}kb", ALLOCATOR.used());

        let mut prev_keys = 0u16;
        let mut repeating_key_mask = 0u16;
        let mut prev_used = 0;
        let mut frames_until_repeat: Option<u16> = None;
        // IntrWait seems to never halt if ignore_existing_interrupts is false (which I need to process a possible
        // pending vblank after processing keys), and this somehow causes multiple missed frames.
        // So just assume that we're in mgba if mgba_logging_available() is true and skip just one frame
        // if keys are being handled while vblank is requested.
        let ignore_existing_interrupts = mgba_logging_available();

        loop {
            IntrWait(
                ignore_existing_interrupts,
                IrqBits::new().with_vblank(true).with_keypad(true),
            );
            let process_vblank = TRIGGERED_IRQS.read() & IrqBits::VBLANK.to_u16() != 0;
            // Processing the keys only after redrawing and then waiting for the next vblank would
            // mean that all key handlers would be delayed by one frame, including their effect on
            // the audio through instruments.
            // So also unblock the loop to process press and releases on keypad interupts so that we
            // can process key changes after we checked and went back to sleep.
            // Also check if KEYINPUT changed on vblank since mgba only seems to register key
            // releases during that time.
            let process_keys = process_vblank || TRIGGERED_IRQS.read() & IrqBits::KEYPAD.to_u16() != 0;
            TRIGGERED_IRQS.write(0);

            let cps = 16 * 1024 * 1024 / 1024;
            // Run main_screen.draw() before key handling to avoid missing the vblank window due to the heaving
            // processing happening in key handlers.
            if process_vblank {
                slint::platform::update_timers_and_animations();

                TIMER0_CONTROL.write(TimerControl::new().with_enabled(false));
                TIMER0_RELOAD.write(0);
                TIMER0_CONTROL.write(TimerControl::new().with_scale(TimerScale::_1024).with_enabled(true));
                main_screen.draw();
                let time = TIMER0_COUNT.read() as u32 * 1000 / cps;
                if time > 0 {
                    log!("--- main_screen.draw(ms) {}", time);
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

                if let Some(frames) = frames_until_repeat.as_mut() {
                    *frames -= 1
                }
            }

            if process_keys {
                TIMER0_CONTROL.write(TimerControl::new().with_enabled(false));
                TIMER0_RELOAD.write(0);
                TIMER0_CONTROL.write(TimerControl::new().with_scale(TimerScale::_1024).with_enabled(true));

                let released_keys = KEYINPUT.read().to_u16();
                let switched_keys = released_keys ^ prev_keys;
                if switched_keys != 0 || frames_until_repeat == Some(0) {
                    log!("{:#b}, {:#b}, {:#b}", prev_keys, released_keys, switched_keys);
                    let mut process_key = |key_mask: u16, out_key: &SharedString| {
                        if switched_keys & key_mask != 0 {
                            if released_keys & key_mask == 0 {
                                log!("PRESS {}", out_key.chars().next().unwrap() as u8);
                                window.dispatch_event(WindowEvent::KeyPressed { text: out_key.clone() });
                                if key_mask & KEYS_REPEATABLE != 0 {
                                    repeating_key_mask = key_mask;
                                    frames_until_repeat = Some(10);
                                }
                            } else {
                                log!("RELEASE {}", out_key.chars().next().unwrap() as u8);
                                window.dispatch_event(WindowEvent::KeyReleased { text: out_key.clone() });
                            }
                        }

                        if frames_until_repeat == Some(0)
                            && released_keys & key_mask == 0
                            && repeating_key_mask == key_mask
                        {
                            log!("REPEAT {}", out_key.chars().next().unwrap() as u8);
                            window.dispatch_event(WindowEvent::KeyPressed { text: out_key.clone() });
                            frames_until_repeat = Some(2);
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
                    prev_keys = released_keys;

                    if released_keys == KEYS_ALL {
                        frames_until_repeat = None;
                    }
                }

                let time = TIMER0_COUNT.read() as u32 * 1000 / cps;
                if time > 0 {
                    log!("--- process_key(ms) {}", time);
                }
            }
        }
    }
}
