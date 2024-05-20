// Copyright © SixtyFPS GmbH <info@slint-ui.com>
// SPDX-License-Identifier: GPL-3.0-only OR LicenseRef-Slint-commercial

extern crate alloc;

use crate::gba_platform::WINDOW;
use crate::utils::MidiNote;
use crate::FocusedPanel;
use crate::GlobalEngine;
use crate::GlobalUI;

use alloc::boxed::Box;
use core::cell::RefCell;
use core::iter::repeat;
use core::mem::replace;
use core::pin::Pin;

use gba::prelude::*;
use i_slint_core::model::ModelChangeListenerContainer;
use slint::Global;
use slint::Model;
use voladdress::{Safe, VolBlock};

// Default palette
const NORMAL_TEXT: u16 = 0;
// Those types can be ORed into mixed palettes
const FADED_TEXT: u16 = 0b001;
const ALT_COL_TEXT: u16 = 0b10;
// Selected eats any mixed-in type
const SELECTED_TEXT: u16 = 0b111;
// Must not be mixed
const ERROR_TEXT: u16 = 0b101;

// Cheap version of Slint's PropertyTracker that just compares values.
struct ChangeChecker<T: PartialEq + Copy> {
    last_seen: T,
}
struct ChangeCheckStatus<T: PartialEq + Copy> {
    current: T,
    previous: T,
}
impl<T: PartialEq + Copy> ChangeChecker<T> {
    fn new(initial: T) -> Self {
        Self { last_seen: initial }
    }

    fn check(&mut self, current: T) -> ChangeCheckStatus<T> {
        let previous = replace(&mut self.last_seen, current);
        ChangeCheckStatus { current, previous }
    }
}
impl<T: PartialEq + Copy> ChangeCheckStatus<T> {
    fn dirty(&self) -> bool {
        self.current != self.previous
    }
}

pub struct MainScreen {
    sequencer_song_patterns_tracker: Pin<Box<i_slint_core::model::ModelChangeListenerContainer<ModelDirtinessTracker>>>,
    sequencer_steps_tracker: Pin<Box<i_slint_core::model::ModelChangeListenerContainer<ModelDirtinessTracker>>>,
    sequencer_pattern_instruments_tracker:
        Pin<Box<i_slint_core::model::ModelChangeListenerContainer<ModelDirtinessTracker>>>,
    instruments_tracker: Pin<Box<i_slint_core::model::ModelChangeListenerContainer<ModelDirtinessTracker>>>,
    focused_panel_checker: ChangeChecker<FocusedPanel>,
    selected_column_checker: ChangeChecker<i32>,
    selected_step_checker: ChangeChecker<i32>,
    selected_step_range_first_checker: ChangeChecker<i32>,
    sequencer_pattern_instruments_len_checker: ChangeChecker<usize>,
    sequencer_song_pattern_active_checker: ChangeChecker<usize>,
    sequencer_step_active_checker: ChangeChecker<usize>,
    displayed_instrument_checker: ChangeChecker<usize>,
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
    let c1 = (v / 10) + b'0';
    let c2 = (v % 10) + b'0';
    [c1, c2]
}

/// Draw a single u8 `char` at the `index` position in the given `vid_row`.
fn draw_ascii_byte<const C: usize>(vid_row: VolBlock<TextEntry, Safe, Safe, C>, index: usize, char: u8, palbank: u16) {
    vid_row
        .index(index)
        .write(TextEntry::new().with_tile(char as u16).with_palbank(palbank));
}

/// Draw an iterator of u8 `chars` within `range` pos at the given `vid_row`.
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

/// Same as `draw_ascii` but with a slice of `u8` `chars`.
fn draw_ascii_ref<RB, const C: usize>(
    vid_row: VolBlock<TextEntry, Safe, Safe, C>,
    range: RB,
    chars: &[u8],
    palbank: u16,
) where
    RB: core::ops::RangeBounds<usize>,
{
    draw_ascii(vid_row, range, chars.iter().copied(), palbank)
}

/// Same as `draw_ascii` but with an iterator of `char` `chars`.
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

pub fn clear_status_text() {
    let tsb = TEXT_SCREENBLOCKS.get_frame(31).unwrap();
    draw_ascii_chars(tsb.get_row(19).unwrap(), 0.., repeat(' '), NORMAL_TEXT);
}

pub fn draw_status_text(t: &str) {
    let tsb = TEXT_SCREENBLOCKS.get_frame(31).unwrap();
    draw_ascii_chars(tsb.get_row(19).unwrap(), 0.., t.chars().chain(repeat(' ')), NORMAL_TEXT);
}

pub fn draw_error_text(t: &str) {
    let tsb = TEXT_SCREENBLOCKS.get_frame(31).unwrap();
    draw_ascii_chars(tsb.get_row(19).unwrap(), 0.., t.chars().chain(repeat(' ')), ERROR_TEXT);
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
            [Color(0b0_11000_11000_11000), Color(0b0_00000_00000_10000)],
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
            focused_panel_checker: ChangeChecker::new(FocusedPanel::Steps),
            selected_column_checker: ChangeChecker::new(0),
            selected_step_checker: ChangeChecker::new(0),
            selected_step_range_first_checker: ChangeChecker::new(-1),
            sequencer_pattern_instruments_len_checker: ChangeChecker::new(0),
            sequencer_song_pattern_active_checker: ChangeChecker::new(0),
            sequencer_step_active_checker: ChangeChecker::new(0),
            displayed_instrument_checker: ChangeChecker::new(0),
        }
    }

    pub fn attach_trackers(&self) {
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

    pub fn draw(&mut self) {
        let handle = unsafe { WINDOW.as_ref().unwrap().upgrade().unwrap() };
        let global_engine = GlobalEngine::get(&handle);
        let global_ui = GlobalUI::get(&handle);
        let focused_panel = self.focused_panel_checker.check(handle.get_focused_panel());
        let selected_column = self.selected_column_checker.check(global_ui.get_selected_column());
        let selected_step = self.selected_step_checker.check(global_ui.get_selected_step());
        let selected_step_range_first = self
            .selected_step_range_first_checker
            .check(global_ui.get_selected_step_range_first());
        let sequencer_pattern_instruments_len = self
            .sequencer_pattern_instruments_len_checker
            .check(global_engine.get_sequencer_pattern_instruments_len() as usize);
        let sequencer_song_pattern_active = self
            .sequencer_song_pattern_active_checker
            .check(global_engine.get_sequencer_song_pattern_active() as usize);
        let sequencer_step_active = self
            .sequencer_step_active_checker
            .check(global_engine.get_sequencer_step_active() as usize);
        let displayed_instrument = self
            .displayed_instrument_checker
            .check(global_engine.get_displayed_instrument() as usize);

        let sequencer_song_patterns_dirty = self.sequencer_song_patterns_tracker.take_dirtiness();
        let sequencer_steps_dirty = self.sequencer_steps_tracker.take_dirtiness();
        let sequencer_pattern_instruments_dirty = self.sequencer_pattern_instruments_tracker.take_dirtiness();
        let instruments_dirty = self.instruments_tracker.take_dirtiness();

        let tsb = TEXT_SCREENBLOCKS.get_frame(31).unwrap();
        const PARAMS_START_X: usize = 4;
        const STEPS_START_X: usize = 10;
        const INSTR_START_X: usize = 14;

        let status_vid_row = tsb.get_row(18).unwrap();
        if focused_panel.dirty() {
            draw_ascii_byte(
                status_vid_row,
                0,
                (focused_panel.current == FocusedPanel::Patterns) as u8 * Cga8x8Thick::BULLET,
                NORMAL_TEXT,
            );
            draw_ascii_byte(
                status_vid_row,
                PARAMS_START_X - 1,
                (focused_panel.current == FocusedPanel::Steps) as u8 * Cga8x8Thick::BULLET,
                NORMAL_TEXT,
            );
            draw_ascii_byte(
                status_vid_row,
                INSTR_START_X - 1,
                (focused_panel.current == FocusedPanel::Instruments) as u8 * Cga8x8Thick::BULLET,
                NORMAL_TEXT,
            );
        }

        if sequencer_song_pattern_active.dirty() || sequencer_song_patterns_dirty {
            let pattern_model = global_engine.get_sequencer_song_patterns();
            draw_ascii_ref(tsb.get_row(0).unwrap(), 0.., b"Sng", NORMAL_TEXT);

            // sequencer_song_patterns_tracker will be dirty and trigger a redraw any time this is changed.
            let sequencer_song_pattern_selected = global_engine.get_sequencer_song_pattern_selected() as usize;
            // The display area is 16 rows, scroll the selected pattern into the middle of the screen
            // by scrolling the top of the screen to the selected pattern - 8.
            // Also make sure that the last pattern is at the bottom of the screen when possible.
            let scroll_pos = if pattern_model.row_count() > 8 {
                sequencer_song_pattern_selected.min(pattern_model.row_count() - 8)
            } else {
                sequencer_song_pattern_selected
            }
            .max(8)
                - 8;
            for i in scroll_pos..(scroll_pos + 16) {
                let vid_row = tsb.get_row(i - scroll_pos + 1).unwrap();
                if i < pattern_model.row_count() {
                    let row_data = pattern_model.row_data(i).unwrap();
                    let palbank = if row_data.selected { SELECTED_TEXT } else { NORMAL_TEXT };
                    if row_data.number > -1 {
                        draw_ascii(vid_row, 1.., to_dec(row_data.number as u8 + 1), palbank);
                    } else {
                        draw_ascii(vid_row, 1..3, repeat(b'-'), palbank | FADED_TEXT);
                    }
                } else {
                    draw_ascii(vid_row, 1..3, repeat(b' '), NORMAL_TEXT);
                }
                draw_ascii_byte(
                    vid_row,
                    0,
                    (i == sequencer_song_pattern_active.current) as u8 * Cga8x8Thick::BULLET,
                    NORMAL_TEXT,
                )
            }
        }

        if sequencer_steps_dirty
            || sequencer_step_active.dirty()
            || selected_column.dirty()
            || selected_step.dirty()
            || selected_step_range_first.dirty()
            || focused_panel.dirty()
        {
            let displayed_instrument = global_engine.get_displayed_instrument() as usize;

            // == Header ==
            let vid_row = tsb.get_row(0).unwrap();

            // Don't check for dirtiness, assume this changes together with sequencer_steps_dirty.
            let param_0_def = global_engine.get_instrument_param_0();
            let param_1_def = global_engine.get_instrument_param_1();
            draw_ascii_chars(
                vid_row,
                PARAMS_START_X..PARAMS_START_X + 3,
                param_0_def.name.chars().chain(repeat(' ')),
                NORMAL_TEXT,
            );
            draw_ascii_byte(vid_row, PARAMS_START_X + 2, b'/', FADED_TEXT);
            draw_ascii_chars(
                vid_row,
                PARAMS_START_X + 3..PARAMS_START_X + 5,
                param_1_def.name.chars().chain(repeat(' ')),
                NORMAL_TEXT,
            );

            let displayed_instrument_id = global_engine
                .get_instruments()
                .row_data(displayed_instrument)
                .unwrap()
                .id;
            draw_ascii_chars(
                vid_row,
                STEPS_START_X..STEPS_START_X + 3,
                displayed_instrument_id.chars().chain(repeat(' ')),
                NORMAL_TEXT,
            );

            // == Steps ==
            let sequencer_steps = global_engine.get_sequencer_steps();
            for i in 0..sequencer_steps.row_count() {
                let vid_row = tsb.get_row(i + 1).unwrap();
                let row_data = sequencer_steps.row_data(i).unwrap();

                let ii = i as i32;
                let selected = focused_panel.current == FocusedPanel::Steps
                    && (selected_step.current == ii
                        || (selected_step_range_first.current > selected_step.current
                            && selected_step.current <= ii
                            && ii <= selected_step_range_first.current)
                        || (selected_step_range_first.current != -1
                            && selected_step_range_first.current <= ii
                            && ii <= selected_step.current));

                let param0_bank = if selected && selected_column.current == 0 {
                    SELECTED_TEXT
                } else {
                    NORMAL_TEXT
                };
                let param1_bank = if selected && selected_column.current == 1 {
                    SELECTED_TEXT
                } else {
                    NORMAL_TEXT
                };
                let press_bank = if selected && selected_column.current == 2 {
                    SELECTED_TEXT
                } else {
                    NORMAL_TEXT
                };
                let release_bank = if selected && selected_column.current >= 2 {
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

                // Draw note
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

                // "current" indicator
                draw_ascii_byte(
                    vid_row,
                    PARAMS_START_X - 1,
                    (i == sequencer_step_active.current) as u8 * Cga8x8Thick::BULLET,
                    NORMAL_TEXT,
                );

                // Press and release brackets
                draw_ascii_byte(vid_row, STEPS_START_X - 1, row_data.press as u8 * b'[', press_bank);
                draw_ascii_byte(vid_row, STEPS_START_X + 3, row_data.release as u8 * b']', release_bank);
            }
        }

        let toggled_instruments = focused_panel.dirty()
            && (focused_panel.current == FocusedPanel::Instruments
                || focused_panel.previous == FocusedPanel::Instruments);
        if toggled_instruments {
            for i in 0..17 {
                draw_ascii(tsb.get_row(i).unwrap(), INSTR_START_X.., repeat(0), NORMAL_TEXT);
            }
            if focused_panel.current == FocusedPanel::Instruments {
                draw_ascii_ref(tsb.get_row(0).unwrap(), INSTR_START_X.., b"Instruments", NORMAL_TEXT);
            }
        }
        if focused_panel.current != FocusedPanel::Instruments
            && (toggled_instruments || sequencer_pattern_instruments_len.dirty() || sequencer_pattern_instruments_dirty)
        {
            let top_vid_row = tsb.get_row(0).unwrap();
            let sequencer_pattern_instruments = global_engine.get_sequencer_pattern_instruments();
            for i in 0..5 {
                let x = i * 3 + INSTR_START_X;
                let col_bank = if i % 2 == 0 { ALT_COL_TEXT } else { NORMAL_TEXT };
                let normal_bank = col_bank | NORMAL_TEXT;
                let faded_bank = col_bank | FADED_TEXT;

                if i < sequencer_pattern_instruments_len.current {
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
        if focused_panel.current == FocusedPanel::Instruments
            && (toggled_instruments || displayed_instrument.dirty() || instruments_dirty)
        {
            let instruments = global_engine.get_instruments();
            let scroll_pos =
                // 4 instruments per row
                (displayed_instrument.current / 4)
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
                    let palbank = if instrument_idx == displayed_instrument.current {
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
