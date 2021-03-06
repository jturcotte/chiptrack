// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::utils::MidiNote;
use crate::GlobalEngine;
use crate::MainWindow;
use crate::SongPatternData;
use serde::{Deserialize, Serialize};
use slint::Global;
use slint::Model;
use slint::VecModel;
use slint::Weak;
use std::collections::HashSet;
use std::fs::File;
use std::path::Path;

use crate::sound_engine::NUM_INSTRUMENTS;
use crate::sound_engine::NUM_PATTERNS;
use crate::sound_engine::NUM_STEPS;

#[cfg(target_arch = "wasm32")]
use crate::utils;

const FRAMES_PER_STEP: u32 = 6;
const SNAP_AT_STEP_FRAME: u32 = 4;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum NoteEvent {
    Press,
    Release,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
struct InstrumentStep {
    note: u32,
    press: bool,
    release: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SequencerSong {
    pub selected_instrument: u32,
    song_patterns: Vec<usize>,
    step_instruments: Vec<Vec<[InstrumentStep; NUM_STEPS]>>,
    #[serde(default)]
    muted_instruments: HashSet<u32>,
}

impl Default for SequencerSong {
    fn default() -> Self {
        SequencerSong {
            selected_instrument: 0,
            song_patterns: Vec::new(),
            // Initialize all notes to C5
            step_instruments: vec![
                vec![
                    [InstrumentStep {
                        note: 60,
                        press: false,
                        release: false
                    }; NUM_STEPS];
                    NUM_INSTRUMENTS
                ];
                NUM_PATTERNS
            ],
            muted_instruments: HashSet::new(),
        }
    }
}

pub struct Sequencer {
    pub song: SequencerSong,
    current_frame: u32,
    current_step: usize,
    current_song_pattern: Option<usize>,
    selected_pattern: usize,
    playing: bool,
    recording: bool,
    erasing: bool,
    last_press_frame: Option<u32>,
    just_recorded_over_next_step: bool,
    main_window: Weak<MainWindow>,
}

impl Sequencer {
    pub fn new(main_window: Weak<MainWindow>) -> Sequencer {
        Sequencer {
            song: Default::default(),
            current_frame: 0,
            current_step: 0,
            current_song_pattern: None,
            selected_pattern: 0,
            playing: false,
            recording: true,
            erasing: false,
            last_press_frame: None,
            just_recorded_over_next_step: false,
            main_window: main_window.clone(),
        }
    }

    pub fn select_song_pattern(&mut self, song_pattern: Option<u32>) -> () {
        let old = self.current_song_pattern;
        self.current_song_pattern = song_pattern.map(|sp| sp as usize);
        let new = self.current_song_pattern;

        self.main_window.upgrade_in_event_loop(move |handle| {
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
        });
    }

    pub fn select_pattern(&mut self, pattern: u32) -> () {
        let old = self.selected_pattern;
        // FIXME: Queue the playback?
        self.selected_pattern = pattern as usize;
        let new = self.selected_pattern;

        self.update_steps();

        self.main_window.upgrade_in_event_loop(move |handle| {
            let model = GlobalEngine::get(&handle).get_sequencer_patterns();
            let mut pattern_row_data = model.row_data(old).unwrap();
            pattern_row_data.active = false;
            model.set_row_data(old, pattern_row_data);

            let mut pattern_row_data = model.row_data(new).unwrap();
            pattern_row_data.active = true;
            model.set_row_data(new, pattern_row_data);
        });
    }

    pub fn select_step(&mut self, step: u32) -> () {
        let old_step = self.current_step;
        self.current_step = step as usize;

        self.main_window.upgrade_in_event_loop(move |handle| {
            let model = GlobalEngine::get(&handle).get_sequencer_steps();
            let mut row_data = model.row_data(old_step).unwrap();
            row_data.active = false;
            model.set_row_data(old_step, row_data);

            let mut row_data = model.row_data(step as usize).unwrap();
            row_data.active = true;
            model.set_row_data(step as usize, row_data);
        });
    }
    pub fn select_instrument(&mut self, instrument: u32) -> () {
        let old_instrument = self.song.selected_instrument as usize;
        self.song.selected_instrument = instrument;

        self.update_steps();

        self.main_window.upgrade_in_event_loop(move |handle| {
            let model = GlobalEngine::get(&handle).get_instruments();
            let mut row_data = model.row_data(old_instrument).unwrap();
            row_data.selected = false;
            model.set_row_data(old_instrument, row_data);

            let mut row_data = model.row_data(instrument as usize).unwrap();
            row_data.selected = true;
            model.set_row_data(instrument as usize, row_data);
        });
    }

    pub fn toggle_mute_instrument(&mut self, instrument: u32) -> () {
        let was_muted = self.song.muted_instruments.take(&instrument).is_some();
        if !was_muted {
            self.song.muted_instruments.insert(instrument);
        }

        self.main_window.upgrade_in_event_loop(move |handle| {
            let model = GlobalEngine::get(&handle).get_instruments();
            let mut row_data = model.row_data(instrument as usize).unwrap();
            row_data.muted = !was_muted;
            model.set_row_data(instrument as usize, row_data);
        });
    }

    fn update_patterns(&mut self) -> () {
        let non_empty_patterns: Vec<usize> = (0..NUM_PATTERNS)
            .filter(|p| {
                (0..NUM_INSTRUMENTS).any(|i| {
                    (0..NUM_STEPS).any(|s| {
                        let step = self.song.step_instruments[*p][i][s];
                        step.press || step.release
                    })
                })
            })
            .collect();

        self.main_window.upgrade_in_event_loop(move |handle| {
            let patterns = GlobalEngine::get(&handle).get_sequencer_patterns();
            for p in non_empty_patterns {
                let mut pattern_row_data = patterns.row_data(p).unwrap();
                pattern_row_data.empty = false;
                patterns.set_row_data(p, pattern_row_data);
            }
        });
    }

    fn update_steps(&mut self) -> () {
        let steps: Vec<InstrumentStep> = (0..NUM_STEPS)
            .map(|i| self.song.step_instruments[self.selected_pattern][self.song.selected_instrument as usize][i])
            .collect();
        self.main_window.upgrade_in_event_loop(move |handle| {
            let model = GlobalEngine::get(&handle).get_sequencer_steps();
            for (i, step) in steps.iter().enumerate() {
                let mut row_data = model.row_data(i).unwrap();
                row_data.press = step.press;
                row_data.release = step.release;
                row_data.note_name = MidiNote(step.note as i32).name();
                model.set_row_data(i, row_data);
            }
        });
    }

    pub fn toggle_step(&mut self, step_num: u32) -> () {
        let step = self.song.step_instruments[self.selected_pattern][self.song.selected_instrument as usize]
            [step_num as usize];
        let toggled = !step.press;
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
        let step = self.song.step_instruments[self.selected_pattern][self.song.selected_instrument as usize]
            [step_num as usize];
        let toggled = !step.release;
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
        set_note: Option<u32>,
    ) -> () {
        let mut step = &mut self.song.step_instruments[pattern][self.song.selected_instrument as usize][step_num];
        if set_press.map_or(true, |v| v == step.press)
            && set_release.map_or(true, |v| v == step.release)
            && set_note.map_or(true, |v| v == step.note)
        {
            return;
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
            (0..NUM_INSTRUMENTS).all(|i| {
                (0..NUM_STEPS).all(|s| {
                    let step = self.song.step_instruments[self.selected_pattern][i][s];
                    !step.press && !step.release
                })
            })
        };

        let selected_pattern = self.selected_pattern;
        self.main_window.upgrade_in_event_loop(move |handle| {
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
                step_row_data.note_name = MidiNote(note as i32).name();
            }
            steps.set_row_data(step_num, step_row_data);
        });
    }
    pub fn set_playing(&mut self, val: bool) -> () {
        self.playing = val;
        // Reset the current_frame so that it's aligned with full
        // steps and that record_press would record any key while
        // stopped to the current frame and not the next.
        self.current_frame = 0;
    }
    pub fn set_recording(&mut self, val: bool) -> () {
        self.recording = val;
    }
    pub fn set_erasing(&mut self, val: bool) -> () {
        self.erasing = val;
        // Already remove the current step.
        self.set_step_events(self.current_step, self.selected_pattern, Some(false), Some(false), None);
    }
    pub fn advance_frame(&mut self) -> (Option<u32>, Vec<(u32, NoteEvent, u32)>) {
        let mut note_events: Vec<(u32, NoteEvent, u32)> = Vec::new();

        if !self.playing {
            return (None, note_events);
        }

        // FIXME: Reset or remove overflow check
        self.current_frame += 1;
        if self.current_frame % FRAMES_PER_STEP == 0 {
            // Release are at then end of a step, so start by triggering any release of the
            // previous frame.
            for i in 0..NUM_INSTRUMENTS {
                if self.song.muted_instruments.contains(&(i as u32)) {
                    continue;
                }
                if self.just_recorded_over_next_step && i as u32 == self.song.selected_instrument {
                    // Let the press loop further down reset the flag.
                    continue;
                }
                let InstrumentStep {
                    note,
                    press: _,
                    release,
                } = self.song.step_instruments[self.selected_pattern][i][self.current_step];
                if release {
                    println!("➖ Instrument release {:?} note {:?}", i, note);
                    note_events.push((i as u32, NoteEvent::Release, note));
                }
            }

            self.advance_step(true);
            if self.erasing {
                self.set_step_events(self.current_step, self.selected_pattern, Some(false), Some(false), None);
            }

            for i in 0..NUM_INSTRUMENTS {
                if self.song.muted_instruments.contains(&(i as u32)) {
                    continue;
                }
                if self.just_recorded_over_next_step && i as u32 == self.song.selected_instrument {
                    self.just_recorded_over_next_step = false;
                    continue;
                }
                let InstrumentStep {
                    note,
                    press,
                    release: _,
                } = self.song.step_instruments[self.selected_pattern][i][self.current_step];
                if press {
                    println!("➕ Instrument press {:?} note {:?}", i, note);
                    note_events.push((i as u32, NoteEvent::Press, note));
                }
            }
            (Some(self.current_step as u32), note_events)
        } else {
            (None, note_events)
        }
    }

    fn record_event(&mut self, event: NoteEvent, note: Option<u32>) {
        if !self.recording {
            return;
        }

        let (press, release, (step, pattern, _)) = match event {
            NoteEvent::Press if !self.playing => {
                let step = self.song.step_instruments[self.selected_pattern][self.song.selected_instrument as usize]
                    [self.current_step as usize];
                if !step.press {
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
                    if self.current_frame % FRAMES_PER_STEP < SNAP_AT_STEP_FRAME {
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
                let steps_note_length = ((self.current_frame as f32 - self.last_press_frame.unwrap() as f32)
                    / FRAMES_PER_STEP as f32)
                    .round() as u32;
                // We need to place the release in the previous step (its end), so substract one step.
                let rounded_end_frame =
                    self.last_press_frame.unwrap() + (steps_note_length.max(1) - 1) * FRAMES_PER_STEP;

                let is_end_in_prev_step = rounded_end_frame / FRAMES_PER_STEP < self.current_frame / FRAMES_PER_STEP;
                let end_snaps_to_next_step = rounded_end_frame % FRAMES_PER_STEP < SNAP_AT_STEP_FRAME;
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

    pub fn record_press(&mut self, note: u32) {
        self.record_event(NoteEvent::Press, Some(note));
        self.last_press_frame = Some(self.current_frame);
    }

    pub fn record_release(&mut self, _note: u32) {
        // The note release won't be passed to the synth on playback,
        // so don't overwrite the note in the step just in case it contained something useful.
        self.record_event(NoteEvent::Release, None);
    }

    pub fn append_song_pattern(&mut self, pattern: u32) {
        self.song.song_patterns.push(pattern as usize);

        self.main_window.upgrade_in_event_loop(move |handle| {
            let model = GlobalEngine::get(&handle).get_sequencer_song_patterns();
            let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
            vec_model.push(SongPatternData {
                number: pattern as i32,
                active: false,
            });
        });
    }

    pub fn remove_last_song_pattern(&mut self) {
        if !self.song.song_patterns.is_empty() {
            self.song.song_patterns.pop();
            if self.current_song_pattern.unwrap() == self.song.song_patterns.len() {
                self.select_song_pattern(if self.song.song_patterns.is_empty() {
                    None
                } else {
                    Some(0)
                });
            }

            self.main_window.upgrade_in_event_loop(move |handle| {
                let model = GlobalEngine::get(&handle).get_sequencer_song_patterns();
                let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
                vec_model.remove(vec_model.row_count() - 1);
            });
        }
    }

    pub fn clear_song_patterns(&mut self) {
        self.song.song_patterns.clear();
        self.select_song_pattern(None);

        self.main_window.upgrade_in_event_loop(move |handle| {
            let model = GlobalEngine::get(&handle).get_sequencer_song_patterns();
            let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
            vec_model.set_vec(Vec::new());
        });
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
        self.main_window.upgrade_in_event_loop(move |handle| {
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
        });

        self.select_pattern(
            *self
                .current_song_pattern
                .map(|i| self.song.song_patterns.get(i).unwrap())
                .unwrap_or(&0_usize) as u32,
        );
        self.select_instrument(self.song.selected_instrument as u32);
        self.update_patterns();
    }

    #[cfg(target_arch = "wasm32")]
    fn deserialize_song(base64: String) -> Result<SequencerSong, Box<dyn std::error::Error>> {
        let decoded = utils::decode_string(&base64)?;
        let deserialized = serde_json::from_str(&decoded)?;
        Ok(deserialized)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load(&mut self, maybe_base64: Option<String>) {
        if let Some(base64) = maybe_base64 {
            let parsed = Sequencer::deserialize_song(base64);

            match parsed {
                Ok(mut song) => {
                    log!("Loaded the project song from the URL.");
                    // Expand the song in memory again.
                    song.step_instruments.resize_with(NUM_PATTERNS, || {
                        [[InstrumentStep {
                            note: 60,
                            press: false,
                            release: false,
                        }; NUM_STEPS]; NUM_INSTRUMENTS]
                    });
                    self.set_song(song);
                }
                Err(e) => {
                    elog!(
                        "Couldn't load the project song from the URL, starting from scratch.\n\tError: {:?}",
                        e
                    );
                    self.set_song(Default::default());
                }
            }
        } else {
            log!("No song provided in the URL, starting from scratch.");
            self.set_song(Default::default());
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(&mut self, project_song_path: &Path) {
        if project_song_path.exists() {
            let parsed: Result<SequencerSong, std::io::Error> =
                File::open(project_song_path).and_then(|f| serde_json::from_reader(f).map_err(|e| e.into()));

            match parsed {
                Ok(mut song) => {
                    log!("Loaded project song from file {:?}", project_song_path);
                    // Expand the song in memory again.
                    for i in &mut song.step_instruments {
                        i.resize_with(NUM_INSTRUMENTS, || {
                            [InstrumentStep {
                                note: 60,
                                press: false,
                                release: false,
                            }; NUM_STEPS]
                        });
                    }
                    song.step_instruments.resize_with(NUM_PATTERNS, || {
                        vec![
                            [InstrumentStep {
                                note: 60,
                                press: false,
                                release: false,
                            }; NUM_STEPS];
                            NUM_INSTRUMENTS
                        ]
                    });
                    self.set_song(song);
                }
                Err(e) => {
                    elog!(
                        "Couldn't load project song from file {:?}, starting from scratch.\n\tError: {:?}",
                        project_song_path,
                        e
                    );
                    self.set_song(Default::default());
                }
            }
        } else {
            log!(
                "Project song file {:?} doesn't exist, starting from scratch.",
                project_song_path
            );
            self.set_song(Default::default());
        }
    }

    pub fn save(&self, project_song_path: &Path) {
        println!("Saving project song to file {:?}.", project_song_path);
        let f = File::create(project_song_path).expect("Unable to create project file");
        serde_json::to_writer_pretty(&f, &self.song).unwrap()
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
