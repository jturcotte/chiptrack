// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

#[cfg(feature = "gba")]
use crate::gba_platform;
use crate::log;
use crate::sequencer::OnEmpty;
use crate::sequencer::Sequencer;
use crate::sequencer::StepEvent;
#[cfg(feature = "desktop")]
use crate::sound_renderer::emulated::invoke_on_sound_engine;
use crate::sound_renderer::Synth;
use crate::synth_script::SequencerInstrumentDef;
use crate::synth_script::SynthScript;
use crate::ui::GlobalEngine;
use crate::ui::Settings;
use crate::ui::SongSettings;
use crate::utils::WeakWindowWrapper;

#[cfg(feature = "desktop_native")]
use native_dialog::FileDialog;
use slint::Global;
use slint::Model;

use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;
#[cfg(feature = "desktop")]
use std::error::Error;
#[cfg(feature = "desktop")]
use std::path::{Path, PathBuf};

pub const NUM_INSTRUMENTS: usize = 64;
pub const NUM_INSTRUMENT_COLS: usize = 4;
pub const NUM_INSTRUMENT_PARAMS: usize = 2;
pub const NUM_STEPS: usize = 16;
pub const NUM_PATTERNS: usize = 64;

#[derive(PartialEq, Clone, Copy, Debug)]
enum NoteSource {
    Key(u8),
    Sequencer(u8),
}

#[derive(PartialEq)]
enum ProjectSource {
    New,
    #[cfg(feature = "desktop_native")]
    /// Contains the path to the song file and the instruments file
    File((PathBuf, PathBuf)),
    #[cfg(feature = "desktop")]
    /// Contains a copy of the WASM/WAT instrument bytes for exporting
    Gist(Vec<u8>),
    #[cfg(feature = "gba")]
    SRAM,
}

/// This component connects the sequencer, synth and synth scripting together.
/// It sits on the sound thread, and thus also handles some of the UI logic from
/// received messages.
pub struct SoundEngine {
    pub sequencer: Rc<RefCell<Sequencer>>,
    pub synth: Synth,
    script: SynthScript,
    frame_number: usize,
    main_window: WeakWindowWrapper,
    pressed_note: Option<NoteSource>,
    project_source: ProjectSource,
}

impl SoundEngine {
    pub fn new(synth: Synth, main_window: WeakWindowWrapper) -> SoundEngine {
        let sequencer = Self::default_sequencer(&main_window);
        let script = Self::default_synth_script(&synth, &sequencer, &main_window);

        SoundEngine {
            sequencer,
            synth,
            script,
            frame_number: 0,
            main_window,
            pressed_note: None,
            project_source: ProjectSource::New,
        }
    }

    fn default_sequencer(main_window: &WeakWindowWrapper) -> Rc<RefCell<Sequencer>> {
        Rc::new(RefCell::new(Sequencer::new(main_window.clone())))
    }

    fn default_synth_script(
        synth: &Synth,
        sequencer: &Rc<RefCell<Sequencer>>,
        main_window: &WeakWindowWrapper,
    ) -> SynthScript {
        SynthScript::new(
            synth.set_sound_reg_callback(),
            synth.set_wave_table_callback(),
            Self::apply_instrument_ids_callback(sequencer.clone(), main_window.clone()),
        )
    }

    fn apply_instrument_ids_callback(
        sequencer: Rc<RefCell<Sequencer>>,
        main_window: WeakWindowWrapper,
    ) -> impl Fn(SequencerInstrumentDef) {
        move |instrument_def: SequencerInstrumentDef| {
            let ids = instrument_def.ids.clone();
            sequencer
                .borrow_mut()
                .set_instrument_def(instrument_def.ids, instrument_def.params);
            main_window
                .upgrade_in_event_loop(move |handle| {
                    let model = GlobalEngine::get(&handle).get_instruments();
                    for (i, id) in ids.iter().enumerate() {
                        let mut row_data = model.row_data(i).unwrap();
                        row_data.id = id.clone();
                        model.set_row_data(i, row_data);
                    }
                })
                .unwrap();
        }
    }

    pub fn is_ready(&self) -> bool {
        self.sequencer.borrow().received_instruments_ids_after_load
    }

    pub fn apply_settings(&mut self, settings: &Settings) {
        self.synth.apply_settings(settings);
    }

    pub fn apply_song_settings(&self, settings: &SongSettings) {
        self.sequencer.borrow_mut().apply_song_settings(settings);
    }

    fn singularize_note_release(&mut self, source: NoteSource, is_press: bool) -> Option<u8> {
        let note_to_release = if is_press {
            self.pressed_note.replace(source)
        } else {
            match (self.pressed_note, source) {
                // Only release pressed live notes if the released note matches (another one wasn't pressed since)
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

    fn send_note_events_to_synth(&mut self, note_events: Vec<(u8, StepEvent)>) {
        for (instrument, event) in note_events {
            let is_selected_instrument = instrument == self.sequencer.borrow().displayed_instrument;

            let (_note_to_press, _note_to_release) = match event {
                StepEvent::Press(note, p0, p1) => {
                    self.script
                        .press_instrument_note(self.frame_number, instrument, note, p0, p1);
                    let p = Some(note);
                    let r = if is_selected_instrument {
                        self.singularize_note_release(NoteSource::Sequencer(note), true)
                    } else {
                        None
                    };
                    (p, r)
                }
                StepEvent::Release => {
                    let note_to_release = if is_selected_instrument {
                        self.singularize_note_release(NoteSource::Sequencer(0), false)
                    } else {
                        None
                    };
                    if !is_selected_instrument || note_to_release.is_some() {
                        // Only send the sequenced release to the synth if the last of the pressed notes is
                        // being released, or if this isn't the selected instruments (in which case only one
                        // note will be pressed at a time).
                        self.script.release_instrument(self.frame_number, instrument);
                    }
                    (None, note_to_release)
                }
                StepEvent::SetParam(param_num, val) => {
                    self.script.set_instrument_param(instrument, param_num, val);
                    (None, None)
                }
            };

            let pressed = matches!(event, StepEvent::Press(_, _, _));
            self.main_window
                .upgrade_in_event_loop(move |handle| {
                    #[cfg(feature = "desktop")]
                    if is_selected_instrument {
                        let notes_model = handle.get_notes();
                        for row in 0..notes_model.row_count() {
                            let mut row_data = notes_model.row_data(row).unwrap();
                            // A note release might not happen if a press happened in-between.
                            if _note_to_release.map_or(false, |n| n == row_data.note_number as u8) {
                                row_data.active = false;
                                notes_model.set_row_data(row, row_data.clone());
                            }

                            if _note_to_press.map_or(false, |n| n == row_data.note_number as u8) {
                                row_data.active = true;
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

    pub fn set_playing(&mut self, playing: bool, song_mode: bool) {
        self.sequencer.borrow_mut().set_playing(playing, song_mode);
        if !playing {
            self.mute_instruments();
        }
    }

    pub fn advance_frame(&mut self) {
        let (step_change, note_events) = self.sequencer.borrow_mut().advance_frame();

        self.send_note_events_to_synth(note_events);
        self.script.advance_frame(self.frame_number);

        self.synth.advance_frame(self.frame_number, step_change);

        self.frame_number += 1;
    }

    pub fn display_instrument(&mut self, instrument: u8) {
        self.sequencer.borrow_mut().user_display_instrument(instrument);

        self.pressed_note = None;

        // Release all notes visually that might have been pressed for the previous instrument.
        #[cfg(feature = "desktop")]
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

    fn release_note_visually(&mut self, _note: u8) {
        #[cfg(feature = "desktop")]
        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let model = handle.get_notes();
                for row in 0..model.row_count() {
                    let mut row_data = model.row_data(row).unwrap();
                    if _note == row_data.note_number as u8 {
                        row_data.active = false;
                        model.set_row_data(row, row_data);
                    }
                }
            })
            .unwrap();
    }

    pub fn cycle_instrument_param_start(&mut self) {
        let seq = self.sequencer.borrow();
        let note = seq.clipboard_note();
        let [p0, p1] = seq.displayed_instrument_params();
        self.script
            .press_instrument_note(self.frame_number, seq.displayed_instrument, note, p0, p1);
    }
    pub fn cycle_instrument_param_end(&mut self) {
        self.script
            .release_instrument(self.frame_number, self.sequencer.borrow().displayed_instrument);
    }
    pub fn cycle_instrument_param(&mut self, param_num: u8, forward: bool) {
        let mut seq = self.sequencer.borrow_mut();
        let instrument = seq.displayed_instrument;

        if !seq.instrument_has_param_defined(instrument, param_num) {
            // Make sure not to update the param and play the note
            return;
        }

        let ps = seq.cycle_instrument_param(param_num, forward);
        let note = seq.clipboard_note();

        if self.script.instrument_has_set_param_fn(instrument, param_num) {
            // The instrument will get the new value without a press.
            self.script
                .set_instrument_param(instrument, param_num, ps[param_num as usize])
        } else {
            // There is no set param function set by the instrument, trigger a press as feedback like we do in cycle_step_note.
            self.script
                .press_instrument_note(self.frame_number, instrument, note, ps[0], ps[1]);
        }
    }

    pub fn cycle_step_param_start(&mut self, step: usize, param_num: u8) {
        let mut seq = self.sequencer.borrow_mut();
        let (note, p0, p1) = seq.cycle_step_param(step, param_num, None, false, OnEmpty::PasteOnEmpty);
        if !seq.playing() {
            self.script
                .press_instrument_note(self.frame_number, seq.displayed_instrument, note, p0, p1);
        }
    }
    pub fn cycle_step_param_end(&mut self, step: usize, param_num: u8) {
        let mut seq = self.sequencer.borrow_mut();
        seq.set_default_step_params(step, Some(param_num));
        if !seq.playing() {
            // FIXME: Ref-count the press or something to handle +Shift,+Ctrl,-Shift,-Ctrl
            self.script
                .release_instrument(self.frame_number, seq.displayed_instrument);
        }
    }
    pub fn cycle_step_param(&mut self, step: usize, param_num: u8, forward: bool, large_inc: bool) {
        let (note, p0, p1) = self.sequencer.borrow_mut().cycle_step_param(
            step,
            param_num,
            Some(forward),
            large_inc,
            OnEmpty::PasteOnEmpty,
        );
        if !self.sequencer.borrow().playing() {
            let instrument = self.sequencer.borrow().displayed_instrument;

            if self.script.instrument_has_set_param_fn(instrument, param_num) {
                // The instrument will get the new value without a press.
                self.script
                    .set_instrument_param(instrument, param_num, if param_num == 0 { p0 } else { p1 })
            } else {
                // There is no set param function set by the instrument, trigger a press as feedback like we do in cycle_step_note.
                self.script
                    .press_instrument_note(self.frame_number, instrument, note, p0, p1);
            }
        }
    }
    pub fn cycle_step_range_param(
        &mut self,
        step_range_first: usize,
        step_range_last: usize,
        param_num: u8,
        forward: bool,
        large_inc: bool,
    ) {
        debug_assert!(step_range_first <= step_range_last);
        let mut seq = self.sequencer.borrow_mut();
        for step in step_range_first..=step_range_last {
            seq.cycle_step_param(step, param_num, Some(forward), large_inc, OnEmpty::EmptyOnEmpty);
        }
    }

    pub fn cycle_step_note_start(&mut self, step: usize) {
        let (new_note, p0, p1) = self
            .sequencer
            .borrow_mut()
            .cycle_step_note(step, None, false, OnEmpty::PasteOnEmpty);
        if !self.sequencer.borrow().playing() {
            self.script.press_instrument_note(
                self.frame_number,
                self.sequencer.borrow().displayed_instrument,
                new_note,
                p0,
                p1,
            );
        }
    }
    pub fn cycle_step_note_end(&mut self, step: usize) {
        let mut seq = self.sequencer.borrow_mut();
        seq.set_default_step_note(step);
        if !seq.playing() {
            self.script
                .release_instrument(self.frame_number, seq.displayed_instrument);
        }
    }
    pub fn cycle_step_note(&mut self, step: usize, forward: bool, large_inc: bool) {
        let (new_note, p0, p1) =
            self.sequencer
                .borrow_mut()
                .cycle_step_note(step, Some(forward), large_inc, OnEmpty::PasteOnEmpty);
        if !self.sequencer.borrow().playing() {
            self.script.press_instrument_note(
                self.frame_number,
                self.sequencer.borrow().displayed_instrument,
                new_note,
                p0,
                p1,
            );
        }
    }
    pub fn cycle_step_range_note(
        &mut self,
        step_range_first: usize,
        step_range_last: usize,
        forward: bool,
        large_inc: bool,
    ) {
        debug_assert!(step_range_first <= step_range_last);
        let mut seq = self.sequencer.borrow_mut();
        for step in step_range_first..=step_range_last {
            seq.cycle_step_note(step, Some(forward), large_inc, OnEmpty::EmptyOnEmpty);
        }
    }

    pub fn press_note(&mut self, note: u8) {
        let (p0, p1) = self.sequencer.borrow_mut().record_press(note);
        self.script.press_instrument_note(
            self.frame_number,
            self.sequencer.borrow().displayed_instrument,
            note,
            p0,
            p1,
        );

        // Check which not
        if let Some(note_to_release) = self.singularize_note_release(NoteSource::Key(note), true) {
            self.release_note_visually(note_to_release)
        }

        #[cfg(feature = "desktop")]
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

    pub fn release_note(&mut self, note: u8) {
        // Instruments are monophonic, ignore any note release, either sequenced or live,
        // for the current instrument if it wasn't the last pressed one.
        if let Some(note_to_release) = self.singularize_note_release(NoteSource::Key(note), false) {
            self.script
                .release_instrument(self.frame_number, self.sequencer.borrow().displayed_instrument);
            self.sequencer.borrow_mut().record_release(note);
            self.release_note_visually(note_to_release);
        }
    }

    pub fn mute_instruments(&mut self) {
        self.synth.mute_instruments();
        self.script.release_instruments();

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                #[cfg(feature = "desktop")]
                {
                    let notes_model = handle.get_notes();
                    for (i, mut row_data) in notes_model.iter().enumerate() {
                        row_data.active = false;
                        notes_model.set_row_data(i, row_data.clone());
                    }
                }
                let instruments_model = GlobalEngine::get(&handle).get_instruments();
                for (i, mut row_data) in instruments_model.iter().enumerate() {
                    row_data.active = false;
                    instruments_model.set_row_data(i, row_data.clone());
                }
            })
            .unwrap();
    }

    pub fn clear_song_and_load_default_instruments(&mut self) {
        self.sequencer.borrow_mut().clear_song();
        self.mute_instruments();
        self.script.load_default().expect("Default instruments couldn't load");

        self.project_source = ProjectSource::New;
    }

    #[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
    pub fn load_file(&mut self, song_path: &Path) {
        match self.load_file_internal(song_path) {
            Ok(instruments_path) => self.project_source = ProjectSource::File((song_path.to_owned(), instruments_path)),
            Err(err) => {
                elog!("Error extracting project from file [{:?}]: {}", song_path, err);
                self.clear_song_and_load_default_instruments();
            }
        }
    }

    #[cfg(feature = "desktop")]
    pub fn load_gist(&mut self, json: serde_json::Value) {
        match self.load_gist_internal(json) {
            Ok(instruments) => self.project_source = ProjectSource::Gist(instruments),
            Err(err) => {
                elog!("Error extracting project from gist: {}", err);
                self.clear_song_and_load_default_instruments();
            }
        }
    }

    #[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
    fn load_file_internal(&mut self, song_path: &Path) -> Result<PathBuf, Box<dyn Error>> {
        log!("Loading the project song from file {:?}", song_path);
        let instruments_file = self.sequencer.borrow_mut().load_file(song_path)?;
        let instruments_path = song_path.with_file_name(instruments_file);
        log!("Loading project instruments from file {:?}", instruments_path);
        self.mute_instruments();
        self.script.load_file(instruments_path.as_path())?;
        Ok(instruments_path)
    }

    #[cfg(feature = "desktop")]
    fn load_gist_internal(&mut self, json: serde_json::Value) -> Result<Vec<u8>, Box<dyn Error>> {
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
        let instruments_file = self.sequencer.borrow_mut().load_str(song)?;

        let instruments = files
            .get(&instruments_file)
            .ok_or_else(|| format!("The gist should have a file named {}", instruments_file))?
            .get("content")
            .ok_or("The file should have a content property")?
            .as_str()
            .ok_or("content should be a string")?
            .as_bytes()
            .to_vec();

        self.mute_instruments();
        self.script.load_wasm_or_wat_bytes(&instruments)?;

        Ok(instruments)
    }

    #[cfg(feature = "desktop_native")]
    pub fn save_project_as(&mut self) {
        // On some platforms the native dialog needs to be invoked from the
        // main thread, but the state needed to decide whether or not we need
        // to show the dialog (e.g. save for new file) is on the sound engine thread.
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
                let song_dir = song_path.parent().expect("Not an absolute path?").to_path_buf();

                invoke_on_sound_engine(move |engine| {
                    move || -> Result<(), Box<dyn Error>> {
                        // Songs shouldn't rely on the default instruments that will vary between versions,
                        // so save a copy of the instruments and make the saved song point to that file.
                        let instruments_path = engine.script.save_default_instruments_as(song_dir.as_path())?;
                        engine
                            .sequencer
                            .borrow_mut()
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
    #[cfg(not(feature = "desktop_native"))]
    pub fn save_project_as(&mut self) {}

    pub fn save_project(&mut self) {
        #[cfg(feature = "gba")]
        self.save_song_to_gba_sram();

        #[cfg(feature = "desktop")]
        match &self.project_source {
            ProjectSource::New => self.save_project_as(),
            #[cfg(feature = "desktop_native")]
            ProjectSource::File((song_path, _)) => self
                .sequencer
                .borrow()
                .save(song_path.as_path())
                .unwrap_or_else(|e| elog!("Error saving the project: {}", e)),
            ProjectSource::Gist(_) => elog!("Can't save a project loaded from a gist URL."),
        }
    }

    pub fn export_project_as_gba_sav(&self) {
        #[cfg(feature = "desktop_native")]
        || -> Result<(), Box<dyn Error>> {
            // TODO: Show a save as dialog.
            let p = Path::new("chiptrack.sav");
            let instruments = self.instruments_bytes();
            let instruments_wasm = wat::parse_bytes(&instruments).map_err(|e| e.to_string())?;
            let song = self.sequencer.borrow().serialize_to_postcard()?;
            println!(
                "Saving project song to file {:?}, instruments: {} bytes, song: {} bytes.",
                p,
                instruments_wasm.len(),
                song.len()
            );
            let mut full = Vec::new();

            // TODO: Support flash ROM in load_gba_sram to get access to 64kb or 128kb saves
            if 8 + instruments_wasm.len() + song.len() >= 32 * 1024 {
                return Err(format!(
                    "SRAM save games currently only support max 32kb but the song is {} bytes.",
                    8 + instruments_wasm.len() + song.len()
                )
                .into());
            }

            full.extend_from_slice(&(instruments_wasm.len() as u32).to_le_bytes());
            full.extend_from_slice(&(song.len() as u32).to_le_bytes());
            full.extend(instruments_wasm.iter());
            full.extend(song);
            std::fs::write(p, full)?;
            Ok(())
        }()
        .unwrap_or_else(|e| elog!("Error exporting the project: {}", e))
    }

    #[cfg(feature = "gba")]
    fn save_song_to_gba_sram(&mut self) {
        unsafe {
            let mut buf = [0u8; 4];
            let sram = 0x0E00_0000 as *mut u8;
            gba::mem_fns::__aeabi_memcpy1(buf.as_mut_ptr(), sram, 4);
            let mut instruments_len = u32::from_le_bytes(buf) as usize;
            if instruments_len == 0xffffffff || self.project_source == ProjectSource::New {
                // SRAM is empty, copy the default instruments from ROM.
                instruments_len = SynthScript::DEFAULT_INSTRUMENTS.len();
                buf = (instruments_len as u32).to_le_bytes();
                gba::mem_fns::__aeabi_memcpy1(sram, buf.as_ptr(), 4);
                gba::mem_fns::__aeabi_memcpy1(
                    sram.offset(8),
                    SynthScript::DEFAULT_INSTRUMENTS.as_ptr(),
                    instruments_len,
                );
            }
            match self.sequencer.borrow().serialize_to_postcard() {
                Ok(song_bytes) => {
                    buf = (song_bytes.len() as u32).to_le_bytes();
                    gba::mem_fns::__aeabi_memcpy1(sram.offset(4), buf.as_ptr(), 4);
                    gba::mem_fns::__aeabi_memcpy1(
                        sram.offset(8 + instruments_len as isize),
                        song_bytes.as_ptr(),
                        song_bytes.len(),
                    );
                    let song_len = u32::from_le_bytes(buf) as usize;
                    gba_platform::renderer::draw_menu_status_text(&alloc::format!("Saved {}B song to SRAM.", song_len));
                }
                Err(e) => elog!("save error: {}", e),
            }
            self.project_source = ProjectSource::SRAM;
        }
    }

    #[cfg(feature = "gba")]
    pub fn load_gba_sram(&mut self) -> Option<()> {
        unsafe {
            // 4 bytes: instruments_len
            // 4 bytes: song_len
            // instruments_len bytes: instruments
            // song_len bytes: song
            let mut buf = [0u8; 4];
            let sram = 0x0E00_0000 as *mut u8;
            gba::mem_fns::__aeabi_memcpy1(buf.as_mut_ptr(), sram, 4);
            let instruments_len = u32::from_le_bytes(buf) as usize;
            if instruments_len == 0xffffffff {
                // SRAM is empty
                return None;
            }
            gba::mem_fns::__aeabi_memcpy1(buf.as_mut_ptr(), sram.offset(4), 4);
            let song_len = u32::from_le_bytes(buf) as usize;
            log!(
                "Loading song ({} bytes) and instruments ({} bytes) from SRAM.",
                song_len,
                instruments_len
            );

            {
                let mut song_bytes = Vec::<u8>::with_capacity(song_len);
                gba::mem_fns::__aeabi_memcpy1(
                    song_bytes.as_mut_ptr(),
                    sram.offset(8 + instruments_len as isize),
                    song_len,
                );
                song_bytes.set_len(song_len);
                self.sequencer.borrow_mut().load_postcard_bytes(&song_bytes).unwrap();
            }

            let mut instrument_bytes = Vec::<u8>::with_capacity(instruments_len);
            gba::mem_fns::__aeabi_memcpy1(instrument_bytes.as_mut_ptr(), sram.offset(8), instruments_len);
            instrument_bytes.set_len(instruments_len);
            if let Err(e) = self.script.load_bytes(instrument_bytes) {
                elog!("load error: {}", e);
            }

            self.project_source = ProjectSource::SRAM;
            Some(())
        }
    }

    #[cfg(feature = "desktop_native")]
    pub fn instruments_path(&self) -> Option<&Path> {
        match &self.project_source {
            ProjectSource::File((_, instruments_path)) => Some(instruments_path.as_path()),
            _ => None,
        }
    }

    #[cfg(feature = "desktop_native")]
    fn instruments_bytes(&self) -> Vec<u8> {
        match &self.project_source {
            ProjectSource::New => SynthScript::DEFAULT_INSTRUMENTS_TEXT.to_vec(),
            ProjectSource::File((_, instruments_path)) => {
                std::fs::read(instruments_path).expect("Error reading instruments file")
            }
            ProjectSource::Gist(instruments) => instruments.clone(),
        }
    }

    #[cfg(feature = "desktop_native")]
    pub fn reload_instruments_from_file(&mut self) {
        if let ProjectSource::File((_, path)) = &self.project_source {
            if let Err(e) = self.script.load_file(path.as_path()) {
                elog!("Couldn't reload instruments from file {:?}.\n\tError: {:?}", path, e);
            }
        }
    }
}
