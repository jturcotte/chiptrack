// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::sequencer::NoteEvent;
use crate::sequencer::Sequencer;
use crate::synth::Synth;
use crate::utils;
use crate::MainWindow;
use crate::Settings;
use slint::Model;
use slint::Weak;
use std::path::PathBuf;

pub const NUM_INSTRUMENTS: usize = 16;
pub const NUM_STEPS: usize = 16;
pub const NUM_PATTERNS: usize = 16;

#[derive(PartialEq, Clone, Copy, Debug)]
enum NoteSource {
    Key(u32),
    Sequencer(u32),
}

pub struct SoundEngine {
    pub sequencer: Sequencer,
    pub synth: Synth,
    main_window: Weak<MainWindow>,
    pressed_note: Option<NoteSource>,
}

impl SoundEngine {
    pub fn new(sample_rate: u32, project_name: &str, main_window: Weak<MainWindow>, settings: Settings) -> SoundEngine {
        let mut sequencer = Sequencer::new(main_window.clone());
        let mut synth = Synth::new(main_window.clone(), sample_rate, settings);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let song_path = SoundEngine::project_song_path(project_name);
            let instruments_path = SoundEngine::project_instruments_path(project_name);
            sequencer.load(song_path.as_path());
            synth.load(instruments_path.as_path());
        }

        #[cfg(target_arch = "wasm32")]
        {
            let window = web_sys::window().unwrap();
            let query_string = window.location().search().unwrap();
            let search_params = web_sys::UrlSearchParams::new_with_str(&query_string).unwrap();

            sequencer.load(search_params.get("s"));
            synth.load(search_params.get("i"));
        }

        SoundEngine {
            sequencer: sequencer,
            synth: synth,
            main_window: main_window,
            pressed_note: None,
        }
    }

    pub fn apply_settings(&mut self, settings: Settings) {
        self.synth.apply_settings(settings);
    }

    fn singularize_note_release(&mut self, source: NoteSource, event: NoteEvent) -> Option<u32> {
        let note_to_release = if event == NoteEvent::Press {
            self.pressed_note.replace(source)
        } else {
            match (self.pressed_note, source) {
                // Only pressed live notes if the released note matches (another one wasn't pressed since)
                (Some(NoteSource::Key(pressed_note)), NoteSource::Key(released_note))
                    if pressed_note == released_note =>
                {
                    self.pressed_note.take()
                }
                // Don't release any other kind of pressed key when a live key is released
                (_, NoteSource::Key(_)) => None,
                // Nor when the last pressed note is a live note
                (Some(NoteSource::Key(_)), _) => None,
                // For anything else involving sequencer notes, a release is a wildcard for any note.
                // This function is only called for the selected instrument (where live keys can be mixed),
                // and this behavior must match recorded releases for non-selected instruments always being
                // sent to the synth. For that reason, we don't take the pressed_note and leave it there.
                (_, _) => self.pressed_note,
            }
        };

        note_to_release.map(|ps| match ps {
            NoteSource::Key(note) => note,
            NoteSource::Sequencer(note) => note,
        })
    }

    pub fn advance_frame(&mut self) -> () {
        let (step_change, note_events) = self.sequencer.advance_frame();
        for (instrument, typ, note) in note_events {
            let is_selected_instrument = instrument == self.sequencer.song.selected_instrument;

            let note_to_release = if is_selected_instrument {
                self.singularize_note_release(NoteSource::Sequencer(note), NoteEvent::Press)
            } else {
                None
            };

            if typ == NoteEvent::Press {
                self.synth.press_instrument_note(instrument, note);
            } else if !is_selected_instrument || note_to_release.is_some() {
                // Only send the sequenced release to the synth if the last of the pressed notes is
                // being released, or if this isn't the selected instruments (in which case only one
                // note will be pressed at a time).
                self.synth.release_instrument(instrument);
            };

            self.main_window.clone().upgrade_in_event_loop(move |handle| {
                let pressed = typ == NoteEvent::Press;
                if is_selected_instrument {
                    let notes_model = handle.get_notes();
                    for row in 0..notes_model.row_count() {
                        let mut row_data = notes_model.row_data(row).unwrap();
                        // A note release might not happen if a press happened in-between.
                        if note_to_release.map_or(false, |n| n == row_data.note_number as u32) {
                            row_data.active = false;
                            notes_model.set_row_data(row, row_data.clone());
                        }

                        if row_data.note_number as u32 == note {
                            row_data.active = pressed;
                            notes_model.set_row_data(row, row_data);
                        }
                    }
                }
                let instruments_model = handle.get_instruments();
                let mut row_data = instruments_model.row_data(instrument as usize).unwrap();
                row_data.active = pressed;
                instruments_model.set_row_data(instrument as usize, row_data);
            });
        }
        self.synth.advance_frame(step_change);
    }

    pub fn select_instrument(&mut self, instrument: u32) -> () {
        self.sequencer.select_instrument(instrument);

        self.pressed_note = None;

        // Release all notes visually that might have been pressed for the previous instrument.
        self.main_window.clone().upgrade_in_event_loop(move |handle| {
            let model = handle.get_notes();
            for row in 0..model.row_count() {
                let mut row_data = model.row_data(row).unwrap();
                row_data.active = false;
                model.set_row_data(row, row_data);
            }
        });
    }

    fn release_note_visually(&mut self, note: u32) -> () {
        self.main_window.clone().upgrade_in_event_loop(move |handle| {
            let model = handle.get_notes();
            for row in 0..model.row_count() {
                let mut row_data = model.row_data(row).unwrap();
                if note == row_data.note_number as u32 {
                    row_data.active = false;
                    model.set_row_data(row, row_data);
                }
            }
        });
    }

    pub fn press_note(&mut self, note: u32) -> () {
        self.synth
            .press_instrument_note(self.sequencer.song.selected_instrument, note);
        self.sequencer.record_press(note);

        // Check which not
        if let Some(note_to_release) = self.singularize_note_release(NoteSource::Key(note), NoteEvent::Press) {
            self.release_note_visually(note_to_release)
        }

        self.main_window.clone().upgrade_in_event_loop(move |handle| {
            let model = handle.get_notes();
            for row in 0..model.row_count() {
                let mut row_data = model.row_data(row).unwrap();
                if row_data.note_number == note as i32 {
                    row_data.active = true;
                    model.set_row_data(row, row_data);
                }
            }
        });
    }

    pub fn release_note(&mut self, note: u32) -> () {
        // Instruments are monophonic, ignore any note release, either sequenced or live,
        // for the current instrument if it wasn't the last pressed one.
        if let Some(note_to_release) = self.singularize_note_release(NoteSource::Key(note), NoteEvent::Release) {
            self.synth.release_instrument(self.sequencer.song.selected_instrument);
            self.sequencer.record_release(note);
            self.release_note_visually(note_to_release);
        }
    }

    pub fn save_project(&self, project_name: &str) {
        let path = SoundEngine::project_song_path(project_name);
        self.sequencer.save(path.as_path());

        // Until a better export function is available, just print this on the console when saving.
        let encoded_song = utils::encode_file(SoundEngine::project_song_path(project_name));
        let encoded_instruments = utils::encode_file(SoundEngine::project_instruments_path(project_name));
        println!(
            "Online player URL: http://localhost:8080?p={}&s={}&i={}",
            project_name, encoded_song, encoded_instruments
        );
    }

    pub fn project_song_path(project_name: &str) -> PathBuf {
        let mut path = PathBuf::new();
        path.push(project_name.to_owned() + "-song.json");
        path
    }

    pub fn project_instruments_path(project_name: &str) -> PathBuf {
        let mut path = PathBuf::new();
        path.push(project_name.to_owned() + "-instruments.rhai");
        path
    }
}
