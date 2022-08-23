// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::sequencer::NoteEvent;
use crate::sequencer::Sequencer;
use crate::synth::Synth;
use crate::GlobalEngine;
use crate::MainWindow;
use crate::Settings;
use slint::Global;
use slint::Model;
use slint::Weak;
use std::path::PathBuf;

pub const NUM_INSTRUMENTS: usize = 64;
pub const NUM_STEPS: usize = 16;
pub const NUM_PATTERNS: usize = 64;

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
    project_name: Option<String>,
}

impl SoundEngine {
    pub fn new(sample_rate: u32, main_window: Weak<MainWindow>, settings: Settings) -> SoundEngine {
        let sequencer = Sequencer::new(main_window.clone());
        let synth = Synth::new(main_window.clone(), sample_rate, settings);

        SoundEngine {
            sequencer: sequencer,
            synth: synth,
            main_window: main_window,
            pressed_note: None,
            project_name: None,
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

            self.main_window.upgrade_in_event_loop(move |handle| {
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
                let instruments_model = GlobalEngine::get(&handle).get_instruments();
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
        self.main_window.upgrade_in_event_loop(move |handle| {
            let model = handle.get_notes();
            for row in 0..model.row_count() {
                let mut row_data = model.row_data(row).unwrap();
                row_data.active = false;
                model.set_row_data(row, row_data);
            }
        });
    }

    fn release_note_visually(&mut self, note: u32) -> () {
        self.main_window.upgrade_in_event_loop(move |handle| {
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

        self.main_window.upgrade_in_event_loop(move |handle| {
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

    pub fn load(&mut self, project_name: String) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let song_path = SoundEngine::project_song_path(&project_name);
            let instruments_path = SoundEngine::project_instruments_path(&project_name);
            log!("Loading the project song from file {:?}", song_path);
            self.sequencer.load(song_path.as_path());
            log!("Loading project instruments from file {:?}", instruments_path);
            self.synth.load(instruments_path.as_path());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Loading a project file isn't supported on wasm, just load the default instruments.
            self.synth.load_default_instruments();
        }

        self.project_name = Some(project_name);
    }

    pub fn load_project_from_gist(&mut self, json: serde_json::Value) {
        self.load_project_from_gist_internal(json)
            .unwrap_or_else(|err| elog!("Error extracting project from gist: {}", err));
    }

    fn load_project_from_gist_internal(&mut self, json: serde_json::Value) -> Result<(), String> {
        let files = json
            .get("files")
            .ok_or("JSON should have a files property")?
            .as_object()
            .ok_or("The files property should be an object")?;

        let song = files
            .iter()
            .find(|(name, _)| name.ends_with("-song.json"))
            .ok_or("should have a file for the song named <project>-song.json")?
            .1
            .get("content")
            .ok_or("The file should have a content property")?
            .as_str()
            .ok_or("content should be a string")?;
        self.sequencer.load_from_gist(song);

        let instruments = files
            .iter()
            .find(|(name, _)| name.ends_with("-instruments.rhai"))
            .ok_or("should have a file for instruments named <project>-instruments.rhai")?
            .1
            .get("content")
            .ok_or("The file should have a content property")?
            .as_str()
            .ok_or("content should be a string")?;
        self.synth.load_from_gist(instruments);

        Ok(())
    }

    pub fn save_project(&self) {
        if let Some(project_name) = &self.project_name {
            let path = SoundEngine::project_song_path(&project_name);
            self.sequencer.save(path.as_path());
        } else {
            elog!("Can't save a project loaded from a gist URL.");
        }
    }

    pub fn project_song_path(project_name: &str) -> PathBuf {
        let mut path = PathBuf::new();
        path.push(project_name.to_owned() + "-song.json");
        path
    }

    pub fn instruments_path(&self) -> Option<PathBuf> {
        self.project_name
            .as_ref()
            .map(|p| SoundEngine::project_instruments_path(&p))
    }

    pub fn project_instruments_path(project_name: &str) -> PathBuf {
        let mut path = PathBuf::new();
        path.push(project_name.to_owned() + "-instruments.rhai");
        path
    }
}
