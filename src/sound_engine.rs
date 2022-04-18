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

pub struct SoundEngine {
    pub sequencer: Sequencer,
    pub synth: Synth,
    main_window: Weak<MainWindow>,
}

impl SoundEngine {
    pub fn new(sample_rate: u32, project_name: &str, main_window: Weak<MainWindow>, settings: Settings) -> SoundEngine {
        let mut sequencer = Sequencer::new(main_window.clone());
        let mut synth = Synth::new(main_window.clone(), sample_rate, settings);

        #[cfg(not(target_arch = "wasm32"))] {
            let song_path = SoundEngine::project_song_path(project_name);
            let instruments_path = SoundEngine::project_instruments_path(project_name);
            sequencer.load(song_path.as_path());
            synth.load(instruments_path.as_path());
        }

        #[cfg(target_arch = "wasm32")] {
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
            }
    }

    pub fn apply_settings(&mut self, settings: Settings) {
        self.synth.apply_settings(settings);
    }

    pub fn advance_frame(&mut self) -> () {
        let (step_change, note_events) = self.sequencer.advance_frame();
        for (instrument, typ, note) in note_events {
            if typ == NoteEvent::Press {
                self.synth.press_instrument_note(instrument, note);
            } else {
                self.synth.release_instrument_note(instrument, note);
            }
            let selected_instrument = self.sequencer.song.selected_instrument;
            self.main_window.clone().upgrade_in_event_loop(move |handle| {
                let pressed = typ == NoteEvent::Press;
                if instrument == selected_instrument {
                    let notes_model = handle.get_notes();
                    for row in 0..notes_model.row_count() {
                        let mut row_data = notes_model.row_data(row).unwrap();
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

    pub fn press_note(&mut self, note: u32) -> () {
        self.synth.press_instrument_note(self.sequencer.song.selected_instrument, note);
        self.sequencer.record_press(note);
    }

    pub fn release_note(&mut self, note: u32) -> () {
        self.synth.release_instrument_note(self.sequencer.song.selected_instrument, note);
        self.sequencer.record_release(note);
    }

    pub fn save_project(&self, project_name: &str) {
        let path = SoundEngine::project_song_path(project_name);
        self.sequencer.save(path.as_path());

        // Until a better export function is available, just print this on the console when saving.
        let encoded_song = utils::encode_file(SoundEngine::project_song_path(project_name));
        let encoded_instruments = utils::encode_file(SoundEngine::project_instruments_path(project_name));
        println!("Online player URL: http://localhost:8080?p={}&s={}&i={}", project_name, encoded_song, encoded_instruments);
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

