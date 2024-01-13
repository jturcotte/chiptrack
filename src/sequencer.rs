// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

#[cfg(feature = "desktop")]
mod markdown;

use crate::sound_engine::NUM_INSTRUMENTS;
use crate::sound_engine::NUM_PATTERNS;
use crate::sound_engine::NUM_STEPS;
use crate::utils::MidiNote;
use crate::utils::WeakWindowWrapper;
use crate::GlobalEngine;
use crate::GlobalSettings;
use crate::SongPatternData;
use crate::SongSettings;
use core::fmt;
use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::ser::{SerializeStruct, Serializer};

#[cfg(feature = "desktop")]
use markdown::{parse_markdown_song, save_markdown_song};
#[cfg(feature = "gba")]
use postcard::from_bytes;
#[cfg(feature = "desktop")]
use postcard::to_allocvec;
use serde::Deserialize;
use serde::Serialize;
use slint::Global;
use slint::Model;
use slint::SharedString;
use slint::VecModel;

use alloc::borrow::ToOwned;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
#[cfg(feature = "desktop")]
use std::error::Error;
#[cfg(feature = "desktop")]
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq)]
enum KeyEvent {
    Press,
    Release,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum StepEvent {
    Press(u8, i8, i8), // note, p0, p1
    Release,
    SetParam(u8, i8), // param_num, val
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct InstrumentStep {
    note: u8,
    release: bool,
    param0: i8,
    param1: i8,
}

impl InstrumentStep {
    const FIELDS: &'static [&'static str] = &["note", "flags", "param0", "param1"];

    pub fn set_press_note(&mut self, note: Option<u8>) {
        match note {
            None => self.note = 0,
            Some(v) => {
                debug_assert!(v != 0, "0 can't be set as a note");
                self.note = v
            }
        }
    }

    pub fn press(&self) -> bool {
        self.note != 0
    }

    pub fn press_note(&self) -> Option<u8> {
        // 0 is a valid MIDI note, but the GBA hardware doesn't support that frequence, so use it to represent "no press".
        if self.note == 0 {
            None
        } else {
            Some(self.note)
        }
    }
}
/// Use a custom serializer instead of derived to represent None parameters as single bits instead of separate 0 bytes
/// so that we need 2 bytes per step instead of 4.
impl Serialize for InstrumentStep {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let p0 = self.param0();
        let p1 = self.param1();
        let num_params = p0.is_some() as usize + p1.is_some() as usize;
        let note = (self.release as u8) << 7 | self.note;
        let flags = (p1.is_some() as u8) << 1 | p0.is_some() as u8;

        let mut rgb = serializer.serialize_struct("InstrumentStep", 2 + num_params)?;
        rgb.serialize_field(InstrumentStep::FIELDS[0], &note)?;
        rgb.serialize_field(InstrumentStep::FIELDS[1], &flags)?;
        if let Some(val) = p0 {
            rgb.serialize_field(InstrumentStep::FIELDS[2], &val)?;
        }
        if let Some(val) = p1 {
            rgb.serialize_field(InstrumentStep::FIELDS[3], &val)?;
        }
        rgb.end()
    }
}

impl<'de> Deserialize<'de> for InstrumentStep {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct InstrumentStepVisitor;

        impl<'de> Visitor<'de> for InstrumentStepVisitor {
            type Value = InstrumentStep;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct InstrumentStep")
            }

            // Used for deserializing postcard.
            fn visit_seq<V>(self, mut seq: V) -> Result<InstrumentStep, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let note: u8 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let flags: u8 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let mut i = InstrumentStep {
                    note: note & 0x7f,
                    release: note & 0x80 != 0,
                    param0: -128,
                    param1: -128,
                };
                if flags & 0b01 != 0 {
                    let val: i8 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(2, &self))?;
                    i.set_param0(Some(val));
                }
                if flags & 0b10 != 0 {
                    let val: i8 = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(2 + flags.count_ones() as usize - 1, &self))?;
                    i.set_param1(Some(val));
                }
                Ok(i)
            }
        }
        deserializer.deserialize_struct("InstrumentStep", InstrumentStep::FIELDS, InstrumentStepVisitor)
    }
}

#[test]
fn postcard_serialize() -> Result<(), Box<dyn Error>> {
    let i = InstrumentStep {
        note: 36,
        release: true,
        param0: -128,
        param1: 8,
    };

    let ser = postcard::to_allocvec(&i)?;
    let r: InstrumentStep = postcard::from_bytes(&ser)?;
    assert!(r == i);
    Ok(())
}

impl InstrumentStep {
    fn param0(&self) -> Option<i8> {
        if self.param0 == -128 {
            None
        } else {
            Some(self.param0)
        }
    }
    fn param1(&self) -> Option<i8> {
        if self.param1 == -128 {
            None
        } else {
            Some(self.param1)
        }
    }
    fn set_param0(&mut self, val: Option<i8>) {
        match val {
            // Unset is encoded as -128 (0x80) to use the same byte, rust won't find that niche by itself.
            None => self.param0 = -128,
            Some(v) => self.param0 = v,
        }
    }
    fn set_param1(&mut self, val: Option<i8>) {
        match val {
            None => self.param1 = -128,
            Some(v) => self.param1 = v,
        }
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Instrument {
    id: String,
    #[serde(skip)]
    synth_index: Option<u8>,
    steps: [InstrumentStep; NUM_STEPS],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Pattern {
    instruments: Vec<Instrument>,
}

impl Pattern {
    fn is_empty(&self) -> bool {
        self.instruments.is_empty()
    }

    fn get_steps(&self, instrument: u8) -> Option<&[InstrumentStep; NUM_STEPS]> {
        self.instruments
            .iter()
            .find(|i| i.synth_index == Some(instrument))
            .map(|i| &i.steps)
    }

    fn next_instrument(&self, current_instrument: u8, forward: bool) -> Option<u8> {
        match self.find_nearest_instrument_pos(current_instrument, forward) {
            // current_instrument is already in the pattern, cycle once forward
            Some((ii, i)) if i == current_instrument && forward => self
                .instruments
                .iter()
                .cycle()
                .skip(ii + 1)
                .take(self.instruments.len() - 1)
                .find_map(|i| i.synth_index),
            // current_instrument is already in the pattern, cycle once backwards
            Some((ii, i)) if i == current_instrument => self
                .instruments
                .iter()
                .rev()
                .cycle()
                .skip(self.instruments.len() - ii)
                .take(self.instruments.len() - 1)
                .find_map(|i| i.synth_index),
            // i is already the nearest in the requested direction
            Some((_, i)) => Some(i),
            // Nothing to return
            None => None,
        }
    }

    fn order(a: u8) -> u32 {
        let mut ai = a as u32;
        // Instrument are indiced by UI pages and are sequenced by row,
        // but we want to sort by column first, so change the order by moving
        // the 2 column bits from being least significant to being most significant.
        ai |= (ai & 0x3) << 8;
        ai
    }

    fn find_nearest_instrument_pos(&self, instrument: u8, forward: bool) -> Option<(usize, u8)> {
        let cmp = if forward { |t, b| t < b } else { |t, b| t > b };
        let mut best = None;

        for (ii, i) in self.instruments.iter().enumerate() {
            best = match (best, i.synth_index) {
                // Found exactly what we were looking for, return immediately
                (_, Some(this_instrument)) if this_instrument == instrument => return Some((ii, this_instrument)),

                // Found an instrument ID closer to the seeked instrument, keep it
                (Some((_, best_instrument)), Some(this_instrument))
                    if cmp(
                        Self::order(this_instrument).overflowing_sub(Self::order(instrument)).0,
                        Self::order(best_instrument).overflowing_sub(Self::order(instrument)).0,
                    ) =>
                {
                    Some((ii, this_instrument))
                }

                // There is no synth_index or it was not closer, keep the best
                (Some(_), _) => best,

                // No best yet, keep the new instrument
                (None, Some(this_instrument)) => Some((ii, this_instrument)),

                // Nothing to keep
                (None, None) => None,
            }
        }
        best
    }

    fn instruments(&self) -> &Vec<Instrument> {
        &self.instruments
    }

    fn get_steps_mut<'a>(
        &'a mut self,
        instrument_id: &str,
        synth_index: Option<u8>,
    ) -> &'a mut [InstrumentStep; NUM_STEPS] {
        // Use the string ID to match the instrument so that reloading instruments and moving
        // instruments to different synth_indices will still match them.
        // FIXME: Check why the string ID isn't used for get_steps
        let ii = match self.instruments.iter().position(|i| i.id == instrument_id) {
            Some(ii) => ii,
            None => {
                self.instruments.push(Instrument {
                    id: instrument_id.to_owned(),
                    synth_index,
                    steps: Default::default(),
                });
                self.instruments.len() - 1
            }
        };
        &mut self.instruments[ii].steps
    }

    fn set_step_events(
        &mut self,
        instrument: u8,
        instrument_id: &str,
        step_num: usize,
        set_press_note: Option<Option<u8>>,
        set_release: Option<bool>,
        set_params: Option<(Option<i8>, Option<i8>)>,
    ) {
        let step = &mut self.get_steps_mut(instrument_id, Some(instrument))[step_num];

        if let Some(release) = set_release {
            step.release = release;
        }
        if let Some(note) = set_press_note {
            step.set_press_note(note);
        }
        if let Some((param0, param1)) = set_params {
            step.set_param0(param0);
            step.set_param1(param1);
        }

        // FIXME: Remove empty instruments
    }

    fn update_synth_index(&mut self, new_instrument_ids: &[SharedString]) {
        for instrument in &mut self.instruments {
            let index = new_instrument_ids
                .iter()
                .position(|s| instrument.id.as_str() == s.as_str());
            instrument.synth_index = index.map(|p| p as u8);
            if instrument.synth_index.is_none() {
                elog!(
                    "Some song pattern refers to instrument id [{}], but the instruments didn't register it, ignoring.",
                    instrument.id
                );
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SequencerSong {
    song_patterns: Vec<usize>,
    patterns: Vec<Pattern>,
    frames_per_step: u32,
    #[serde(skip)]
    #[cfg(feature = "desktop")]
    markdown_header: String,
    #[serde(skip)]
    #[cfg(feature = "desktop")]
    instruments_file: String,
}

// Initialize all notes to C5
const DEFAULT_NOTE: u8 = 60;
const DEFAULT_PARAM_VAL: i8 = 0;

impl Default for InstrumentStep {
    fn default() -> Self {
        InstrumentStep {
            note: 0,
            release: false,
            param0: -128,
            param1: -128,
        }
    }
}

impl Default for SequencerSong {
    fn default() -> Self {
        SequencerSong {
            song_patterns: Vec::new(),
            patterns: vec![
                Pattern {
                    instruments: Vec::new(),
                };
                NUM_PATTERNS
            ],
            frames_per_step: 7,
            #[cfg(feature = "desktop")]
            markdown_header: String::new(),
            #[cfg(feature = "desktop")]
            instruments_file: String::new(),
        }
    }
}

struct NoteClipboard {
    note: u8,
    release: bool,
}

pub struct Sequencer {
    pub song: SequencerSong,
    active_frame: u32,
    active_step: usize,
    active_song_pattern: usize,
    pub selected_instrument: u8,
    selected_step: usize,
    selected_song_pattern: usize,
    pin_selection_to_active: bool,
    playing: bool,
    recording: bool,
    erasing: bool,
    has_stub_pattern: bool,
    last_press_frame: Option<u32>,
    just_recorded_over_next_step: bool,
    // FIXME: Use a bitset
    muted_instruments: BTreeSet<u8>,
    synth_instrument_ids: Vec<SharedString>,
    instrument_params: Vec<(i8, i8)>,
    note_clipboard: NoteClipboard,
    main_window: WeakWindowWrapper,
}

impl Sequencer {
    pub fn new(main_window: WeakWindowWrapper) -> Sequencer {
        Sequencer {
            song: Default::default(),
            active_frame: 0,
            active_step: 0,
            active_song_pattern: 0,
            selected_instrument: 0,
            selected_step: 0,
            selected_song_pattern: 0,
            pin_selection_to_active: true,
            playing: false,
            recording: true,
            erasing: false,
            has_stub_pattern: false,
            last_press_frame: None,
            just_recorded_over_next_step: false,
            muted_instruments: BTreeSet::new(),
            synth_instrument_ids: vec![SharedString::new(); NUM_INSTRUMENTS],
            instrument_params: vec![(0, 0); NUM_INSTRUMENTS],
            note_clipboard: NoteClipboard {
                note: DEFAULT_NOTE,
                release: true,
            },
            main_window: main_window.clone(),
        }
    }

    fn pattern_idx(&self, song_pattern_idx: usize) -> usize {
        self.song.song_patterns[song_pattern_idx]
    }

    fn active_pattern_idx(&self) -> usize {
        self.pattern_idx(self.active_song_pattern)
    }

    fn selected_pattern_idx(&self) -> usize {
        self.pattern_idx(self.selected_song_pattern)
    }

    fn activate_song_pattern(&mut self, song_pattern: usize) {
        self.active_song_pattern = song_pattern;

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                GlobalEngine::get(&handle).set_sequencer_song_pattern_active(song_pattern as i32);
            })
            .unwrap();

        if self.playing && self.pin_selection_to_active {
            self.select_song_pattern_internal(song_pattern);
            self.update_steps();
        }
    }

    fn activate_step(&mut self, step: usize) {
        self.active_step = step;

        let active_step = if self.active_song_pattern == self.selected_song_pattern {
            step as i32
        } else {
            -1
        };

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                GlobalEngine::get(&handle).set_sequencer_step_active(active_step);
            })
            .unwrap();

        if self.playing && self.pin_selection_to_active {
            self.select_step_internal(step);
        }
    }

    pub fn apply_song_settings(&mut self, settings: &SongSettings) {
        self.song.frames_per_step = settings.frames_per_step as u32;
    }

    pub fn select_next_song_pattern(&mut self, forward: bool) {
        let selected = self.selected_song_pattern;
        // Leave the possibility of selecting the stub pattern
        let last_pattern = self.song.song_patterns.len() - if self.has_stub_pattern { 1 } else { 0 };

        if forward && selected < last_pattern {
            self.select_song_pattern(selected + 1);
        } else if !forward && selected > 0 {
            self.select_song_pattern(selected - 1);
        };
    }

    pub fn select_song_pattern(&mut self, song_pattern: usize) {
        self.select_song_pattern_internal(song_pattern);

        if self.playing {
            self.pin_selection_to_active = false;
        } else {
            self.pin_selection_to_active = true;
            self.activate_song_pattern(song_pattern);
        }

        // update_steps relies on both the active and selected song pattern to be set to be able
        // to properly highlight the active step.
        self.update_steps();
    }

    fn select_song_pattern_internal(&mut self, song_pattern: usize) {
        if !self.has_stub_pattern && song_pattern == self.song.song_patterns.len() {
            // Only append the stub while selecting it so that it doesn't affect playback unless selected
            // and also to avoid allowing the user to change patterns before the selection, which could
            // prevent us from picking the best match for the next pattern index.
            self.append_stub_song_pattern();
        } else if self.has_stub_pattern && song_pattern != self.selected_song_pattern {
            self.remove_stub_song_pattern();
        }
        let prev_selected = self.selected_song_pattern;
        self.selected_song_pattern = song_pattern;

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                GlobalEngine::get(&handle).set_sequencer_song_pattern_selected(song_pattern as i32);
                let model = GlobalEngine::get(&handle).get_sequencer_song_patterns();
                let mut row_data = model.row_data(prev_selected).unwrap();
                row_data.selected = false;
                model.set_row_data(prev_selected, row_data);
                let mut row_data = model.row_data(song_pattern).unwrap();
                row_data.selected = true;
                model.set_row_data(song_pattern, row_data);
            })
            .unwrap();
    }

    pub fn select_step(&mut self, step: usize) {
        self.select_step_internal(step);

        if self.playing {
            self.pin_selection_to_active = false;
        } else {
            if !self.pin_selection_to_active {
                self.pin_selection_to_active = true;
                // When re-pinning, make sure that the selected pattern is also activated.
                // activate_step also need this to properly show the current step as active.
                self.activate_song_pattern(self.selected_song_pattern);
            }
            self.activate_step(step);
        }
    }

    fn select_step_internal(&mut self, step: usize) {
        let prev_selected = self.selected_step;
        self.selected_step = step;

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let steps = GlobalEngine::get(&handle).get_sequencer_steps();
                let mut row_data = steps.row_data(prev_selected).unwrap();
                row_data.selected = false;
                steps.set_row_data(prev_selected, row_data);
                let mut row_data = steps.row_data(step).unwrap();
                row_data.selected = true;
                steps.set_row_data(step, row_data);
            })
            .unwrap();
    }

    pub fn select_next_step(&mut self, forward: bool) {
        let (next_step, next_song_pattern) = Self::next_step_and_pattern_and_song_pattern(
            forward,
            self.selected_step,
            self.selected_song_pattern,
            self.num_song_patterns_including_stub(),
        );

        // Potentially activate the pattern first so that activate_step knows that it's current.
        if next_song_pattern != self.selected_song_pattern {
            self.select_song_pattern(next_song_pattern);
        }

        self.select_step(next_step);
    }

    pub fn select_instrument(&mut self, instrument: u8) {
        self.selected_instrument = instrument;

        self.update_steps();

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                GlobalEngine::get(&handle).set_selected_instrument(instrument as i32);
            })
            .unwrap();
    }

    pub fn cycle_instrument(&mut self, col_delta: i32, row_delta: i32) {
        // Wrap
        let col = (self.selected_instrument as i32 + 4 + col_delta) % 4;
        // Don't wrap
        let row = (self.selected_instrument as i32 / 4 + row_delta).max(0).min(15);
        self.select_instrument((col + row * 4) as u8)
    }

    pub fn cycle_pattern_instrument(&mut self, forward: bool) {
        let maybe_next =
            self.song.patterns[self.selected_pattern_idx()].next_instrument(self.selected_instrument, forward);
        if let Some(instrument) = maybe_next {
            self.select_instrument(instrument)
        }
    }

    pub fn toggle_mute_instrument(&mut self, instrument: u8) {
        let was_muted = self.muted_instruments.take(&instrument).is_some();
        if !was_muted {
            self.muted_instruments.insert(instrument);
        }

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = GlobalEngine::get(&handle).get_instruments();
                let mut row_data = model.row_data(instrument as usize).unwrap();
                row_data.muted = !was_muted;
                model.set_row_data(instrument as usize, row_data);
            })
            .unwrap();
    }

    fn update_steps(&mut self) {
        let pattern = &self.song.patterns[self.selected_pattern_idx()];
        let maybe_steps = pattern.get_steps(self.selected_instrument).copied();

        let active_step = if self.active_song_pattern == self.selected_song_pattern {
            self.active_step as i32
        } else {
            -1
        };

        // Don't clone the pattern instruments if the closure will be handled synchronously.
        #[cfg(not(feature = "std"))]
        let instruments = pattern.instruments();
        #[cfg(feature = "std")]
        let instruments = pattern.instruments().clone();

        let (instruments_to_skip, instruments_len) =
            match pattern.find_nearest_instrument_pos(self.selected_instrument, true) {
                // Advance once more to not show the found instrument both in the patterns and pattern_instruments models
                Some((ii, i)) if i == self.selected_instrument => (ii + 1, instruments.len() - 1),
                Some((ii, _)) => (ii, instruments.len()),
                None => (0, 0),
            };

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = GlobalEngine::get(&handle).get_sequencer_steps();
                for (i, step) in maybe_steps.unwrap_or_default().iter().enumerate() {
                    let mut row_data = model.row_data(i).unwrap();
                    row_data.press = step.press();
                    row_data.release = step.release;
                    row_data.note = step.press_note().unwrap_or(0) as i32;
                    match step.param0() {
                        Some(v) => {
                            row_data.param0_set = true;
                            row_data.param0_val = v as i32;
                        }
                        None => row_data.param0_set = false,
                    }
                    match step.param1() {
                        Some(v) => {
                            row_data.param1_set = true;
                            row_data.param1_val = v as i32;
                        }
                        None => row_data.param1_set = false,
                    }

                    model.set_row_data(i, row_data);
                }

                GlobalEngine::get(&handle).set_sequencer_step_active(active_step);

                let model2 = GlobalEngine::get(&handle).get_sequencer_pattern_instruments();
                let len = instruments_len.min(model2.row_count());
                GlobalEngine::get(&handle).set_sequencer_pattern_instruments_len(len as i32);

                instruments
                    .iter()
                    .cycle()
                    .skip(instruments_to_skip)
                    .take(len)
                    .enumerate()
                    .for_each(|(idx, mi)| {
                        let mut instrument_data = model2.row_data(idx).unwrap();

                        let notes_model = &instrument_data.notes;
                        mi.steps.iter().enumerate().for_each(|(idx, s)| {
                            notes_model.set_row_data(idx, s.press_note().map_or(-1, |n| n as i32));
                        });

                        instrument_data.id = (&mi.id).into();
                        model2.set_row_data(idx, instrument_data);
                    });
            })
            .unwrap();
    }

    pub fn toggle_step(&mut self, step_num: usize) {
        // Assuming that toggle is called from a mouse press, select the step (FIXME: decouple)
        self.select_step(step_num);

        let maybe_steps = self.song.patterns[self.selected_pattern_idx()].get_steps(self.selected_instrument);
        let pressed = maybe_steps.map_or(false, |ss| ss[step_num].press());
        if pressed {
            self.cut_step_note(step_num);
        } else {
            self.cycle_selected_step_note(None, false);
        }
    }

    pub fn copy_step_note(&mut self, step_num: usize) {
        let maybe_steps = self.song.patterns[self.selected_pattern_idx()].get_steps(self.selected_instrument);
        let pressed_note_and_release =
            maybe_steps.and_then(|ss| ss[step_num].press_note().map(|p| (p, ss[step_num].release)));
        if let Some((note, release)) = pressed_note_and_release {
            self.note_clipboard = NoteClipboard { note, release };
        }
    }

    pub fn cut_step_note(&mut self, step_num: usize) {
        self.copy_step_note(step_num);
        self.copy_step_params(step_num, None);

        self.set_pattern_step_events(
            step_num,
            self.selected_song_pattern,
            Some(None),
            Some(false),
            Some((None, None)),
        );
    }

    pub fn copy_selected_step_note(&mut self) {
        self.copy_step_note(self.selected_step);
    }

    pub fn cut_selected_step_note(&mut self) {
        self.cut_step_note(self.selected_step);
    }

    pub fn toggle_step_release(&mut self, step_num: usize) {
        let maybe_steps = self.song.patterns[self.active_pattern_idx()].get_steps(self.selected_instrument);
        let toggled = !maybe_steps.map_or(false, |ss| ss[step_num].release);
        self.set_pattern_step_events(step_num, self.selected_song_pattern, None, Some(toggled), None);

        self.copy_step_note(step_num);
        self.select_step(step_num);
    }

    pub fn toggle_selected_step_release(&mut self) {
        self.toggle_step_release(self.selected_step)
    }

    fn advance_step(&mut self, forward: bool) {
        let (next_step, next_song_pattern) = Self::next_step_and_pattern_and_song_pattern(
            forward,
            self.active_step,
            self.active_song_pattern,
            self.num_song_patterns(),
        );

        if next_song_pattern != self.active_song_pattern {
            self.activate_song_pattern(next_song_pattern);
        }

        self.activate_step(next_step);
    }

    fn set_pattern_step_events(
        &mut self,
        step_num: usize,
        song_pattern: usize,
        set_press_note: Option<Option<u8>>,
        set_release: Option<bool>,
        set_params: Option<(Option<i8>, Option<i8>)>,
    ) {
        if song_pattern == self.song.song_patterns.len() - 1 {
            self.commit_stub_song_pattern();
        }

        let pattern = self.pattern_idx(song_pattern);
        let instrument_id = &self.synth_instrument_ids[self.selected_instrument as usize];
        self.song.patterns[pattern].set_step_events(
            self.selected_instrument,
            instrument_id,
            step_num,
            set_press_note,
            set_release,
            set_params,
        );

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let steps = GlobalEngine::get(&handle).get_sequencer_steps();
                let mut step_row_data = steps.row_data(step_num).unwrap();
                if let Some(maybe_note) = set_press_note {
                    step_row_data.press = maybe_note.is_some();
                    step_row_data.note = maybe_note.unwrap_or(0) as i32;
                }
                if let Some(release) = set_release {
                    step_row_data.release = release;
                }
                if let Some((param0, param1)) = set_params {
                    match param0 {
                        Some(v) => {
                            step_row_data.param0_set = true;
                            step_row_data.param0_val = v as i32;
                        }
                        None => step_row_data.param0_set = false,
                    }
                    match param1 {
                        Some(v) => {
                            step_row_data.param1_set = true;
                            step_row_data.param1_val = v as i32;
                        }
                        None => step_row_data.param1_set = false,
                    }
                }
                steps.set_row_data(step_num, step_row_data);
            })
            .unwrap();
    }
    pub fn playing(&self) -> bool {
        self.playing
    }

    pub fn set_playing(&mut self, val: bool) -> Vec<(u8, StepEvent)> {
        self.playing = val;
        // Reset the active_frame so that it's aligned with full
        // steps and that record_press would record any key while
        // stopped to the current frame and not the next.
        self.active_frame = 0;

        if self.playing {
            // The first advance_frame after playing will move from frame 0 to frame 1 and skip presses
            // of frame 0. Since we don't care about releases of the non-existant previous frame, do the
            // presses now, right after starting the playback.
            let mut note_events: Vec<(u8, StepEvent)> = Vec::new();
            self.handle_active_step_presses_and_params(&mut note_events);
            note_events
        } else {
            Vec::new()
        }
    }
    pub fn set_recording(&mut self, val: bool) {
        self.recording = val;
    }
    pub fn set_erasing(&mut self, val: bool) {
        self.erasing = val;
        // Already remove the current step.
        self.set_pattern_step_events(
            self.active_step,
            self.active_song_pattern,
            Some(None),
            Some(false),
            Some((None, None)),
        );
    }

    fn handle_active_step_presses_and_params(&mut self, note_events: &mut Vec<(u8, StepEvent)>) {
        for instrument in &self.song.patterns[self.active_pattern_idx()].instruments {
            let i = match instrument.synth_index {
                Some(i) => i,
                None => {
                    // instruments don't define it, ignore
                    continue;
                }
            };
            if self.muted_instruments.contains(&i) {
                continue;
            }
            if self.just_recorded_over_next_step && i == self.selected_instrument {
                self.just_recorded_over_next_step = false;
                continue;
            }

            if let Some(step) = self.song.patterns[self.active_pattern_idx()]
                .get_steps(i)
                .map(|ss| ss[self.active_step])
            {
                if let Some(note) = step.press_note() {
                    // Use the default if the sequencer didn't have any param set for that press.
                    let p0 = step.param0().unwrap_or(DEFAULT_PARAM_VAL);
                    let p1 = step.param1().unwrap_or(DEFAULT_PARAM_VAL);
                    log!(
                        "➕ PRS {} note {} params {} / {}",
                        self.synth_instrument_ids[i as usize],
                        MidiNote(note as i32).name(),
                        p0,
                        p1
                    );
                    note_events.push((i, StepEvent::Press(step.note, p0, p1)));
                } else {
                    if let Some(val) = step.param0() {
                        log!("✖️ PAR {} param {} = {}", self.synth_instrument_ids[i as usize], 0, val);
                        note_events.push((i, StepEvent::SetParam(0, val)));
                    }
                    if let Some(val) = step.param1() {
                        log!("✖️ PAR {} param {} = {}", self.synth_instrument_ids[i as usize], 1, val);
                        note_events.push((i, StepEvent::SetParam(1, val)));
                    }
                }
            }
        }
    }

    fn handle_active_step_releases(&mut self, note_events: &mut Vec<(u8, StepEvent)>) {
        for instrument in &self.song.patterns[self.active_pattern_idx()].instruments {
            let i = match instrument.synth_index {
                Some(i) => i,
                None => {
                    // instruments don't define it, ignore
                    continue;
                }
            };
            if self.muted_instruments.contains(&i) {
                continue;
            }
            if self.just_recorded_over_next_step && i == self.selected_instrument {
                // Let the press loop further down reset the flag.
                continue;
            }
            if let Some(InstrumentStep {
                note: _,
                release,
                param0: _,
                param1: _,
            }) = self.song.patterns[self.active_pattern_idx()]
                .get_steps(i)
                .map(|ss| ss[self.active_step])
            {
                if release {
                    log!("➖ REL {}", self.synth_instrument_ids[i as usize]);
                    note_events.push((i, StepEvent::Release));
                }
            }
        }
    }

    pub fn advance_frame(&mut self) -> (Option<u32>, Vec<(u8, StepEvent)>) {
        let mut note_events: Vec<(u8, StepEvent)> = Vec::new();

        if !self.playing {
            return (None, note_events);
        }

        // FIXME: Reset or remove overflow check
        self.active_frame += 1;
        if self.active_frame % self.song.frames_per_step == 0 {
            // Release are at then end of a step, so start by triggering any release of the
            // previous frame.
            self.handle_active_step_releases(&mut note_events);

            self.advance_step(true);
            if self.erasing {
                self.set_pattern_step_events(
                    self.active_step,
                    self.active_song_pattern,
                    Some(None),
                    Some(false),
                    Some((None, None)),
                );
            }

            self.handle_active_step_presses_and_params(&mut note_events);
            (Some(self.active_step as u32), note_events)
        } else {
            (None, note_events)
        }
    }

    pub fn selected_note(&self) -> u8 {
        let maybe_steps = self.song.patterns[self.active_pattern_idx()].get_steps(self.selected_instrument);
        maybe_steps
            .and_then(|steps| steps[self.selected_step].press_note())
            .unwrap_or(self.note_clipboard.note)
    }

    // pub fn selected_note_and_params(&self) -> (u8, i8, i8) {
    //     let maybe_steps = self.song.patterns[self.active_pattern_idx()].get_steps(self.selected_instrument);
    //     maybe_steps
    //         .map(|steps| {
    //             let s = steps[self.selected_step];
    //             (
    //                 s.note,
    //                 s.param0().unwrap_or(DEFAULT_PARAM_VAL),
    //                 s.param1().unwrap_or(DEFAULT_PARAM_VAL),
    //             )
    //         })
    //         .unwrap_or((DEFAULT_NOTE, DEFAULT_PARAM_VAL, DEFAULT_PARAM_VAL))
    // }

    fn record_key_event(&mut self, event: KeyEvent, note: Option<u8>, params: Option<(Option<i8>, Option<i8>)>) {
        if !self.recording {
            return;
        }

        let (press_note, release, (step, song_pattern)) = match event {
            KeyEvent::Press if !self.playing => {
                let pressed = self.song.patterns[self.active_pattern_idx()]
                    .get_steps(self.selected_instrument)
                    .map_or(false, |ss| ss[self.active_step].press());
                if !pressed {
                    // If the step isn't already pressed, record it both as pressed and released.
                    (Some(note), Some(true), (self.active_step, self.active_song_pattern))
                } else {
                    // Else, only set the note.
                    (Some(note), None, (self.active_step, self.active_song_pattern))
                }
            }
            KeyEvent::Release if !self.playing =>
            // Ignore the release when recording and not playing,
            // it should be the same step as the press anyway.
            {
                return
            }
            KeyEvent::Press => {
                (
                    Some(note),
                    None,
                    // Try to clamp the event to the nearest frame.
                    // Use 4 instead of 3 just to try to compensate for the key press to visual and audible delay.
                    if self.active_frame % self.song.frames_per_step < self.snap_at_step_frame() {
                        (self.active_step, self.active_song_pattern)
                    } else {
                        self.just_recorded_over_next_step = true;
                        Self::next_step_and_pattern_and_song_pattern(
                            true,
                            self.active_step,
                            self.active_song_pattern,
                            self.num_song_patterns(),
                        )
                    },
                )
            }
            KeyEvent::Release => {
                // Align the release with the same frame position within the step as the press had.
                // We're going to sequence full steps anyway.
                // This is to prevent the release to be offset only by one frame but still end up
                // one step later just because the press would already have been on the step's edge itself.
                // To do so, first find the frames length rounded to the number of frames per step,
                // and add it to the press frame.
                fn round(n: u32, to: u32) -> u32 {
                    (n + to / 2) / to * to
                }
                let rounded_steps_note_length = round(
                    self.active_frame - self.last_press_frame.unwrap(),
                    self.song.frames_per_step,
                );
                // We need to place the release in the previous step (its end), so substract one step.
                let rounded_end_frame = self.last_press_frame.unwrap() + (rounded_steps_note_length.max(1) - 1);

                let is_end_in_prev_step =
                    rounded_end_frame / self.song.frames_per_step < self.active_frame / self.song.frames_per_step;
                let end_snaps_to_next_step = rounded_end_frame % self.song.frames_per_step < self.snap_at_step_frame();
                (
                    None,
                    Some(true),
                    if is_end_in_prev_step && end_snaps_to_next_step {
                        // It ends before the snap frame of the previous step.
                        // Register the release at the end of the previous step.
                        Self::next_step_and_pattern_and_song_pattern(
                            false,
                            self.active_step,
                            self.active_song_pattern,
                            self.num_song_patterns(),
                        )
                    } else if is_end_in_prev_step || end_snaps_to_next_step {
                        // It ends between the snap frame of the previous step and the snap frame of the current step
                        // Register the release at the end of the current step.
                        (self.active_step, self.active_song_pattern)
                    } else {
                        self.just_recorded_over_next_step = true;
                        // It ends on or after the snap frame of the current step.
                        // Register the release at the end of the next step.
                        Self::next_step_and_pattern_and_song_pattern(
                            true,
                            self.active_step,
                            self.active_song_pattern,
                            self.num_song_patterns(),
                        )
                    },
                )
            }
        };
        self.set_pattern_step_events(step, song_pattern, press_note, release, params);
    }

    pub fn record_press(&mut self, note: u8) -> (i8, i8) {
        let (p0, p1) = self.selected_instrument_params();
        self.record_key_event(
            KeyEvent::Press,
            Some(note),
            Some((Some(p0).filter(|v| *v != 0), Some(p1).filter(|v| *v != 0))),
        );
        self.last_press_frame = Some(self.active_frame);

        // It's a bit weird to keep the release part of the clipboard here,
        // but the fact that recording affects the active step instead of selected one
        // makes it difficult to find the right compromize.
        self.note_clipboard.note = note;
        (p0, p1)
    }

    pub fn record_release(&mut self, _note: u8) {
        // The note release won't be passed to the synth on playback,
        // so don't overwrite the note in the step just in case it contained something useful.
        self.record_key_event(KeyEvent::Release, None, None);
    }

    pub fn cycle_selected_step_note(&mut self, forward: Option<bool>, large_inc: bool) -> (u8, i8, i8) {
        // The GBA only handles frenquencies from C1 upwards.
        const LOWEST_NOTE: u8 = 24;

        let (maybe_selected_note, p0, p1) = self.song.patterns[self.selected_pattern_idx()]
            .get_steps(self.selected_instrument)
            .map_or((None, None, None), |ss| {
                let s = ss[self.selected_step];
                (s.press_note(), s.param0(), s.param1())
            });
        let inc = if large_inc { 12 } else { 1 };
        let active_note = maybe_selected_note.unwrap_or(self.note_clipboard.note);
        let new_note = if forward.unwrap_or(false) && active_note + inc <= 127 {
            active_note + inc
        } else if forward.map(|f| !f).unwrap_or(false) && active_note - inc >= LOWEST_NOTE {
            active_note - inc
        } else {
            active_note
        };

        let instrument_params = self.instrument_params[self.selected_instrument as usize];
        // Also set params and the note as released according to the clipboard if it wasn't previously pressed.
        let (set_release, set_params) = if maybe_selected_note.is_none() {
            (
                Some(self.note_clipboard.release),
                Some((Some(instrument_params.0), Some(instrument_params.1))),
            )
        } else {
            (None, None)
        };
        self.set_pattern_step_events(
            self.selected_step,
            self.selected_song_pattern,
            Some(Some(new_note)),
            set_release,
            set_params,
        );

        (
            new_note,
            p0.unwrap_or(instrument_params.0),
            p1.unwrap_or(instrument_params.1),
        )
    }

    pub fn selected_instrument_params(&self) -> (i8, i8) {
        let instrument = self.selected_instrument as usize;
        self.instrument_params[instrument]
    }

    pub fn cycle_instrument_param(&mut self, param_num: u8, forward: bool) -> i8 {
        let instrument = self.selected_instrument as usize;
        let instrument_params = &mut self.instrument_params[instrument];
        let val = if param_num == 0 {
            &mut instrument_params.0
        } else {
            &mut instrument_params.1
        };

        if forward && *val < 127 {
            *val += 1;
        } else if !forward && *val > -127 {
            *val -= 1;
        }

        let val2 = *val as i32;
        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let instruments_model = GlobalEngine::get(&handle).get_instruments();
                let mut row_data = instruments_model.row_data(instrument).unwrap();
                if param_num == 0 {
                    row_data.param0 = val2;
                } else {
                    row_data.param1 = val2;
                }
                instruments_model.set_row_data(instrument, row_data);
            })
            .unwrap();

        *val
    }

    pub fn cycle_selected_step_param(&mut self, param_num: u8, forward: Option<bool>) -> (u8, i8, i8) {
        let (maybe_selected_note, mut step_parameters) = self.song.patterns[self.selected_pattern_idx()]
            .get_steps(self.selected_instrument)
            .map_or((None, (None, None)), |ss| {
                let s = ss[self.selected_step];
                (s.press_note(), (s.param0(), s.param1()))
            });

        let instrument_params = self.instrument_params[self.selected_instrument as usize];
        let val = if param_num == 0 {
            if step_parameters.0.is_none() {
                step_parameters.0 = Some(instrument_params.0);
            }
            &mut step_parameters.0
        } else {
            if step_parameters.1.is_none() {
                step_parameters.1 = Some(instrument_params.1);
            }
            &mut step_parameters.1
        };
        if forward.unwrap_or(false) && val.unwrap() < 127 {
            *val.as_mut().unwrap() += 1;
        } else if forward.map(|f| !f).unwrap_or(false) && val.unwrap() > -127 {
            *val.as_mut().unwrap() -= 1;
        }

        self.set_pattern_step_events(
            self.selected_step,
            self.selected_song_pattern,
            None,
            None,
            Some(step_parameters),
        );

        (
            maybe_selected_note.unwrap_or(self.note_clipboard.note),
            step_parameters.0.unwrap_or(instrument_params.0),
            step_parameters.1.unwrap_or(instrument_params.1),
        )
    }

    /// Returns all parameters
    pub fn copy_step_params(&mut self, step_num: usize, only_param_num: Option<u8>) -> (Option<i8>, Option<i8>) {
        let step_parameters = self.song.patterns[self.selected_pattern_idx()]
            .get_steps(self.selected_instrument)
            .map_or((None, None), |ss| {
                let s = ss[step_num];
                (s.param0(), s.param1())
            });
        let instrument = self.selected_instrument as usize;
        let instrument_params = &mut self.instrument_params[instrument];
        if only_param_num.map_or(true, |p| p == 0) {
            if let Some(p) = step_parameters.0 {
                // FIXME: This needs to update the instruments param model if I keep it.
                instrument_params.0 = p;
            }
        }
        if only_param_num.map_or(true, |p| p == 1) {
            if let Some(p) = step_parameters.1 {
                instrument_params.1 = p;
            }
        }

        step_parameters
    }

    pub fn cut_step_param(&mut self, step_num: usize, param_num: u8) {
        let mut cut_params = self.copy_step_params(step_num, Some(param_num));
        if param_num == 0 {
            cut_params.0 = None;
        }
        if param_num == 1 {
            cut_params.1 = None;
        }

        self.set_pattern_step_events(step_num, self.active_song_pattern, None, None, Some(cut_params));
    }

    pub fn copy_selected_step_param(&mut self, param_num: u8) {
        self.copy_step_params(self.selected_step, Some(param_num));
    }

    pub fn cut_selected_step_param(&mut self, param_num: u8) {
        self.cut_step_param(self.selected_step, param_num);
    }

    pub fn cycle_song_pattern_start(&mut self) {
        if self.selected_song_pattern == self.song.song_patterns.len() - 1 {
            self.commit_stub_song_pattern();
        }
    }

    pub fn cycle_song_pattern(&mut self, forward: bool) {
        let song_pattern_idx = self.selected_song_pattern;
        let pattern = &mut self.song.song_patterns[song_pattern_idx];
        if forward && *pattern < NUM_PATTERNS - 1 {
            *pattern += 1;
        } else if !forward && *pattern > 0 {
            *pattern -= 1;
        }

        let new_pattern = *pattern as i32;
        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = GlobalEngine::get(&handle).get_sequencer_song_patterns();

                let mut row_data = model.row_data(song_pattern_idx).unwrap();
                row_data.number = new_pattern;
                model.set_row_data(song_pattern_idx, row_data);
            })
            .unwrap();

        self.update_steps();
    }

    fn append_stub_song_pattern(&mut self) {
        // The last song_pattern entry is a stub one to allow editing without an explicit append operation.
        // To prevent having to check everywhere whether the selected song pattern is real or not, append
        // a stub pattern only when the user select that slot, and commit it if any edit of an explicit pattern cycle was made.
        let patterns_to_skip = self.song.song_patterns.iter().max().map_or(0, |m| m + 1);
        self.song.song_patterns.push(
            self.song
                .patterns
                .iter()
                .skip(patterns_to_skip)
                .position(Pattern::is_empty)
                .map_or(0, |p| p + patterns_to_skip),
        );
        self.has_stub_pattern = true;
    }

    fn remove_stub_song_pattern(&mut self) {
        // The UI will still show the stub and we'll append it back if it gets selected again.
        self.song.song_patterns.pop();
        self.has_stub_pattern = false;
        // The song was currently playing the stub and the user moved the selection away so we have to remove it.
        // Ideally we'd delay the removal until the pattern is done playing, but for now just move the playback
        // back into the previous song pattern, which could replay some notes and sound buggy, but still better
        // than cutting up the beginning of the song if we'd move the playback halfway through the first song pattern.
        if self.active_song_pattern == self.selected_song_pattern {
            self.activate_song_pattern(self.song.song_patterns.len() - 1);
        }
    }

    fn commit_stub_song_pattern(&mut self) {
        if !self.has_stub_pattern {
            return;
        }
        let committed_song_pattern = self.song.song_patterns.len() - 1;
        let committed_pattern = self.song.song_patterns[committed_song_pattern] as i32;
        self.has_stub_pattern = false;

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = GlobalEngine::get(&handle).get_sequencer_song_patterns();
                let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();

                // Show the real pattern number for the previous stub
                let mut row_data = vec_model.row_data(committed_song_pattern).unwrap();
                row_data.number = committed_pattern;
                vec_model.set_row_data(committed_song_pattern, row_data);

                // Append a new UI-only stub
                vec_model.push(SongPatternData {
                    number: -1,
                    selected: false,
                });
            })
            .unwrap();
    }

    pub fn remove_last_song_pattern(&mut self) {
        // Eventually we should be able to multi-select, cut and paste multiple song patterns,
        // but for now we only allow appending at the end and removing the last non-stub song pattern,
        // requiring the user to select exactly that one.
        if !self.has_stub_pattern && self.selected_song_pattern + 1 == self.song.song_patterns.len() {
            self.song.song_patterns.remove(self.selected_song_pattern);

            self.main_window
                .upgrade_in_event_loop(move |handle| {
                    let model = GlobalEngine::get(&handle).get_sequencer_song_patterns();
                    let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
                    // The last index is the UI-only stub, so remove the one before.
                    vec_model.remove(vec_model.row_count() - 2);
                })
                .unwrap();

            let next_selection = if !self.song.song_patterns.is_empty() {
                self.song.song_patterns.len() - 1
            } else {
                // select_song_pattern_internal(0) in this case will append a stub pattern.
                0
            };
            if self.active_song_pattern == self.selected_song_pattern {
                self.activate_song_pattern(next_selection);
            }
            self.select_song_pattern_internal(next_selection);
            self.update_steps();
        }
    }

    fn set_song(&mut self, song: SequencerSong) {
        self.song = song;

        let song_patterns = self.song.song_patterns.clone();
        let frames_per_step = self.song.frames_per_step;
        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = GlobalEngine::get(&handle).get_sequencer_song_patterns();
                let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
                for number in song_patterns.iter() {
                    vec_model.push(SongPatternData {
                        number: *number as i32,
                        selected: false,
                    });
                }
                // Append a UI-only stub
                vec_model.push(SongPatternData {
                    number: -1,
                    selected: false,
                });

                let settings = SongSettings {
                    frames_per_step: frames_per_step as i32,
                };
                GlobalSettings::get(&handle).set_song_settings(settings);
            })
            .unwrap();

        self.select_step_internal(0);
        self.select_song_pattern_internal(0);
        self.activate_song_pattern(0);
        self.update_steps();
        self.select_instrument(self.selected_instrument);
    }

    pub fn load_default(&mut self) {
        self.set_song(Default::default());
    }

    #[cfg(feature = "desktop")]
    pub fn load_str(&mut self, markdown: &str) -> Result<String, Box<dyn Error>> {
        let song = parse_markdown_song(markdown)?;

        let instruments_file = song.instruments_file.clone();
        self.set_song(song);
        Ok(instruments_file)
    }

    #[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
    pub fn load_file(&mut self, song_path: &Path) -> Result<String, Box<dyn Error>> {
        if song_path.exists() {
            let md = std::fs::read_to_string(song_path)?;
            let song = parse_markdown_song(&md)?;

            let instruments_file = song.instruments_file.clone();
            self.set_song(song);
            Ok(instruments_file)
        } else {
            Err(format!("Project song file {:?} doesn't exist.", song_path).into())
        }
    }

    #[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
    pub fn save(&self, song_path: &Path) -> Result<(), Box<dyn Error>> {
        save_markdown_song(&self.song, song_path)
    }

    #[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
    pub fn save_as(&mut self, song_path: &Path, instruments_path: &Path) -> Result<(), Box<dyn Error>> {
        self.song.instruments_file = instruments_path
            .file_name()
            .unwrap()
            .to_str()
            .expect("Bad path?")
            .to_owned();
        save_markdown_song(&self.song, song_path)
    }

    #[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
    pub fn serialize_to_postcard(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(to_allocvec(&self.song)?)
    }

    #[cfg(feature = "gba")]
    pub fn load_postcard_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        let song: SequencerSong = from_bytes(bytes).unwrap();
        self.set_song(song);
        Ok(())
    }

    pub fn set_synth_instrument_ids(&mut self, instrument_ids: Vec<SharedString>) {
        for p in &mut self.song.patterns {
            p.update_synth_index(&instrument_ids);
        }
        self.update_steps();

        self.synth_instrument_ids = instrument_ids;
        let ui_copy = self.synth_instrument_ids.clone();
        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = GlobalEngine::get(&handle).get_script_instrument_ids();
                let vec_model = model.as_any().downcast_ref::<VecModel<SharedString>>().unwrap();
                vec_model.set_vec(ui_copy);
            })
            .unwrap();
    }

    fn snap_at_step_frame(&self) -> u32 {
        // Use +1 just to try to compensate for the key press to visual and audible delay.
        self.song.frames_per_step / 2 + 1
    }

    fn num_song_patterns(&self) -> usize {
        let len = self.song.song_patterns.len() as isize;
        // Still count the stub if there are no non-stub song patterns
        (len - (self.has_stub_pattern && len > 1) as isize) as usize
    }
    fn num_song_patterns_including_stub(&self) -> usize {
        (self.song.song_patterns.len() as isize - self.has_stub_pattern as isize + 1) as usize
    }

    fn next_step_and_pattern_and_song_pattern(
        forward: bool,
        from_step: usize,
        from_song_pattern: usize,
        num_song_patterns: usize,
    ) -> (usize, usize) {
        let delta = if forward { 1_isize } else { -1 };
        let next_step = ((from_step as isize + NUM_STEPS as isize + delta) % NUM_STEPS as isize) as usize;
        let wraps = forward && next_step == 0 || !forward && from_step == 0;
        if wraps {
            let next_song_pattern = ((from_song_pattern as isize + num_song_patterns as isize + delta)
                % num_song_patterns as isize) as usize;
            return (next_step, next_song_pattern);
        }
        (next_step, from_song_pattern)
    }
}
