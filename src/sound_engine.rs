// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::sequencer::NoteEvent;
use crate::sequencer::Sequencer;
use crate::synth::Synth;
use crate::GlobalEngine;
use crate::MainWindow;
use crate::Settings;
use crate::SongSettings;

use native_dialog::FileDialog;
use slint::Global;
use slint::Model;
use slint::Weak;

use std::error::Error;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

pub const NUM_INSTRUMENTS: usize = 64;
pub const NUM_INSTRUMENT_COLS: usize = 4;
pub const NUM_STEPS: usize = 16;
pub const NUM_PATTERNS: usize = 64;

#[derive(PartialEq, Clone, Copy, Debug)]
enum NoteSource {
    Key(u32),
    Sequencer(u32),
}

enum ProjectSource {
    New,
    File((PathBuf, PathBuf)),
    Gist,
}

pub struct SoundEngine {
    pub sequencer: Sequencer,
    pub synth: Synth,
    main_window: Weak<MainWindow>,
    pressed_note: Option<NoteSource>,
    project_source: ProjectSource,
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
            project_source: ProjectSource::New,
        }
    }

    pub fn apply_settings(&mut self, settings: &Settings) {
        self.synth.apply_settings(settings);
    }

    pub fn apply_song_settings(&mut self, settings: &SongSettings) {
        self.sequencer.apply_song_settings(settings);
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

    fn send_note_events_to_synth(&mut self, note_events: Vec<(u8, NoteEvent, u32)>) {
        for (instrument, typ, note) in note_events {
            let is_selected_instrument = instrument == self.sequencer.selected_instrument;

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

            self.main_window
                .upgrade_in_event_loop(move |handle| {
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
                })
                .unwrap();
        }
    }

    pub fn set_playing(&mut self, val: bool) -> () {
        let note_events = self.sequencer.set_playing(val);
        self.send_note_events_to_synth(note_events);
    }

    pub fn advance_frame(&mut self) -> () {
        let (step_change, note_events) = self.sequencer.advance_frame();
        self.send_note_events_to_synth(note_events);
        self.synth.advance_frame(step_change);
    }

    pub fn select_instrument(&mut self, instrument: u8) -> () {
        self.sequencer.select_instrument(instrument);

        self.pressed_note = None;

        // Release all notes visually that might have been pressed for the previous instrument.
        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = handle.get_notes();
                for row in 0..model.row_count() {
                    let mut row_data = model.row_data(row).unwrap();
                    row_data.active = false;
                    model.set_row_data(row, row_data);
                }
            })
            .unwrap();
    }

    fn release_note_visually(&mut self, note: u32) -> () {
        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = handle.get_notes();
                for row in 0..model.row_count() {
                    let mut row_data = model.row_data(row).unwrap();
                    if note == row_data.note_number as u32 {
                        row_data.active = false;
                        model.set_row_data(row, row_data);
                    }
                }
            })
            .unwrap();
    }

    pub fn press_note(&mut self, note: u32) -> () {
        self.synth
            .press_instrument_note(self.sequencer.selected_instrument, note);
        self.sequencer.record_press(note);

        // Check which not
        if let Some(note_to_release) = self.singularize_note_release(NoteSource::Key(note), NoteEvent::Press) {
            self.release_note_visually(note_to_release)
        }

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = handle.get_notes();
                for row in 0..model.row_count() {
                    let mut row_data = model.row_data(row).unwrap();
                    if row_data.note_number == note as i32 {
                        row_data.active = true;
                        model.set_row_data(row, row_data);
                    }
                }
            })
            .unwrap();
    }

    pub fn release_note(&mut self, note: u32) -> () {
        // Instruments are monophonic, ignore any note release, either sequenced or live,
        // for the current instrument if it wasn't the last pressed one.
        if let Some(note_to_release) = self.singularize_note_release(NoteSource::Key(note), NoteEvent::Release) {
            self.synth.release_instrument(self.sequencer.selected_instrument);
            self.sequencer.record_release(note);
            self.release_note_visually(note_to_release);
        }
    }

    pub fn load_default(&mut self) {
        self.sequencer.load_default();
        self.synth.load_default();
        self.sequencer.set_synth_instrument_ids(&self.synth.instrument_ids());
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_file(&mut self, song_path: &Path) {
        match self.load_file_internal(song_path) {
            Ok(instruments_path) => self.project_source = ProjectSource::File((song_path.to_owned(), instruments_path)),
            Err(err) => elog!("Error extracting project from file [{:?}]: {}", song_path, err),
        }
    }

    pub fn load_gist(&mut self, json: serde_json::Value) {
        match self.load_gist_internal(json) {
            Ok(_) => self.project_source = ProjectSource::Gist,
            Err(err) => elog!("Error extracting project from gist: {}", err),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_file_internal(&mut self, song_path: &Path) -> Result<PathBuf, Box<dyn Error>> {
        log!("Loading the project song from file {:?}", song_path);
        let instruments_file = self.sequencer.load_file(song_path)?;
        let instruments_path = song_path.with_file_name(instruments_file);
        log!("Loading project instruments from file {:?}", instruments_path);
        self.synth.load_file(instruments_path.as_path())?;
        self.sequencer.set_synth_instrument_ids(&self.synth.instrument_ids());
        Ok(instruments_path)
    }

    fn load_gist_internal(&mut self, json: serde_json::Value) -> Result<(), Box<dyn Error>> {
        let files = json
            .get("files")
            .ok_or("JSON should have a files property")?
            .as_object()
            .ok_or("The files property should be an object")?;

        let song = files
            .iter()
            .find(|(name, _)| name.ends_with(".ct.md"))
            .ok_or("should have a file for the song with the extension 'ct.md'")?
            .1
            .get("content")
            .ok_or("The file should have a content property")?
            .as_str()
            .ok_or("content should be a string")?;
        let instruments_file = self.sequencer.load_str(song)?;

        let instruments = files
            .get(&instruments_file)
            .ok_or_else(|| format!("The gist should have a file named {}", instruments_file))?
            .get("content")
            .ok_or("The file should have a content property")?
            .as_str()
            .ok_or("content should be a string")?;

        self.synth.load_str(instruments)?;
        self.sequencer.set_synth_instrument_ids(&self.synth.instrument_ids());

        Ok(())
    }

    fn save_project_as(&mut self) {
        // On some platforms the native dialog needs to be invoked from the
        // main thread, but the state needed to decide whether or not we need
        // to show the dialog is on the sound engine thread.
        // So
        // - The UI asks the sound engine thread to save
        // - The sound engine thread might decide to show the native save dialog from the main thread
        // - Once done, the main thread re-asks the sound engine thread to save at the selected path.
        slint::invoke_from_event_loop(move || {
            // FIXME: Ask for confirmation if the file exists
            let maybe_song_path = FileDialog::new()
                .set_filename("song.ct.md")
                .show_save_single_file()
                .expect("Error showing the save dialog.");
            if let Some(mut song_path) = maybe_song_path {
                if song_path
                    .file_name()
                    .map_or(false, |f| f.to_str().map_or(false, |s| !s.ends_with(".ct.md")))
                {
                    song_path.set_extension("ct.md");
                }
                let mut instruments_path = song_path.clone();
                instruments_path.set_file_name(OsString::from(
                    instruments_path
                        .file_name()
                        .unwrap()
                        .to_str()
                        .expect("Bad path?")
                        .replace(".ct.md", "-instruments.rhai"),
                ));
                crate::invoke_on_sound_engine(move |engine| {
                    move || -> Result<(), Box<dyn Error>> {
                        // Songs shouldn't rely on the default instruments that will vary between versions,
                        // so save a copy of the instruments and make the saved song point to that file.
                        engine.synth.save_as(instruments_path.as_path())?;
                        engine
                            .sequencer
                            .save_as(song_path.as_path(), instruments_path.as_path())?;
                        engine.project_source = ProjectSource::File((song_path, instruments_path));
                        Ok(())
                    }()
                    .unwrap_or_else(|e| elog!("Error saving the project: {}", e))
                });
            }
        })
        .unwrap();
    }

    pub fn save_project(&mut self) {
        match &self.project_source {
            ProjectSource::New => self.save_project_as(),
            ProjectSource::File((song_path, _)) => self
                .sequencer
                .save(song_path.as_path())
                .unwrap_or_else(|e| elog!("Error saving the project: {}", e)),
            ProjectSource::Gist => elog!("Can't save a project loaded from a gist URL."),
        }
    }

    pub fn instruments_path(&self) -> Option<&Path> {
        match &self.project_source {
            ProjectSource::File((_, instruments_path)) => Some(instruments_path.as_path()),
            _ => None,
        }
    }

    pub fn reload_instruments_from_file(&mut self) {
        if let ProjectSource::File((_, path)) = &self.project_source {
            self.synth
                .load_file(&path.as_path())
                .unwrap_or_else(|e| elog!("Couldn't reload instruments from file {:?}.\n\tError: {:?}", path, e));
        }
    }
}
