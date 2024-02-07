// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "gba", no_main)]
#![cfg_attr(feature = "gba", feature(alloc_error_handler))]

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

#[cfg(feature = "desktop")]
use slint::Model;
#[cfg(feature = "desktop")]
use url::Url;

#[cfg(feature = "desktop")]
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec;
use core::cell::RefCell;
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

    let global_engine = GlobalEngine::get(&window);
    // The model set in the UI are only for development.
    // Rewrite the models and use that version.
    global_engine.set_sequencer_song_patterns(slint::ModelRc::from(Rc::new(slint::VecModel::default())));
    global_engine.set_sequencer_steps(slint::ModelRc::from(sequencer_step_model));
    global_engine.set_instruments(slint::ModelRc::from(instruments_model));
    global_engine.set_synth_trace_notes(slint::ModelRc::from(Rc::new(slint::VecModel::default())));
    global_engine.set_synth_active_notes(slint::ModelRc::from(Rc::new(slint::VecModel::default())));

    let sound_renderer = Rc::new(RefCell::new(new_sound_renderer(&window)));
    #[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
    if let Some(ref file_path) = maybe_file_path {
        // FIXME: Update it when saving as.
        sound_renderer.borrow_mut().set_song_path(file_path.to_path_buf());
    }

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

    #[cfg(feature = "gba")]
    {
        let cloned_sound_renderer = sound_renderer.clone();
        window.on_save_to_sram(move || {
            cloned_sound_renderer
                .borrow_mut()
                .invoke_on_sound_engine(move |se| se.save_song_to_gba_sram());
        });

        window.on_clear_status_text(move || {
            gba_platform::clear_status_text();
        });
    }
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
            // The note_number changed and thus the sequencer release events
            // won't see that note anymore, so release it already while we're here here.
            row_data.active = false;
            model.set_row_data(row, row_data);
        }
    });

    let cloned_sound_renderer = sound_renderer.clone();
    #[cfg(feature = "desktop")]
    window.on_animate_waveform(move |tick, width, height| {
        cloned_sound_renderer.borrow_mut().update_waveform(tick, width, height)
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
    global_engine.on_cycle_step_param_start(move |param_num| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_step_param_start(param_num as u8));
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_step_param_end(move |param_num| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_step_param_end(param_num as u8));
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_step_param(move |param_num, forward| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_step_param(param_num as u8, forward));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_song_pattern_start_with_new(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().cycle_song_pattern_start_with_new());
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
    global_engine.on_set_erasing(move |v| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().set_erasing(v));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_toggle_step(move |step_num| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().toggle_step(step_num as usize));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cut_selected_step_note(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().cut_selected_step_note());
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cut_selected_step_param(move |param_num| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().cut_selected_step_param(param_num as u8));
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

    #[cfg(feature = "desktop")]
    {
        let cloned_sound_renderer = sound_renderer.clone();
        let window_weak = window.as_weak();
        global_engine.on_record_clicked(move |toggled| {
            cloned_sound_renderer
                .borrow_mut()
                .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().set_recording(toggled));
            window_weak.unwrap().set_recording(toggled);
        });
    }

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_select_next_song_pattern(move |forward| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().select_next_song_pattern(forward));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_select_song_pattern(move |song_pattern_idx| {
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
    global_engine.on_clone_selected_song_pattern(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(|se| se.sequencer.borrow_mut().clone_selected_song_pattern());
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
    let mut maybe_previous_phasing: Option<f32> = None;
    #[cfg(feature = "desktop")]
    global_engine.on_phase_visualization_tick(move |animation_ms| {
        // 4194304 Hz / 70224 Hz per frame = ~59.7 frames per second
        let animation_synth_tick = animation_ms * (4194304.0 / 70224.0) / 1000.0;
        let window = window_weak.clone().upgrade().unwrap();
        let last_synth_tick = GlobalEngine::get(&window).get_last_synth_tick();
        let cur_phasing = last_synth_tick as f32 - animation_synth_tick;

        // The phasing will fluctuate depending on the timing of the sound and rendering loops,
        // which would cause non-smooth scrolling if we'd rely on it for timing, so fix the phasing.
        // But the refresh rates are slightly different, and thus the two drift apart slowly by one
        // frame every few minutes, so we also need to slowly bring them back together from time to
        // time to keep the timelines in sync.
        let phasing = match maybe_previous_phasing {
            // If the rendering and sound synthesis diverged too much, snap them back
            Some(previous_phasing) if (previous_phasing - cur_phasing).abs() > 60.0 => {
                maybe_previous_phasing = Some(cur_phasing);
                cur_phasing
            }
            // If they diverged a little, bring them back little by little
            Some(previous_phasing) if previous_phasing - cur_phasing > 2.0 => {
                let new_phasing = previous_phasing - 0.5;
                maybe_previous_phasing = Some(new_phasing);
                new_phasing
            }
            Some(previous_phasing) if cur_phasing - previous_phasing > 2.0 => {
                let new_phasing = previous_phasing + 0.5;
                maybe_previous_phasing = Some(new_phasing);
                new_phasing
            }
            // If it only diverged within the fluctuation range, keep the current phasing.
            Some(previous_phasing) => previous_phasing,
            // First time
            None => {
                maybe_previous_phasing = Some(cur_phasing);
                cur_phasing
            }
        };

        animation_synth_tick + phasing
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_mute_instruments(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(|se| se.mute_instruments());
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
