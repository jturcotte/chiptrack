// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

#[cfg(feature = "desktop")]
mod markdown;

use crate::sound_engine::NUM_INSTRUMENTS;
use crate::sound_engine::NUM_PATTERNS;
use crate::sound_engine::NUM_STEPS;
use crate::utils::MidiNote;
use crate::GlobalEngine;
use crate::GlobalSettings;
use crate::PatternInstrumentData;
use crate::SongPatternData;
use crate::SongSettings;
use crate::utils::WeakWindowWrapper;
#[cfg(feature = "desktop")]
use markdown::{parse_markdown_song, save_markdown_song};

use serde::Serialize;
use serde::Deserialize;
#[cfg(feature = "gba")]
use postcard::from_bytes;
#[cfg(feature = "desktop")]
use postcard::to_allocvec;
use slint::Global;
use slint::Model;
use slint::VecModel;

use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
#[cfg(feature = "desktop")]
use std::error::Error;
#[cfg(feature = "desktop")]
use std::path::Path;

#[cfg(target_arch = "wasm32")]
use crate::utils;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum NoteEvent {
    Press,
    Release,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
struct InstrumentStep {
    note: u8,
    press: bool,
    release: bool,
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

    fn get_steps<'a>(&'a self, instrument: u8) -> Option<&'a [InstrumentStep; NUM_STEPS]> {
        self.instruments
            .iter()
            .find(|i| i.synth_index == Some(instrument))
            .map(|i| &i.steps)
    }

    fn next_instrument(&self, current_instrument: u8, forwards: bool) -> Option<u8> {
        let maybe_current_position = self.instruments
            .iter()
            .position(|i| i.synth_index == Some(current_instrument));
        match maybe_current_position {
            Some(ii) if forwards => self.instruments.iter().cycle().skip(ii + 1).take(self.instruments.len() - 1).find_map(|i| i.synth_index),
            Some(ii) => self.instruments.iter().rev().cycle().skip(self.instruments.len() - ii).take(self.instruments.len() - 1).find_map(|i| i.synth_index),
            // The selected instrument isn't yet in the pattern, use the first available instrument.
            None => self.instruments.iter().find_map(|i| i.synth_index),
        }
    }

    fn find_instrument_pos(&self, instrument: u8) -> Option<usize> {
        self.instruments
            .iter()
            .position(|i| i.synth_index == Some(instrument))
    }

    fn instruments(&self) -> &Vec<Instrument> {
        &self.instruments
    }

    fn get_steps_mut<'a>(
        &'a mut self,
        instrument_id: &str,
        synth_index: Option<u8>,
    ) -> &'a mut [InstrumentStep; NUM_STEPS] {
        let ii = match self.instruments.iter().position(|i| i.id == instrument_id) {
            Some(ii) => ii,
            None => {
                self.instruments.push(Instrument {
                    id: instrument_id.to_owned(),
                    synth_index: synth_index,
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
        set_press: Option<bool>,
        set_release: Option<bool>,
        set_note: Option<u8>,
    ) -> bool {
        let mut step = &mut self.get_steps_mut(instrument_id, Some(instrument))[step_num];
        if set_press.map_or(true, |v| v == step.press)
            && set_release.map_or(true, |v| v == step.release)
            && set_note.map_or(true, |v| v == step.note)
        {
            return false;
        }

        if let Some(press) = set_press {
            step.press = press;
        }
        if let Some(release) = set_release {
            step.release = release;
        }
        if let Some(note) = set_note {
            step.note = note;
        }

        let pattern_empty = if set_press.unwrap_or(false) || set_release.unwrap_or(false) {
            false
        } else {
            // FIXME: Remove empty instruments instead and get the caller to use is_empty()
            self.instruments
                .iter()
                .all(|i| i.steps.iter().all(|step| !step.press && !step.release))
        };
        pattern_empty
    }

    fn update_synth_index(&mut self, new_instrument_ids: &Vec<String>) {
        let mut instrument_by_id: BTreeMap<String, Instrument> =
            self.instruments.drain(..).map(|i| (i.id.clone(), i)).collect();
        self.instruments = new_instrument_ids
            .iter()
            .enumerate()
            .flat_map(|(synth_index, id)| {
                let mut maybe_i = instrument_by_id.remove(id);
                match maybe_i.as_mut() {
                    Some(i) => i.synth_index = Some(synth_index as u8),
                    None => (),
                }
                maybe_i
            })
            .collect();

        // Append any remaining sequencer instruments not available in the synth instruments
        // until the synth is maybe updated.
        self.instruments.extend(instrument_by_id.into_values().map(|mut i| {
            i.synth_index = None;
            i
        }))
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

impl Default for InstrumentStep {
    fn default() -> Self {
        // Initialize all notes to C5
        InstrumentStep {
            note: 60,
            press: false,
            release: false,
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

pub struct Sequencer {
    pub song: SequencerSong,
    current_frame: u32,
    current_step: usize,
    current_song_pattern: Option<usize>,
    selected_pattern: usize,
    pub selected_instrument: u8,
    playing: bool,
    recording: bool,
    erasing: bool,
    last_press_frame: Option<u32>,
    just_recorded_over_next_step: bool,
    // FIXME: Use a bitset
    muted_instruments: BTreeSet<u8>,
    synth_instrument_ids: Vec<String>,
    main_window: WeakWindowWrapper,
}

impl Sequencer {
    pub fn new(main_window: WeakWindowWrapper) -> Sequencer {
        Sequencer {
            song: Default::default(),
            current_frame: 0,
            current_step: 0,
            current_song_pattern: None,
            selected_pattern: 0,
            selected_instrument: 0,
            playing: false,
            recording: true,
            erasing: false,
            last_press_frame: None,
            just_recorded_over_next_step: false,
            muted_instruments: BTreeSet::new(),
            synth_instrument_ids: vec![String::new(); NUM_INSTRUMENTS],
            main_window: main_window.clone(),
        }
    }

    pub fn apply_song_settings(&mut self, settings: &SongSettings) {
        self.song.frames_per_step = settings.frames_per_step as u32;
    }

    pub fn select_song_pattern(&mut self, song_pattern: Option<u32>) -> () {
        let old = self.current_song_pattern;
        self.current_song_pattern = song_pattern.map(|sp| sp as usize);
        let new = self.current_song_pattern;

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = GlobalEngine::get(&handle).get_sequencer_song_patterns();
                if let Some(current) = old {
                    let mut pattern_row_data = model.row_data(current).unwrap();
                    pattern_row_data.active = false;
                    model.set_row_data(current, pattern_row_data);
                }
                if let Some(current) = new {
                    let mut pattern_row_data = model.row_data(current).unwrap();
                    pattern_row_data.active = true;
                    model.set_row_data(current, pattern_row_data);
                }
            })
            .unwrap();
    }

    pub fn select_pattern(&mut self, pattern: u32) -> () {
        let old = self.selected_pattern;
        // FIXME: Queue the playback?
        self.selected_pattern = pattern as usize;
        let new = self.selected_pattern;

        self.update_steps();

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = GlobalEngine::get(&handle).get_sequencer_patterns();
                let mut pattern_row_data = model.row_data(old).unwrap();
                pattern_row_data.active = false;
                model.set_row_data(old, pattern_row_data);

                let mut pattern_row_data = model.row_data(new).unwrap();
                pattern_row_data.active = true;
                model.set_row_data(new, pattern_row_data);
            })
            .unwrap();
    }

    pub fn select_step(&mut self, step: u32) -> () {
        let old_step = self.current_step;
        self.current_step = step as usize;

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = GlobalEngine::get(&handle).get_sequencer_steps();
                let mut row_data = model.row_data(old_step).unwrap();
                row_data.active = false;
                model.set_row_data(old_step, row_data);

                let mut row_data = model.row_data(step as usize).unwrap();
                row_data.active = true;
                model.set_row_data(step as usize, row_data);
            })
            .unwrap();
    }
    pub fn select_instrument(&mut self, instrument: u8) -> () {
        let old_instrument = self.selected_instrument as usize;
        self.selected_instrument = instrument;

        self.update_steps();

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = GlobalEngine::get(&handle).get_instruments();
                let mut row_data = model.row_data(old_instrument).unwrap();
                row_data.selected = false;
                model.set_row_data(old_instrument, row_data);

                let mut row_data = model.row_data(instrument as usize).unwrap();
                row_data.selected = true;
                model.set_row_data(instrument as usize, row_data);

                GlobalEngine::get(&handle).set_current_instrument(instrument as i32);
            })
            .unwrap();
    }

    pub fn cycle_pattern_instrument(&mut self, forwards: bool) -> () {
        let maybe_next = self.song.patterns[self.selected_pattern].next_instrument(self.selected_instrument, forwards);
        if let Some(instrument) = maybe_next {
            self.select_instrument(instrument)
        }
    }

    pub fn toggle_mute_instrument(&mut self, instrument: u8) -> () {
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

    fn update_patterns(&mut self) -> () {
        let non_empty_patterns: Vec<usize> = (0..NUM_PATTERNS)
            .filter(|&p| !self.song.patterns[p].is_empty())
            .collect();

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let patterns = GlobalEngine::get(&handle).get_sequencer_patterns();
                for p in non_empty_patterns {
                    let mut pattern_row_data = patterns.row_data(p).unwrap();
                    pattern_row_data.empty = false;
                    patterns.set_row_data(p, pattern_row_data);
                }
            })
            .unwrap();
    }

    fn update_steps(&mut self) -> () {
        let pattern = &self.song.patterns[self.selected_pattern];
        let maybe_steps = pattern.get_steps(self.selected_instrument).map(|s| s.clone());
        let instruments = pattern.instruments().clone();
        // FIXME: Falling back to 0 makes the navigation difficult.
        //        It would be nice to have the right insertion point, but without having instruments
        //        ordered by synth_index in the patterns this will be difficult.
        let instruments_to_skip = pattern.find_instrument_pos(self.selected_instrument).map(|i| i + 1).unwrap_or(0);

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = GlobalEngine::get(&handle).get_sequencer_steps();
                for (i, step) in maybe_steps.unwrap_or_default().iter().enumerate() {
                    let mut row_data = model.row_data(i).unwrap();
                    row_data.press = step.press;
                    row_data.release = step.release;
                    // FIXME: Pre-create the note SharedStrings somewhere
                    row_data.note_name = MidiNote(step.note as i32).name().into();
                    model.set_row_data(i, row_data);
                }

                let model2 = GlobalEngine::get(&handle).get_sequencer_pattern_instruments();
                let vec_model = model2.as_any().downcast_ref::<VecModel<PatternInstrumentData>>().unwrap();

                let modeled: Vec<PatternInstrumentData> =
                    instruments.iter()
                        .cycle()
                        .skip(instruments_to_skip)
                        .take(if instruments_to_skip == 0 { instruments.len() } else { instruments.len() - 1 })
                        .map(|mi| {
                            let steps_empty: Vec<bool> = mi.steps.iter()
                                .map(|s| !(s.press || s.release))
                                .collect();
                            PatternInstrumentData {
                                id: (&mi.id).into(),
                                steps_empty: slint::ModelRc::new(VecModel::from(steps_empty)),
                            }
                        }).collect();
                vec_model.set_vec(modeled);
            })
            .unwrap();
    }

    pub fn toggle_step(&mut self, step_num: u32) -> () {
        let maybe_steps = self.song.patterns[self.selected_pattern].get_steps(self.selected_instrument);
        let toggled = !maybe_steps.map_or(false, |ss| ss[step_num as usize].press);
        self.set_step_events(
            step_num as usize,
            self.selected_pattern,
            Some(toggled),
            Some(toggled),
            None,
        );

        // We don't yet have a separate concept of step cursor independent of the
        // current sequencer step. But for now use the same thing to allow selecting
        // which step to record a note onto when the playback is stopped.
        if !self.playing {
            self.select_step(step_num);
        }
    }

    pub fn toggle_step_release(&mut self, step_num: u32) -> () {
        let maybe_steps = self.song.patterns[self.selected_pattern].get_steps(self.selected_instrument);
        let toggled = !maybe_steps.map_or(false, |ss| ss[step_num as usize].release);
        self.set_step_events(step_num as usize, self.selected_pattern, None, Some(toggled), None);

        // We don't yet have a separate concept of step cursor independent of the
        // current sequencer step. But for now use the same thing to allow selecting
        // which step to record a note onto when the playback is stopped.
        if !self.playing {
            self.select_step(step_num);
        }
    }

    pub fn manually_advance_step(&mut self, forwards: bool) -> () {
        if !self.playing {
            self.advance_step(forwards);
        }
    }

    fn advance_step(&mut self, forwards: bool) -> () {
        let (next_step, next_pattern, next_song_pattern) = self.next_step_and_pattern_and_song_pattern(forwards);
        self.select_step(next_step as u32);

        if next_pattern != self.selected_pattern {
            self.select_pattern(next_pattern as u32);
        }
        if next_song_pattern != self.current_song_pattern {
            self.select_song_pattern(next_song_pattern.map(|sp| sp as u32));
        }
    }

    fn set_step_events(
        &mut self,
        step_num: usize,
        pattern: usize,
        set_press: Option<bool>,
        set_release: Option<bool>,
        set_note: Option<u8>,
    ) -> () {
        let instrument_id = &self.synth_instrument_ids[self.selected_instrument as usize];
        let pattern_empty = self.song.patterns[pattern].set_step_events(
            self.selected_instrument,
            instrument_id,
            step_num,
            set_press,
            set_release,
            set_note,
        );

        let selected_pattern = self.selected_pattern;
        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let patterns = GlobalEngine::get(&handle).get_sequencer_patterns();
                let mut pattern_row_data = patterns.row_data(selected_pattern).unwrap();
                pattern_row_data.empty = pattern_empty;
                patterns.set_row_data(selected_pattern, pattern_row_data);

                let steps = GlobalEngine::get(&handle).get_sequencer_steps();
                let mut step_row_data = steps.row_data(step_num).unwrap();
                if let Some(press) = set_press {
                    step_row_data.press = press;
                }
                if let Some(release) = set_release {
                    step_row_data.release = release;
                }
                if let Some(note) = set_note {
                    step_row_data.note_name = MidiNote(note as i32).name().into();
                }
                steps.set_row_data(step_num, step_row_data);
            })
            .unwrap();
    }
    pub fn set_playing(&mut self, val: bool) -> Vec<(u8, NoteEvent, u8)> {
        self.playing = val;
        // Reset the current_frame so that it's aligned with full
        // steps and that record_press would record any key while
        // stopped to the current frame and not the next.
        self.current_frame = 0;

        if self.playing {
            // The first advance_frame after playing will move from frame 0 to frame 1 and skip presses
            // of frame 0. Since we don't care about releases of the non-existant previous frame, do the
            // presses now, right after starting the playback.
            let mut note_events: Vec<(u8, NoteEvent, u8)> = Vec::new();
            self.handle_current_step_presses(&mut note_events);
            note_events
        } else {
            Vec::new()
        }
    }
    pub fn set_recording(&mut self, val: bool) -> () {
        self.recording = val;
    }
    pub fn set_erasing(&mut self, val: bool) -> () {
        self.erasing = val;
        // Already remove the current step.
        self.set_step_events(self.current_step, self.selected_pattern, Some(false), Some(false), None);
    }

    fn handle_current_step_presses(&mut self, note_events: &mut Vec<(u8, NoteEvent, u8)>) {
        for instrument in &self.song.patterns[self.selected_pattern].instruments {
            let i = match instrument.synth_index {
                Some(i) => i,
                None => {
                    // elog!("The song is attempting to press instrument id [{}], but the instruments don't define it, ignoring.", instrument.id);
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
            if let Some(InstrumentStep {
                note,
                press,
                release: _,
            }) = self.song.patterns[self.selected_pattern]
                .get_steps(i)
                .map(|ss| ss[self.current_step])
            {
                if press {
                    log!(
                        "➕ PRS {} note {}",
                        self.synth_instrument_ids[i as usize],
                        MidiNote(note as i32).name()
                    );
                    note_events.push((i, NoteEvent::Press, note));
                }
            }
        }
    }

    fn handle_current_step_releases(&mut self, note_events: &mut Vec<(u8, NoteEvent, u8)>) {
        for instrument in &self.song.patterns[self.selected_pattern].instruments {
            let i = match instrument.synth_index {
                Some(i) => i,
                None => {
                    // elog!("The song is attempting to release instrument id [{}], but the instruments don't define it, ignoring.", instrument.id);
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
                note,
                press: _,
                release,
            }) = self.song.patterns[self.selected_pattern]
                .get_steps(i)
                .map(|ss| ss[self.current_step])
            {
                if release {
                    log!(
                        "➖ REL {} note {}",
                        self.synth_instrument_ids[i as usize],
                        MidiNote(note as i32).name()
                    );
                    note_events.push((i, NoteEvent::Release, note));
                }
            }
        }
    }

    pub fn advance_frame(&mut self) -> (Option<u32>, Vec<(u8, NoteEvent, u8)>) {
        let mut note_events: Vec<(u8, NoteEvent, u8)> = Vec::new();

        if !self.playing {
            return (None, note_events);
        }

        // FIXME: Reset or remove overflow check
        self.current_frame += 1;
        if self.current_frame % self.song.frames_per_step == 0 {
            // Release are at then end of a step, so start by triggering any release of the
            // previous frame.
            self.handle_current_step_releases(&mut note_events);

            self.advance_step(true);
            if self.erasing {
                self.set_step_events(self.current_step, self.selected_pattern, Some(false), Some(false), None);
            }

            self.handle_current_step_presses(&mut note_events);
            (Some(self.current_step as u32), note_events)
        } else {
            (None, note_events)
        }
    }

    fn record_event(&mut self, event: NoteEvent, note: Option<u8>) {
        if !self.recording {
            return;
        }

        let (press, release, (step, pattern, _)) = match event {
            NoteEvent::Press if !self.playing => {
                let pressed = self.song.patterns[self.selected_pattern]
                    .get_steps(self.selected_instrument)
                    .map_or(false, |ss| ss[self.current_step as usize].press);
                if !pressed {
                    // If the step isn't already pressed, set both the press and the release it.
                    (Some(true), Some(true), (self.current_step, self.selected_pattern, None))
                } else {
                    // Else, only set the note.
                    (None, None, (self.current_step, self.selected_pattern, None))
                }
            }
            NoteEvent::Release if !self.playing =>
            // Ignore the release when recording and not playing,
            // it should be the same step as the press anyway.
            {
                return
            }
            NoteEvent::Press => {
                (
                    Some(true),
                    None,
                    // Try to clamp the event to the nearest frame.
                    // Use 4 instead of 3 just to try to compensate for the key press to visual and audible delay.
                    if self.current_frame % self.song.frames_per_step < self.snap_at_step_frame() {
                        (self.current_step, self.selected_pattern, None)
                    } else {
                        self.just_recorded_over_next_step = true;
                        self.next_step_and_pattern_and_song_pattern(true)
                    },
                )
            }
            NoteEvent::Release => {
                // Align the release with the same frame position within the step as the press had.
                // We're going to sequence full steps anyway.
                // This is to prevent the release to be offset only by one frame but still end up
                // one step later just because the press would already have been on the step's edge itself.
                // To do so, first find the frames length rounded to the number of frames per step,
                // and add it to the press frame.
                fn round(n: u32, to: u32) -> u32 { (n + to / 2) / to * to }
                let rounded_steps_note_length =
                    round(self.current_frame - self.last_press_frame.unwrap(), self.song.frames_per_step);
                // We need to place the release in the previous step (its end), so substract one step.
                let rounded_end_frame =
                    self.last_press_frame.unwrap() + (rounded_steps_note_length.max(1) - 1);

                let is_end_in_prev_step =
                    rounded_end_frame / self.song.frames_per_step < self.current_frame / self.song.frames_per_step;
                let end_snaps_to_next_step = rounded_end_frame % self.song.frames_per_step < self.snap_at_step_frame();
                (
                    None,
                    Some(true),
                    if is_end_in_prev_step && end_snaps_to_next_step {
                        // It ends before the snap frame of the previous step.
                        // Register the release at the end of the previous step.
                        self.next_step_and_pattern_and_song_pattern(false)
                    } else if is_end_in_prev_step || end_snaps_to_next_step {
                        // It ends between the snap frame of the previous step and the snap frame of the current step
                        // Register the release at the end of the current step.
                        (self.current_step, self.selected_pattern, None)
                    } else {
                        self.just_recorded_over_next_step = true;
                        // It ends on or after the snap frame of the current step.
                        // Register the release at the end of the next step.
                        self.next_step_and_pattern_and_song_pattern(true)
                    },
                )
            }
        };
        self.set_step_events(step, pattern, press, release, note);
    }

    pub fn record_press(&mut self, note: u8) {
        self.record_event(NoteEvent::Press, Some(note));
        self.last_press_frame = Some(self.current_frame);
    }

    pub fn record_release(&mut self, _note: u8) {
        // The note release won't be passed to the synth on playback,
        // so don't overwrite the note in the step just in case it contained something useful.
        self.record_event(NoteEvent::Release, None);
    }

    pub fn append_song_pattern(&mut self, pattern: u32) {
        self.song.song_patterns.push(pattern as usize);

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = GlobalEngine::get(&handle).get_sequencer_song_patterns();
                let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
                vec_model.push(SongPatternData {
                    number: pattern as i32,
                    active: false,
                });
            })
            .unwrap();
    }

    pub fn remove_last_song_pattern(&mut self) {
        if !self.song.song_patterns.is_empty() {
            self.song.song_patterns.pop();
            if self.current_song_pattern == Some(self.song.song_patterns.len()) {
                self.select_song_pattern(if self.song.song_patterns.is_empty() {
                    None
                } else {
                    Some(0)
                });
            }

            self.main_window
                .upgrade_in_event_loop(move |handle| {
                    let model = GlobalEngine::get(&handle).get_sequencer_song_patterns();
                    let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
                    vec_model.remove(vec_model.row_count() - 1);
                })
                .unwrap();
        }
    }

    pub fn clear_song_patterns(&mut self) {
        self.song.song_patterns.clear();
        self.select_song_pattern(None);

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = GlobalEngine::get(&handle).get_sequencer_song_patterns();
                let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
                vec_model.set_vec(Vec::new());
            })
            .unwrap();
    }

    fn set_song(&mut self, song: SequencerSong) {
        self.song = song;

        self.current_song_pattern = if self.song.song_patterns.is_empty() {
            None
        } else {
            Some(0)
        };

        let current_song_pattern = self.current_song_pattern;
        let song_patterns = self.song.song_patterns.clone();
        let frames_per_step = self.song.frames_per_step;
        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = GlobalEngine::get(&handle).get_sequencer_song_patterns();
                let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
                for (i, number) in song_patterns.iter().enumerate() {
                    vec_model.push(SongPatternData {
                        number: *number as i32,
                        active: match current_song_pattern {
                            Some(sp) => i == sp,
                            None => false,
                        },
                    });
                }

                let mut settings: SongSettings = Default::default();
                settings.frames_per_step = frames_per_step as i32;
                GlobalSettings::get(&handle).set_song_settings(settings);
            })
            .unwrap();

        self.select_pattern(
            *self
                .current_song_pattern
                .map(|i| self.song.song_patterns.get(i).unwrap())
                .unwrap_or(&0_usize) as u32,
        );
        self.select_instrument(self.selected_instrument);
        self.update_patterns();
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
            return Err(format!("Project song file {:?} doesn't exist.", song_path).into());
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

    pub fn set_synth_instrument_ids(&mut self, instrument_ids: &Vec<String>) {
        for p in &mut self.song.patterns {
            p.update_synth_index(instrument_ids);
        }
        self.update_steps();

        self.synth_instrument_ids = instrument_ids.clone();
    }

    fn snap_at_step_frame(&self) -> u32 {
        // Use +1 just to try to compensate for the key press to visual and audible delay.
        self.song.frames_per_step / 2 + 1
    }

    fn next_step_and_pattern_and_song_pattern(&self, forwards: bool) -> (usize, usize, Option<usize>) {
        let delta = if forwards { 1_isize } else { -1 };
        let next_step = ((self.current_step as isize + NUM_STEPS as isize + delta) % NUM_STEPS as isize) as usize;
        let wraps = forwards && next_step == 0 || !forwards && self.current_step == 0;
        if wraps {
            let (next_pattern, next_song_pattern) = if !self.song.song_patterns.is_empty() {
                let sp = self
                    .current_song_pattern
                    .map(|sp| {
                        ((sp as isize + self.song.song_patterns.len() as isize + delta)
                            % self.song.song_patterns.len() as isize) as usize
                    })
                    .unwrap_or(0);
                (self.song.song_patterns[sp], Some(sp))
            } else {
                (self.selected_pattern, None)
            };
            return (next_step, next_pattern, next_song_pattern);
        }
        (next_step, self.selected_pattern, self.current_song_pattern)
    }
}
