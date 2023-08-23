// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "gba", no_main)]
#![cfg_attr(
    feature = "gba",
    feature(error_in_core, alloc_error_handler, start, core_intrinsics, lang_items, link_cfg)
)]

extern crate alloc;

#[cfg(feature = "gba")]
mod gba_platform;
mod log;
#[cfg(feature = "desktop")]
mod midi;
mod sequencer;
mod sound_engine;
mod sound_renderer;
mod synth_script;
mod utils;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "desktop")]
use crate::midi::Midi;
use crate::sound_engine::NUM_INSTRUMENTS;
use crate::sound_engine::NUM_STEPS;
use crate::sound_renderer::new_sound_renderer;
use crate::utils::MidiNote;

use slint::Model;
use slint::{Timer, TimerMode};
#[cfg(feature = "desktop")]
use url::Url;

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::time::Duration;
#[cfg(feature = "desktop")]
use std::env;
#[cfg(feature = "desktop")]
use std::path::PathBuf;

slint::include_modules!();

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
#[cfg(not(feature = "gba"))]
pub fn main() {
    run_main()
}

// FIXME: Can it be moved?
#[cfg(feature = "gba")]
#[no_mangle]
extern "C" fn main() -> ! {
    gba_platform::init();
    run_main();

    panic!("Should not return")
}

fn run_main() {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(all(debug_assertions, target_arch = "wasm32"))]
    console_error_panic_hook::set_once();

    #[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
    let (maybe_file_path, maybe_gist_path) = match env::args().nth(1).map(|u| Url::parse(&u)) {
        None => (None, None),
        Some(Ok(url)) => {
            if url.host_str().map_or(false, |h| h == "gist.github.com") {
                (None, Some(url.path().trim_start_matches('/').to_owned()))
            } else {
                elog!(
                    "Found a URL parameter but it wasn't for gist.github.com: {:?}",
                    url.to_string()
                );
                (None, None)
            }
        }
        Some(Err(_)) =>
        // This isn't a URL, assume it's a file path.
        {
            let song_path = PathBuf::from(env::args().nth(1).unwrap());
            if !song_path.exists() {
                elog!("Error: the provided song path doesn't exist [{:?}]", song_path);
                std::process::exit(1);
            }
            (Some(song_path), None)
        }
    };
    #[cfg(target_arch = "wasm32")]
    let (maybe_file_path, maybe_gist_path) = {
        let window = web_sys::window().unwrap();
        let query_string = window.location().search().unwrap();
        let search_params = web_sys::UrlSearchParams::new_with_str(&query_string).unwrap();

        (None::<PathBuf>, search_params.get("gist"))
    };

    let sequencer_step_model = Rc::new(slint::VecModel::<_>::from(vec![StepData::default(); NUM_STEPS]));
    let instruments_model = Rc::new(slint::VecModel::<_>::from(vec![
        InstrumentData::default();
        NUM_INSTRUMENTS
    ]));

    let window = MainWindow::new().unwrap();

    #[cfg(feature = "desktop")]
    {
        let note_model = Rc::new(slint::VecModel::default());
        let start: i32 = 60;
        let start_octave: i32 = MidiNote(start).octave();
        let notes: Vec<NoteData> = (start..(start + 13))
            .map(|i| {
                let note = MidiNote(i);
                let pos = note.key_pos() + (note.octave() - start_octave) * 7;
                NoteData {
                    note_number: i,
                    key_pos: pos,
                    is_black: note.is_black(),
                    active: false,
                }
            })
            .collect();
        for n in notes.iter().filter(|n| !n.is_black) {
            note_model.push(n.clone());
        }
        // Push the black notes at the end of the model so that they appear on top of the white ones.
        for n in notes.iter().filter(|n| n.is_black) {
            note_model.push(n.clone());
        }

        window.set_notes(slint::ModelRc::from(note_model.clone()));
    }

    #[cfg(target_arch = "wasm32")]
    if !web_sys::window().unwrap().location().search().unwrap().is_empty() {
        // Show the UI directly in song mode if the URL might contain a song.
        GlobalUI::get(&window).set_song_mode(true);
    }

    let global_engine = GlobalEngine::get(&window);
    // The model set in the UI are only for development.
    // Rewrite the models and use that version.
    global_engine.set_sequencer_song_patterns(slint::ModelRc::from(Rc::new(slint::VecModel::default())));
    global_engine.set_sequencer_steps(slint::ModelRc::from(sequencer_step_model));
    global_engine.set_instruments(slint::ModelRc::from(instruments_model));
    global_engine.set_synth_trace_notes(slint::ModelRc::from(Rc::new(slint::VecModel::default())));
    global_engine.set_synth_active_notes(slint::ModelRc::from(Rc::new(slint::VecModel::default())));

    let sound_renderer = Rc::new(RefCell::new(new_sound_renderer(&window)));

    #[cfg(feature = "desktop")]
    if let Some(gist_path) = maybe_gist_path {
        let api_url = "https://api.github.com/gists/".to_owned() + gist_path.splitn(2, '/').last().unwrap();
        log!("Loading the project from gist API URL {}", api_url.to_string());
        let cloned_sound_send = sound_renderer.borrow().sender();
        ehttp::fetch(
            ehttp::Request::get(&api_url),
            move |result: ehttp::Result<ehttp::Response>| {
                result
                    .and_then(|res| {
                        if res.ok {
                            let decoded: serde_json::Value =
                                serde_json::from_slice(&res.bytes).expect("JSON was not well-formatted");
                            cloned_sound_send
                                .send(Box::new(move |se| se.load_gist(decoded)))
                                .unwrap();
                            Ok(())
                        } else {
                            Err(format!("{} - {}", res.status, res.status_text))
                        }
                    })
                    .unwrap_or_else(|err| {
                        elog!(
                            "Error fetching the project from {}: {}. Exiting.",
                            api_url.to_string(),
                            err
                        );
                        std::process::exit(1);
                    });
            },
        );
    } else if let Some(file_path) = maybe_file_path {
        #[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
        sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine_no_force(move |se| se.load_file(&file_path));
    } else {
        sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine_no_force(|se| se.load_default());
    }
    #[cfg(feature = "gba")]
    sound_renderer
        .borrow_mut()
        .invoke_on_sound_engine(|se| se.load_gba_sram().unwrap_or_else(|| se.load_default()));

    // The midir web backend needs to be asynchronously initialized, but midir doesn't tell
    // us when that initialization is done and that we can start querying the list of midi
    // devices. It's also annoying for users that don't care about MIDI to get a permission
    // request, so I'll need this to be enabled explicitly for the Web version.
    // The audio latency is still so bad with the web version though,
    // so I'm not sure if that's really worth it.
    #[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
    let _midi = {
        let cloned_sound_renderer = sound_renderer.borrow().sender();
        let cloned_sound_renderer2 = sound_renderer.borrow().sender();
        let press = move |key| {
            cloned_sound_renderer2
                .send(Box::new(move |se| se.press_note(key)))
                .unwrap()
        };
        let release = move |key| {
            cloned_sound_renderer
                .send(Box::new(move |se| se.release_note(key)))
                .unwrap();
        };
        Some(Midi::new(press, release))
    };

    let _window_weak = window.as_weak();
    #[cfg(feature = "desktop")]
    window.on_octave_increased(move |octave_delta| {
        let window = _window_weak.clone().upgrade().unwrap();
        let first_note = window.get_first_note();
        if first_note <= 24 && octave_delta < 0 || first_note >= 96 && octave_delta > 0 {
            return;
        }
        window.set_first_note(first_note + octave_delta * 12);
        let model = window.get_notes();
        for row in 0..model.row_count() {
            let mut row_data = model.row_data(row).unwrap();
            row_data.note_number += octave_delta * 12;
            // The note_number changed and thus the sequencer release events
            // won't see that note anymore, so release it already while we're here here.
            row_data.active = false;
            model.set_row_data(row, row_data);
        }
    });

    // KeyEvent doesn't expose yet whether a press event is due to auto-repeat.
    // Do the deduplication natively until such an API exists.
    let mut already_pressed = Vec::new();
    let cloned_sound_renderer = sound_renderer.clone();
    window.on_global_key_event(move |text, pressed| {
        if let Some(code) = text.as_str().chars().next() {
            if pressed {
                if !already_pressed.contains(&code) {
                    already_pressed.push(code.to_owned());
                    if code == '\u{8}' {
                        cloned_sound_renderer
                            .borrow_mut()
                            .invoke_on_sound_engine(|se| se.sequencer.borrow_mut().set_erasing(true));
                        return true;
                    }
                }
            } else {
                if let Some(index) = already_pressed.iter().position(|x| *x == code) {
                    already_pressed.swap_remove(index);
                }
                if code == '\u{8}' {
                    cloned_sound_renderer
                        .borrow_mut()
                        .invoke_on_sound_engine(|se| se.sequencer.borrow_mut().set_erasing(false));
                    return true;
                };
            }
        }
        false
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_select_instrument(move |instrument| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.select_instrument(instrument as u8));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_instrument(move |col_delta, row_delta| {
        // FIXME: This might need to go through the SoundEngine as with select_instrument.
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().cycle_instrument(col_delta, row_delta));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_pattern_instrument(move |forward| {
        // FIXME: This might need to go through the SoundEngine as with select_instrument.
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().cycle_pattern_instrument(forward));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_note_start(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_note_start());
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_note_end(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_note_end());
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_note(move |forward, large_inc| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_note(forward, large_inc));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_instrument_param_start(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_instrument_param_start());
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_instrument_param_end(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_instrument_param_end());
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_instrument_param(move |param_num, forward| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_instrument_param(param_num as u8, forward));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_step_param_start(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_step_param_start());
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_step_param_end(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_step_param_end());
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_step_param(move |param_num, forward| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_step_param(param_num as u8, forward));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_song_pattern_start(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().cycle_song_pattern_start());
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_song_pattern(move |forward| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().cycle_song_pattern(forward));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_toggle_mute_instrument(move |instrument| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().toggle_mute_instrument(instrument as u8));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_note_pressed(move |note| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.press_note(note as u8));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_note_released(move |note| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.release_note(note as u8));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    let key_release_timer: Timer = Default::default();
    global_engine.on_note_key_pressed(move |note| {
        let cloned_sound_renderer2 = cloned_sound_renderer.clone();
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.press_note(note as u8));

        // We have only one timer for direct interactions, and we don't handle
        // keys being held or even multiple keys at time yet, so just visually release all notes.
        key_release_timer.start(
            TimerMode::SingleShot,
            Duration::from_millis(15 * 6),
            Box::new(move || {
                cloned_sound_renderer2
                    .borrow_mut()
                    .invoke_on_sound_engine(move |se| se.release_note(note as u8));
            }),
        );
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_toggle_step(move |step_num| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().toggle_step(step_num as usize));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_toggle_selected_step(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().toggle_selected_step());
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_toggle_step_release(move |step_num| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().toggle_step_release(step_num as usize));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_toggle_selected_step_release(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().toggle_selected_step_release());
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_select_next_step(move |forward| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().select_next_step(forward));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    let window_weak = window.as_weak();
    global_engine.on_play_clicked(move |toggled| {
        // FIXME: Stop the sound device
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.set_playing(toggled));
        window_weak.unwrap().set_playing(toggled);
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_record_clicked(move |toggled| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().set_recording(toggled));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_select_next_song_pattern(move |forward| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().select_next_song_pattern(forward));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_song_pattern_clicked(move |song_pattern_idx| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().select_song_pattern(song_pattern_idx as usize));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_remove_last_song_pattern(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(|se| se.sequencer.borrow_mut().remove_last_song_pattern());
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_save_project(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(|se| se.save_project())
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_export_project_as_gba_sav(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(|se| se.export_project_as_gba_sav())
    });

    let window_weak = window.as_weak();
    let mut previous_phasing = None;
    global_engine.on_phase_visualization_tick(move |animation_tick| {
        let phasing = match previous_phasing {
            None => {
                let window = window_weak.clone().upgrade().unwrap();
                let synth_trace_notes = GlobalEngine::get(&window).get_synth_trace_notes();
                if synth_trace_notes.row_count() == 0 {
                    // Wait until the first synth_trace_notes entry to lock the phasing.
                    0f32
                } else {
                    let first_synth_tick = synth_trace_notes.row_data(0).unwrap().start_tick;
                    let phasing = first_synth_tick as f32 - animation_tick;
                    previous_phasing = Some(phasing);
                    phasing
                }
            }
            Some(phasing) => phasing,
        };
        animation_tick + phasing
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_mute_instruments(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(|se| se.synth.mute_instruments());
    });

    let cloned_sound_renderer = sound_renderer.clone();
    window.global::<GlobalSettings>().on_settings_changed(move |settings| {
        log!("SET {:?}", settings);
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.apply_settings(&settings));
    });
    let cloned_sound_renderer = sound_renderer.clone();
    window
        .global::<GlobalSettings>()
        .on_song_settings_changed(move |settings| {
            log!("SET {:?}", settings);
            cloned_sound_renderer
                .borrow_mut()
                .invoke_on_sound_engine(move |se| se.apply_song_settings(&settings));
        });

    window
        .global::<GlobalUtils>()
        .on_get_midi_note_name(|note| MidiNote(note).name().into());
    window
        .global::<GlobalUtils>()
        .on_get_midi_note_short_name(|note| MidiNote(note).short_name());

    // For WASM we need to wait for the user to trigger the creation of the sound
    // device through an input event. For other platforms, artificially force the
    // lazy context immediately.
    #[cfg(not(target_arch = "wasm32"))]
    sound_renderer.borrow_mut().force();

    #[cfg(feature = "gba")]
    gba_platform::set_sound_renderer(sound_renderer);
    #[cfg(feature = "gba")]
    gba_platform::set_main_window(window.as_weak());

    window.run().unwrap();
}
