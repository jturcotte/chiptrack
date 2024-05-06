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
use crate::GlobalUI;
use crate::SongPatternData;
use crate::SongSettings;
use core::fmt;
use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::ser::{SerializeStruct, Serializer};

#[cfg(feature = "desktop")]
use markdown::{parse_markdown_song, save_markdown_song};
#[cfg(feature = "gba")]
use postcard::from_bytes;
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct InstrumentStep {
    note: u8,
    release: bool,
    param0: Option<i8>,
    param1: Option<i8>,
}

impl InstrumentStep {
    const FIELDS: &'static [&'static str] = &["note", "flags", "param0", "param1"];

    pub fn is_empty(&self) -> bool {
        self.note == 0 && !self.release && self.param0.is_none() && self.param1.is_none()
    }
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
        // 0 is a valid MIDI note, but the GBA hardware doesn't support that frequency, so use it to represent "no press".
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
        let num_params = self.param0.is_some() as usize + self.param1.is_some() as usize;
        let note = (self.release as u8) << 7 | self.note;
        let flags = (self.param1.is_some() as u8) << 1 | self.param0.is_some() as u8;

        let mut rgb = serializer.serialize_struct("InstrumentStep", 2 + num_params)?;
        rgb.serialize_field(InstrumentStep::FIELDS[0], &note)?;
        rgb.serialize_field(InstrumentStep::FIELDS[1], &flags)?;
        if let Some(val) = self.param0 {
            rgb.serialize_field(InstrumentStep::FIELDS[2], &val)?;
        }
        if let Some(val) = self.param1 {
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
                    param0: None,
                    param1: None,
                };
                if flags & 0b01 != 0 {
                    let val: i8 = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(2, &self))?;
                    i.param0 = Some(val);
                }
                if flags & 0b10 != 0 {
                    let val: i8 = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(2 + flags.count_ones() as usize - 1, &self))?;
                    i.param1 = Some(val);
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
        param0: None,
        param1: Some(8),
    };

    let ser = postcard::to_allocvec(&i)?;
    let r: InstrumentStep = postcard::from_bytes(&ser)?;
    assert!(r == i);
    Ok(())
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
        // Instrument are indexed by UI pages and are sequenced by row,
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

                // Found an instrument ID closer to the sought instrument, keep it
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

    fn remove_instrument(&mut self, instrument: u8) {
        self.instruments.retain(|i| i.synth_index != Some(instrument));
    }

    fn get_steps(&self, instrument: u8) -> Option<&[InstrumentStep; NUM_STEPS]> {
        self.instruments
            .iter()
            .find(|i| i.synth_index == Some(instrument))
            .map(|i| &i.steps)
    }

    fn get_steps_mut(&mut self, instrument: u8) -> Option<&mut [InstrumentStep; NUM_STEPS]> {
        self.instruments
            .iter_mut()
            .find(|i| i.synth_index == Some(instrument))
            .map(|i| &mut i.steps)
    }

    /// This might insert a new instrument if not there already and thus
    /// requires the corresponding string `instrument_id` to be passed.
    fn get_steps_mut_or_insert<'a>(
        &'a mut self,
        instrument_id: &str,
        synth_index: Option<u8>,
    ) -> &'a mut [InstrumentStep; NUM_STEPS] {
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

    /// Returns a copy of the state before the change.
    fn set_step_events(
        &mut self,
        instrument: u8,
        instrument_id: &str,
        step: usize,
        set_press_note: Option<Option<u8>>,
        set_release: Option<bool>,
        set_params: Option<(Option<i8>, Option<i8>)>,
    ) -> InstrumentStep {
        let instrument_steps = self.get_steps_mut_or_insert(instrument_id, Some(instrument));
        let step = &mut instrument_steps[step];
        let previous = *step;

        let mut something_added = false;
        if let Some(release) = set_release {
            something_added |= release;
            step.release = release;
        }
        if let Some(note) = set_press_note {
            something_added |= note.is_some();
            step.set_press_note(note);
        }
        if let Some((param0, param1)) = set_params {
            something_added |= param0.is_some() || param1.is_some();
            step.param0 = param0;
            step.param1 = param1;
        }

        if !something_added && instrument_steps.iter().all(|s| s.is_empty()) {
            self.remove_instrument(instrument);
        }

        previous
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

enum SelectionClipboard {
    Empty,
    InstrumentParams(Vec<Option<i8>>),
    WholeSteps(Vec<InstrumentStep>),
}

#[derive(PartialEq)]
pub enum OnEmpty {
    PasteOnEmpty,
    EmptyOnEmpty,
}

pub struct Sequencer {
    pub song: SequencerSong,
    active_frame: Option<u32>,
    active_step: usize,
    active_song_pattern: usize,
    pub displayed_instrument: u8,
    displayed_song_pattern: usize,
    playing: bool,
    play_song_mode: bool,
    recording: bool,
    erasing: bool,
    has_stub_pattern: bool,
    next_cycle_song_pattern_start_can_have_new: bool,
    pub received_instruments_ids_after_load: bool,
    last_press_frame: Option<u32>,
    just_recorded_over_next_step: bool,
    // FIXME: Use a bitset
    muted_instruments: BTreeSet<u8>,
    synth_instrument_ids: Vec<SharedString>,
    instrument_params: Vec<(i8, i8)>,
    /// The note that will be used when an empty step is press-toggled.
    default_note_clipboard: NoteClipboard,
    /// The pattern that will be used when a new song pattern slot is added.
    default_song_pattern_clipboard: usize,
    selection_clipboard: SelectionClipboard,
    main_window: WeakWindowWrapper,
}

impl Sequencer {
    pub fn new(main_window: WeakWindowWrapper) -> Sequencer {
        Sequencer {
            song: Default::default(),
            active_frame: None,
            active_step: 0,
            active_song_pattern: 0,
            displayed_instrument: 0,
            displayed_song_pattern: 0,
            playing: false,
            play_song_mode: false,
            recording: true,
            erasing: false,
            has_stub_pattern: false,
            next_cycle_song_pattern_start_can_have_new: false,
            received_instruments_ids_after_load: false,
            last_press_frame: None,
            just_recorded_over_next_step: false,
            muted_instruments: BTreeSet::new(),
            synth_instrument_ids: vec![SharedString::new(); NUM_INSTRUMENTS],
            instrument_params: vec![(0, 0); NUM_INSTRUMENTS],
            default_note_clipboard: NoteClipboard {
                note: DEFAULT_NOTE,
                release: true,
            },
            default_song_pattern_clipboard: 0,
            main_window: main_window.clone(),
            selection_clipboard: SelectionClipboard::Empty,
        }
    }

    fn active_frame_or_zero(&self) -> u32 {
        self.active_frame.unwrap_or(0)
    }

    // Pattern number in a given `song_pattern_idx`.
    fn pattern_idx(&self, song_pattern_idx: usize) -> usize {
        self.song.song_patterns[song_pattern_idx]
    }

    /// The pattern number of the active song pattern.
    fn active_pattern_idx(&self) -> usize {
        self.pattern_idx(self.active_song_pattern)
    }

    /// The pattern number of the displayed song pattern.
    fn displayed_pattern_idx(&self) -> usize {
        self.pattern_idx(self.displayed_song_pattern)
    }

    pub fn activate_song_pattern(&mut self, song_pattern: usize) {
        self.active_song_pattern = song_pattern;

        #[cfg(not(feature = "gba"))]
        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let engine = GlobalEngine::get(&handle);
                let ui = GlobalUI::get(&handle);

                engine.set_sequencer_song_pattern_active(song_pattern as i32);
                if ui.get_playing() && ui.get_pin_selection_to_active() {
                    engine.invoke_display_song_pattern(song_pattern as i32);
                }
            })
            .unwrap();

        // There is currently no event loop on the GBA and the UI lives on a separate thread
        // on the desktop. So the implementation needs to be separate to avoid the reentrancy
        // caused by the UI callback invoking the GlobalEngine, requiring a RefCell that
        // is already held.
        #[cfg(feature = "gba")]
        {
            let should_display = self.main_window.run_direct(move |handle| {
                GlobalEngine::get(&handle).set_sequencer_song_pattern_active(song_pattern as i32);
                let ui = GlobalUI::get(&handle);
                ui.get_playing() && ui.get_pin_selection_to_active()
            });
            if should_display {
                self.display_song_pattern(song_pattern);
            }
        }
    }

    pub fn activate_step(&mut self, step: usize) {
        self.active_step = step;

        let active_step = if self.active_song_pattern == self.displayed_song_pattern {
            step as i32
        } else {
            -1
        };

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                GlobalEngine::get(&handle).set_sequencer_step_active(active_step);
                GlobalUI::get(&handle).invoke_update_selected_step();
            })
            .unwrap();
    }

    pub fn apply_song_settings(&mut self, settings: &SongSettings) {
        self.song.frames_per_step = settings.frames_per_step as u32;
    }

    pub fn display_song_pattern(&mut self, song_pattern: usize) {
        if !self.has_stub_pattern && song_pattern == self.song.song_patterns.len() {
            // Only append the stub while selecting it so that it doesn't affect playback unless selected
            // and also to avoid allowing the user to change patterns before the selection, which could
            // prevent us from picking the best match for the next pattern index.
            self.append_stub_song_pattern();
        } else if self.has_stub_pattern && song_pattern != self.displayed_song_pattern {
            self.remove_stub_song_pattern();
        }
        let prev_selected = self.displayed_song_pattern;
        self.displayed_song_pattern = song_pattern;

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

        self.update_steps();
    }

    pub fn display_instrument(&mut self, instrument: u8) {
        self.displayed_instrument = instrument;

        self.update_steps();

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                GlobalEngine::get(&handle).set_displayed_instrument(instrument as i32);
            })
            .unwrap();
    }

    pub fn cycle_instrument(&mut self, col_delta: i32, row_delta: i32) {
        // Wrap
        let col = (self.displayed_instrument as i32 + 4 + col_delta) % 4;
        // Don't wrap
        let row = (self.displayed_instrument as i32 / 4 + row_delta).max(0).min(15);
        self.display_instrument((col + row * 4) as u8)
    }

    pub fn cycle_pattern_instrument(&mut self, forward: bool) {
        let maybe_next =
            self.song.patterns[self.displayed_pattern_idx()].next_instrument(self.displayed_instrument, forward);
        if let Some(instrument) = maybe_next {
            self.display_instrument(instrument)
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
        let pattern = &self.song.patterns[self.displayed_pattern_idx()];
        let maybe_steps = pattern.get_steps(self.displayed_instrument).copied();

        let active_step = if self.active_song_pattern == self.displayed_song_pattern {
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
            match pattern.find_nearest_instrument_pos(self.displayed_instrument, true) {
                // Advance once more to not show the found instrument both in the patterns and pattern_instruments models
                Some((ii, i)) if i == self.displayed_instrument => (ii + 1, instruments.len() - 1),
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
                    match step.param0 {
                        Some(v) => {
                            row_data.param0_set = true;
                            row_data.param0_val = v as i32;
                        }
                        None => row_data.param0_set = false,
                    }
                    match step.param1 {
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
                        instrument_data.synth_index = mi.synth_index.map_or(-1, |i| i as i32);
                        model2.set_row_data(idx, instrument_data);
                    });
            })
            .unwrap();
    }

    pub fn toggle_step(&mut self, step: usize) {
        let maybe_steps = self.song.patterns[self.displayed_pattern_idx()].get_steps(self.displayed_instrument);
        let pressed = maybe_steps.map_or(false, |ss| ss[step].press());
        if pressed {
            self.cut_step_range_note(step, step);
        } else {
            self.cycle_step_note(step, None, false, OnEmpty::PasteOnEmpty);
        }
    }

    pub fn set_default_step_note(&mut self, step: usize) {
        let maybe_steps = self.song.patterns[self.displayed_pattern_idx()].get_steps(self.displayed_instrument);
        let pressed_note_and_release = maybe_steps.and_then(|ss| ss[step].press_note().map(|p| (p, ss[step].release)));
        if let Some((note, release)) = pressed_note_and_release {
            self.default_note_clipboard = NoteClipboard { note, release };
        }
    }

    pub fn copy_step_range_note(&mut self, step_range_first: usize, step_range_last: usize) {
        let maybe_steps = self.song.patterns[self.displayed_pattern_idx()].get_steps(self.displayed_instrument);

        self.selection_clipboard = SelectionClipboard::WholeSteps(maybe_steps.map_or_else(
            || vec![InstrumentStep::default(); step_range_last - step_range_first + 1],
            |ss| ss[step_range_first..=step_range_last].to_vec(),
        ));
    }

    pub fn cut_step_range_note(&mut self, step_range_first: usize, step_range_last: usize) {
        let displayed_pattern_idx = self.displayed_pattern_idx();
        let maybe_steps = self.song.patterns[displayed_pattern_idx].get_steps_mut(self.displayed_instrument);
        let steps = maybe_steps.map_or_else(
            || vec![InstrumentStep::default(); step_range_last - step_range_first + 1],
            |ss| {
                let slice = &mut ss[step_range_first..=step_range_last];
                // Take a copy
                let r = slice.to_vec();
                // Replace the cut steps with an empty step
                slice.fill(InstrumentStep::default());
                r
            },
        );

        self.selection_clipboard = SelectionClipboard::WholeSteps(steps);
        self.update_steps();
    }

    /// Cut while not in selection mode, it sets both the edit and selection clipboards.
    pub fn cut_step_single_note(&mut self, step: usize) {
        self.set_default_step_note(step);
        self.set_default_step_params(step, None);

        let cut_step = self.set_pattern_step_events(
            step,
            self.displayed_song_pattern,
            Some(None),
            Some(false),
            Some((None, None)),
        );

        if !cut_step.is_empty() {
            self.selection_clipboard = SelectionClipboard::WholeSteps(vec![cut_step]);
        }
    }

    pub fn paste_step_range_note(&mut self, at_step: usize) {
        let instrument_id = &self.synth_instrument_ids[self.displayed_instrument as usize];
        let displayed_pattern_idx = self.displayed_pattern_idx();
        let steps = self.song.patterns[displayed_pattern_idx]
            .get_steps_mut_or_insert(instrument_id, Some(self.displayed_instrument));

        if let SelectionClipboard::WholeSteps(clip_steps) = &self.selection_clipboard {
            for (i, step) in clip_steps.iter().enumerate() {
                steps[(at_step + i) % NUM_STEPS] = *step;
            }
        }

        self.update_steps();
    }

    pub fn toggle_step_release(&mut self, step: usize) {
        let maybe_steps = self.song.patterns[self.active_pattern_idx()].get_steps(self.displayed_instrument);
        let toggled = !maybe_steps.map_or(false, |ss| ss[step].release);
        self.set_pattern_step_events(step, self.displayed_song_pattern, None, Some(toggled), None);

        self.set_default_step_note(step);
    }

    fn advance_step(&mut self) {
        let next_step = if self.play_song_mode {
            let (next_step, next_song_pattern) = Self::next_step_and_pattern_and_song_pattern(
                true,
                self.active_step,
                self.active_song_pattern,
                self.num_song_patterns(),
            );

            if next_song_pattern != self.active_song_pattern {
                self.activate_song_pattern(next_song_pattern);
            }
            next_step
        } else {
            // In pattern playback, continue playing from the displayed pattern if it changed.
            if self.active_step == NUM_STEPS - 1 && self.displayed_song_pattern != self.active_song_pattern {
                self.activate_song_pattern(self.displayed_song_pattern);
                0
            } else {
                (self.active_step + 1) % NUM_STEPS
            }
        };

        self.activate_step(next_step);
    }

    fn set_pattern_step_events(
        &mut self,
        step: usize,
        song_pattern: usize,
        set_press_note: Option<Option<u8>>,
        set_release: Option<bool>,
        set_params: Option<(Option<i8>, Option<i8>)>,
    ) -> InstrumentStep {
        // If editing the stub pattern, commit it to a real pattern now.
        if self.has_stub_pattern {
            self.commit_stub_song_pattern();
        }

        let pattern = self.pattern_idx(song_pattern);
        let instrument_id = &self.synth_instrument_ids[self.displayed_instrument as usize];
        let previous = self.song.patterns[pattern].set_step_events(
            self.displayed_instrument,
            instrument_id,
            step,
            set_press_note,
            set_release,
            set_params,
        );

        if song_pattern == self.displayed_song_pattern {
            self.main_window
                .upgrade_in_event_loop(move |handle| {
                    let steps = GlobalEngine::get(&handle).get_sequencer_steps();
                    let mut step_row_data = steps.row_data(step).unwrap();
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
                    steps.set_row_data(step, step_row_data);
                })
                .unwrap();
        }

        previous
    }
    pub fn playing(&self) -> bool {
        self.playing
    }

    pub fn set_playing(&mut self, val: bool, song_mode: bool) {
        self.playing = val;
        self.play_song_mode = song_mode;
        // Reset the active_frame so that it's aligned with full
        // steps and that record_press would record any key while
        // stopped to the current frame and not the next.
        self.active_frame = None;
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
            if self.just_recorded_over_next_step && i == self.displayed_instrument {
                self.just_recorded_over_next_step = false;
                continue;
            }

            if let Some(step) = self.song.patterns[self.active_pattern_idx()]
                .get_steps(i)
                .map(|ss| ss[self.active_step])
            {
                if let Some(note) = step.press_note() {
                    // Use the default if the sequencer didn't have any param set for that press.
                    let p0 = step.param0.unwrap_or(DEFAULT_PARAM_VAL);
                    let p1 = step.param1.unwrap_or(DEFAULT_PARAM_VAL);
                    log!(
                        "➕ PRS {} note {} params {} / {}",
                        self.synth_instrument_ids[i as usize],
                        MidiNote(note as i32).name(),
                        p0,
                        p1
                    );
                    note_events.push((i, StepEvent::Press(step.note, p0, p1)));
                } else {
                    if let Some(val) = step.param0 {
                        log!("✖️ PAR {} param {} = {}", self.synth_instrument_ids[i as usize], 0, val);
                        note_events.push((i, StepEvent::SetParam(0, val)));
                    }
                    if let Some(val) = step.param1 {
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
            if self.just_recorded_over_next_step && i == self.displayed_instrument {
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

        match self.active_frame {
            None => {
                self.active_frame = Some(1);

                // The first advance_frame after playing will move from frame 0 to frame 1 and skip presses
                // of frame 0. Since we don't care about releases of the non-existent previous frame, do the
                // presses now, right after starting the playback.
                let mut note_events: Vec<(u8, StepEvent)> = Vec::new();
                self.handle_active_step_presses_and_params(&mut note_events);

                (Some(self.active_step as u32), note_events)
            }
            Some(frame) if frame % self.song.frames_per_step == 0 => {
                // FIXME: Reset or remove overflow check
                self.active_frame = Some(frame + 1);
                // Release are at then end of a step, so start by triggering any release of the
                // previous frame.
                self.handle_active_step_releases(&mut note_events);

                self.advance_step();
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
            }
            Some(frame) => {
                self.active_frame = Some(frame + 1);
                (None, note_events)
            }
        }
    }

    pub fn clipboard_note(&self) -> u8 {
        self.default_note_clipboard.note
    }

    fn record_key_event(&mut self, event: KeyEvent, note: Option<u8>, params: Option<(Option<i8>, Option<i8>)>) {
        if !self.recording {
            return;
        }

        let (press_note, release, (step, song_pattern)) = match event {
            KeyEvent::Press if !self.playing => {
                let pressed = self.song.patterns[self.active_pattern_idx()]
                    .get_steps(self.displayed_instrument)
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
                    if self.active_frame_or_zero() % self.song.frames_per_step < self.snap_at_step_frame() {
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
                    self.active_frame_or_zero() - self.last_press_frame.unwrap(),
                    self.song.frames_per_step,
                );
                // We need to place the release in the previous step (its end), so subtract one step.
                let rounded_end_frame = self.last_press_frame.unwrap() + (rounded_steps_note_length.max(1) - 1);

                let is_end_in_prev_step = rounded_end_frame / self.song.frames_per_step
                    < self.active_frame_or_zero() / self.song.frames_per_step;
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
        let (p0, p1) = self.displayed_instrument_params();
        self.record_key_event(
            KeyEvent::Press,
            Some(note),
            Some((Some(p0).filter(|v| *v != 0), Some(p1).filter(|v| *v != 0))),
        );
        self.last_press_frame = Some(self.active_frame_or_zero());

        // It's a bit weird to keep the release part of the clipboard here,
        // but the fact that recording affects the active step instead of selected one
        // makes it difficult to find the right compromise.
        self.default_note_clipboard.note = note;
        (p0, p1)
    }

    pub fn record_release(&mut self, _note: u8) {
        // The note release won't be passed to the synth on playback,
        // so don't overwrite the note in the step just in case it contained something useful.
        self.record_key_event(KeyEvent::Release, None, None);
    }

    pub fn cycle_step_note(
        &mut self,
        step: usize,
        forward: Option<bool>,
        large_inc: bool,
        on_empty: OnEmpty,
    ) -> (u8, i8, i8) {
        // The GBA only handles frequencies from C1 upwards.
        const LOWEST_NOTE: u8 = 24;

        let (maybe_selected_note, p0, p1) = self.song.patterns[self.displayed_pattern_idx()]
            .get_steps(self.displayed_instrument)
            .map_or((None, None, None), |ss| {
                let s = ss[step];
                (s.press_note(), s.param0, s.param1)
            });
        if maybe_selected_note.is_none() && on_empty == OnEmpty::EmptyOnEmpty {
            return (0, 0, 0);
        }
        let inc = if large_inc { 12 } else { 1 };
        let active_note = maybe_selected_note.unwrap_or(self.default_note_clipboard.note);
        let new_note = if forward.unwrap_or(false) && active_note + inc <= 127 {
            active_note + inc
        } else if forward.map(|f| !f).unwrap_or(false) && active_note - inc >= LOWEST_NOTE {
            active_note - inc
        } else {
            active_note
        };

        let instrument_params = self.instrument_params[self.displayed_instrument as usize];
        // Also set params and the note as released according to the clipboard if it wasn't previously pressed.
        let (set_release, set_params) = if maybe_selected_note.is_none() {
            (
                Some(self.default_note_clipboard.release),
                Some((Some(instrument_params.0), Some(instrument_params.1))),
            )
        } else {
            (None, None)
        };
        self.set_pattern_step_events(
            step,
            self.displayed_song_pattern,
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

    pub fn displayed_instrument_params(&self) -> (i8, i8) {
        let instrument = self.displayed_instrument as usize;
        self.instrument_params[instrument]
    }

    pub fn cycle_instrument_param(&mut self, param_num: u8, forward: bool) -> (i8, i8) {
        let instrument = self.displayed_instrument as usize;
        let instrument_params = &mut self.instrument_params[instrument];
        let val = if param_num == 0 {
            &mut instrument_params.0
        } else {
            &mut instrument_params.1
        };

        if forward && *val < 127 {
            *val += 1;
        } else if !forward && *val > -128 {
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

        *instrument_params
    }

    pub fn cycle_step_param(
        &mut self,
        step: usize,
        param_num: u8,
        forward: Option<bool>,
        large_inc: bool,
        on_empty: OnEmpty,
    ) -> (u8, i8, i8) {
        let (maybe_selected_note, mut step_parameters) = self.song.patterns[self.displayed_pattern_idx()]
            .get_steps(self.displayed_instrument)
            .map_or((None, (None, None)), |ss| {
                let s = ss[step];
                (s.press_note(), (s.param0, s.param1))
            });

        let instrument_params = self.instrument_params[self.displayed_instrument as usize];
        let val = if param_num == 0 {
            if step_parameters.0.is_none() {
                if on_empty == OnEmpty::EmptyOnEmpty {
                    return (0, 0, 0);
                }
                step_parameters.0 = Some(instrument_params.0);
            }
            &mut step_parameters.0
        } else {
            if step_parameters.1.is_none() {
                if on_empty == OnEmpty::EmptyOnEmpty {
                    return (0, 0, 0);
                }
                step_parameters.1 = Some(instrument_params.1);
            }
            &mut step_parameters.1
        };
        if large_inc {
            let inc = if forward.unwrap_or(false) { 0x10 } else { -0x10 };
            let v = val.as_mut().unwrap();
            // Do it with wrapping here in case the instrument wants to split parameters
            // in two and have the 4 most significant bits do something else.
            // The signed integer however gets in the way of this, but it's still nice
            // to have it for the full value, so wrap the 0x10 increments but cap the 0x01 ones.
            *v = v.wrapping_add(inc);
        } else if forward.unwrap_or(false) && val.unwrap() < 127 {
            *val.as_mut().unwrap() += 0x01;
        } else if forward.map(|f| !f).unwrap_or(false) && val.unwrap() > -128 {
            *val.as_mut().unwrap() -= 0x01;
        }

        self.set_pattern_step_events(step, self.displayed_song_pattern, None, None, Some(step_parameters));

        (
            maybe_selected_note.unwrap_or(self.default_note_clipboard.note),
            step_parameters.0.unwrap_or(instrument_params.0),
            step_parameters.1.unwrap_or(instrument_params.1),
        )
    }

    /// Default parameters are set for each instrument and also act as a clipboard for new note presses.
    /// Returns all parameters
    pub fn set_default_step_params(&mut self, step: usize, only_param_num: Option<u8>) -> (Option<i8>, Option<i8>) {
        let step_parameters = self.song.patterns[self.displayed_pattern_idx()]
            .get_steps(self.displayed_instrument)
            .map_or((None, None), |ss| {
                let s = ss[step];
                (s.param0, s.param1)
            });
        let instrument = self.displayed_instrument as usize;
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

    pub fn copy_step_range_param(&mut self, step_range_first: usize, step_range_last: usize, param_num: u8) {
        let maybe_steps = self.song.patterns[self.displayed_pattern_idx()].get_steps(self.displayed_instrument);

        let get_param = if param_num == 0 {
            |s: &InstrumentStep| s.param0
        } else {
            |s: &InstrumentStep| s.param1
        };
        self.selection_clipboard = SelectionClipboard::InstrumentParams(maybe_steps.map_or_else(
            || vec![None; step_range_last - step_range_first + 1],
            |ss| ss[step_range_first..=step_range_last].iter().map(get_param).collect(),
        ));
    }

    pub fn cut_step_range_param(&mut self, step_range_first: usize, step_range_last: usize, param_num: u8) {
        let displayed_pattern_idx = self.displayed_pattern_idx();
        let maybe_steps = self.song.patterns[displayed_pattern_idx].get_steps_mut(self.displayed_instrument);
        let get_param = if param_num == 0 {
            |s: &InstrumentStep| s.param0
        } else {
            |s: &InstrumentStep| s.param1
        };
        let clear_param = if param_num == 0 {
            |s: &mut InstrumentStep| s.param0 = None
        } else {
            |s: &mut InstrumentStep| s.param1 = None
        };

        let values = maybe_steps.map_or_else(
            || vec![None; step_range_last - step_range_first + 1],
            |ss| {
                let slice = &mut ss[step_range_first..=step_range_last];
                // Take a copy
                let r = slice.iter().map(get_param).collect();
                // Empty the cut params
                slice.iter_mut().for_each(clear_param);
                r
            },
        );

        self.selection_clipboard = SelectionClipboard::InstrumentParams(values);
        self.update_steps();
    }

    /// Cut while not in selection mode, it sets both the edit and selection clipboards.
    pub fn cut_step_single_param(&mut self, step: usize, param_num: u8) {
        let mut cut_params = self.set_default_step_params(step, Some(param_num));
        if param_num == 0 && cut_params.0.is_some() {
            self.selection_clipboard = SelectionClipboard::InstrumentParams(vec![cut_params.0.take()]);
        } else if param_num == 1 && cut_params.1.is_some() {
            self.selection_clipboard = SelectionClipboard::InstrumentParams(vec![cut_params.1.take()]);
        }

        self.set_pattern_step_events(step, self.active_song_pattern, None, None, Some(cut_params));
    }

    pub fn paste_step_range_param(&mut self, at_step: usize, param_num: u8) {
        let instrument_id = &self.synth_instrument_ids[self.displayed_instrument as usize];
        let displayed_pattern_idx = self.displayed_pattern_idx();
        let steps = self.song.patterns[displayed_pattern_idx]
            .get_steps_mut_or_insert(instrument_id, Some(self.displayed_instrument));

        let set_param = if param_num == 0 {
            |s: &mut InstrumentStep, v: Option<i8>| s.param0 = v
        } else {
            |s: &mut InstrumentStep, v: Option<i8>| s.param1 = v
        };

        if let SelectionClipboard::InstrumentParams(clip_params) = &self.selection_clipboard {
            for (i, param) in clip_params.iter().enumerate() {
                set_param(&mut steps[(at_step + i) % NUM_STEPS], *param);
            }
        }

        self.update_steps();
    }

    /// Return the number of the first pattern that is not referenced by the song and that is still empty,
    /// or None if there is no empty pattern left.
    pub fn find_first_unused_pattern_idx(&self) -> Option<usize> {
        let pattern_after_max_song_reference = self.song.song_patterns.iter().max().map_or(0, |m| m + 1);
        self.song
            .patterns
            .iter()
            .skip(pattern_after_max_song_reference)
            .position(Pattern::is_empty)
            // Re-add the skipped amount
            .map(|p| p + pattern_after_max_song_reference)
    }

    pub fn cycle_song_pattern_start_with_new(&mut self) {
        if self.next_cycle_song_pattern_start_can_have_new {
            if let Some(sp) = self.find_first_unused_pattern_idx() {
                self.write_displayed_song_pattern(sp);
            } else {
                elog!("Max pattern reached");
            }
        }
        self.cycle_song_pattern_start()
    }

    pub fn cycle_song_pattern_start(&mut self) {
        // Double-press to use new pattern is only allowed after an insert,
        // so make sure to reset this state to prevent accidental new pattern writes.
        self.next_cycle_song_pattern_start_can_have_new = false;

        // Check if the stub inserted during selection needs to be committed.
        if self.has_stub_pattern {
            // This is the
            self.song.song_patterns[self.displayed_song_pattern] = self.default_song_pattern_clipboard;
            self.commit_stub_song_pattern();
            self.update_steps();
        }

        // Update the clipboard, whether it's after a touch or appending a new song pattern.
        self.default_song_pattern_clipboard = self.displayed_pattern_idx();
    }

    fn write_displayed_song_pattern(&mut self, new_pattern: usize) {
        let song_pattern_idx = self.displayed_song_pattern;
        self.song.song_patterns[song_pattern_idx] = new_pattern;

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = GlobalEngine::get(&handle).get_sequencer_song_patterns();

                let mut row_data = model.row_data(song_pattern_idx).unwrap();
                row_data.number = new_pattern as i32;
                model.set_row_data(song_pattern_idx, row_data);
            })
            .unwrap();

        self.update_steps();
    }

    pub fn cycle_song_pattern(&mut self, forward: bool) {
        let mut pattern = self.displayed_pattern_idx();
        if forward && pattern < NUM_PATTERNS - 1 {
            pattern += 1;
        } else if !forward && pattern > 0 {
            pattern -= 1;
        }

        // Write the cycled value to the clipboard.
        self.default_song_pattern_clipboard = pattern;

        self.write_displayed_song_pattern(pattern);
    }

    fn append_stub_song_pattern(&mut self) {
        // The last song_pattern entry is a stub one to allow editing without an explicit append operation.
        // To prevent having to check everywhere whether the displayed song pattern is real or not, append
        // a stub pattern only when the user select that slot, and commit it if any edit of an explicit pattern cycle was made.
        // Start with an empty pattern so that the user can move back to the pattern panel and edit it from scratch.
        self.song.song_patterns.push(
            self.find_first_unused_pattern_idx()
                .unwrap_or(self.default_song_pattern_clipboard),
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
        if self.active_song_pattern == self.displayed_song_pattern {
            self.activate_song_pattern(self.song.song_patterns.len() - 1);
        }
    }

    fn commit_stub_song_pattern(&mut self) {
        assert!(self.has_stub_pattern);
        let committed_song_pattern = self.song.song_patterns.len() - 1;
        let committed_pattern = self.song.song_patterns[committed_song_pattern] as i32;
        self.has_stub_pattern = false;

        // After the user committed the stub song pattern, allow replacing the automatically
        // inserted clipboard value with the next empty pattern number on a second press.
        self.next_cycle_song_pattern_start_can_have_new = true;

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
        if !self.has_stub_pattern && self.displayed_song_pattern + 1 == self.song.song_patterns.len() {
            let removed = self.song.song_patterns.remove(self.displayed_song_pattern);

            // Make sure that doing Z,X,X doesn't allow picking a new pattern after the removing one
            // since it ends with X,X and the flag might still be set.
            self.next_cycle_song_pattern_start_can_have_new = false;

            // Cut the removed value into the clipboard.
            self.default_song_pattern_clipboard = removed;

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
                // select_song_pattern(0) in this case will append a stub pattern.
                0
            };
            if self.active_song_pattern == self.displayed_song_pattern {
                self.activate_song_pattern(next_selection);
            }
            self.display_song_pattern(next_selection);
        }
    }

    pub fn clone_displayed_song_pattern(&mut self) {
        if let Some(new_pattern_idx) = self.find_first_unused_pattern_idx() {
            let displayed_pattern = self.displayed_pattern_idx();
            self.song.patterns[new_pattern_idx] = self.song.patterns[displayed_pattern].clone();
            self.write_displayed_song_pattern(new_pattern_idx);
        } else {
            elog!("Max pattern reached");
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

        self.activate_song_pattern(0);
        self.display_song_pattern(0);
        self.display_instrument(self.displayed_instrument);
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

    #[cfg(not(target_arch = "wasm32"))]
    pub fn serialize_to_postcard(&self) -> Result<alloc::vec::Vec<u8>, postcard::Error> {
        to_allocvec(&self.song)
    }

    #[cfg(feature = "gba")]
    pub fn load_postcard_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        let song: SequencerSong = from_bytes(bytes).unwrap();
        self.set_song(song);
        Ok(())
    }

    pub fn set_synth_instrument_ids(&mut self, instrument_ids: Vec<SharedString>) {
        // Playback can start
        self.received_instruments_ids_after_load = true;

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
