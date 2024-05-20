// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "gba", no_main)]
#![cfg_attr(feature = "gba", feature(alloc_error_handler, panic_info_message))]

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
mod ui;
mod utils;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "desktop")]
use crate::midi::Midi;
use crate::sound_engine::NUM_INSTRUMENTS;
use crate::sound_engine::NUM_STEPS;
use crate::sound_renderer::new_sound_renderer;
use crate::sound_renderer::SoundRendererTrait;

#[cfg(feature = "desktop")]
use url::Url;

#[cfg(feature = "desktop")]
use alloc::borrow::ToOwned;
#[cfg(feature = "desktop")]
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec;
use core::cell::RefCell;
use slint::ComponentHandle;
use slint::Global;
#[cfg(feature = "desktop")]
use std::env;
#[cfg(feature = "desktop")]
use std::path::PathBuf;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
#[cfg(not(feature = "gba"))]
pub fn main() {
    run_main()
}

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

    let sequencer_step_model = Rc::new(slint::VecModel::<_>::from(vec![ui::StepData::default(); NUM_STEPS]));
    let instruments_model = Rc::new(slint::VecModel::<_>::from(vec![
        ui::InstrumentData::default();
        NUM_INSTRUMENTS
    ]));

    let window = ui::MainWindow::new().unwrap();
    let sound_renderer = Rc::new(RefCell::new(new_sound_renderer(&window)));

    // This is where UI callbacks gets a native handler attached.
    ui::set_window_handlers(&window, sound_renderer.clone());
    ui::set_global_engine_handlers(&window, sound_renderer.clone());
    ui::set_global_ui_handlers(&window);
    ui::set_global_settings_handlers(&window, sound_renderer.clone());
    ui::set_global_utils_handlers(&window);

    let cloned_sound_renderer = sound_renderer.clone();
    #[cfg(feature = "desktop")]
    window.on_animate_waveform(move |tick, width, height| {
        cloned_sound_renderer.borrow_mut().update_waveform(tick, width, height)
    });

    let global_engine = ui::GlobalEngine::get(&window);
    // The model set in the UI are only for development.
    // Rewrite the models and use that version.
    global_engine.set_sequencer_song_patterns(slint::ModelRc::from(Rc::new(slint::VecModel::default())));
    global_engine.set_sequencer_steps(slint::ModelRc::from(sequencer_step_model));
    global_engine.set_instruments(slint::ModelRc::from(instruments_model));
    global_engine.set_synth_trace_notes(slint::ModelRc::from(Rc::new(slint::VecModel::default())));
    global_engine.set_synth_active_notes(slint::ModelRc::from(Rc::new(slint::VecModel::default())));

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
