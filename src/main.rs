// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

mod log;
mod midi;
mod sequencer;
mod sound_engine;
mod synth;
mod synth_script;
mod utils;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::midi::Midi;
use crate::sound_engine::SoundEngine;
use crate::sound_engine::NUM_INSTRUMENTS;
use crate::sound_engine::NUM_PATTERNS;
use crate::sound_engine::NUM_STEPS;
use crate::utils::MidiNote;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat};
use notify::{DebouncedEvent, RecursiveMode, Watcher};
use once_cell::unsync::Lazy;
use slint::{Model, Rgba8Pixel, SharedPixelBuffer, Timer, TimerMode};
use tiny_skia::*;
use url::Url;

use std::cell::RefCell;
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

slint::include_modules!();

thread_local! {static SOUND_ENGINE: RefCell<Option<SoundEngine>> = RefCell::new(None);}
thread_local! {static SOUND_SENDER: RefCell<Option<std::sync::mpsc::Sender<Box<dyn FnOnce(&mut SoundEngine) + Send>>>> = RefCell::new(None);}

pub fn invoke_on_sound_engine<F>(f: F)
where
    F: FnOnce(&mut SoundEngine) + Send + 'static,
{
    SOUND_SENDER
        .with(|s| {
            s.borrow_mut()
                .as_ref()
                .expect("Should be initialized")
                .send(Box::new(f))
        })
        .unwrap();
}

fn update_waveform(window: &MainWindow, samples: Vec<f32>, consumed: Arc<AtomicBool>) {
    let was_non_zero = !window.get_waveform_is_zero();
    let res_divider = 2.;

    // Already let the audio thread know that it can send us a new waveform.
    consumed.store(true, Ordering::Relaxed);

    let width = window.get_waveform_width() / res_divider;
    let height = window.get_waveform_height() / res_divider;
    let mut pb = PathBuilder::new();
    let mut non_zero = false;
    {
        for (i, source) in samples.iter().enumerate() {
            if *source != 0.0 {
                non_zero = true;
            }
            // Input samples are in the range [-1.0, 1.0].
            // The gameboy emulator mixer however just use a gain of 0.25
            // per channel to avoid clipping when all channels are playing.
            // So multiply by 2.0 to amplify the visualization of single
            // channels a bit.
            let x = i as f32 * width / samples.len() as f32;
            let y = (source * 2.0 + 1.0) * height / 2.0;
            if i == 0 {
                pb.move_to(x, y);
            } else {
                pb.line_to(x, y);
            }
        }
    }
    // Painting this takes a lot of CPU since we need to paint, clone
    // the whole pixmap buffer, and changing the image will trigger a
    // repaint of the full viewport.
    // So at least avoig eating CPU while no sound is being output.
    if non_zero || was_non_zero {
        if let Some(path) = pb.finish() {
            let mut pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::new(width as u32, height as u32);
            if let Some(mut pixmap) = PixmapMut::from_bytes(pixel_buffer.make_mut_bytes(), width as u32, height as u32)
            {
                pixmap.fill(tiny_skia::Color::TRANSPARENT);
                let mut paint = Paint::default();
                paint.blend_mode = BlendMode::Source;
                // #a0a0a0
                paint.set_color_rgba8(160, 160, 160, 255);

                let mut stroke = Stroke::default();
                // Use hairline stroking, faster.
                stroke.width = 0.0;
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);

                let image = slint::Image::from_rgba8_premultiplied(pixel_buffer);
                window.set_waveform_image(image);
                window.set_waveform_is_zero(!non_zero);
            }
        }
    }
}

fn check_if_project_changed(notify_recv: &mpsc::Receiver<DebouncedEvent>, engine: &mut SoundEngine) -> () {
    while let Ok(msg) = notify_recv.try_recv() {
        let reload = if let Some(instruments_path) = engine.instruments_path() {
            let instruments = instruments_path.file_name();
            match msg {
                DebouncedEvent::Write(path) if path.file_name() == instruments => true,
                DebouncedEvent::Create(path) if path.file_name() == instruments => true,
                DebouncedEvent::Remove(path) if path.file_name() == instruments => true,
                DebouncedEvent::Rename(from, to)
                    if from.file_name() == instruments || to.file_name() == instruments =>
                {
                    true
                }
                _ => false,
            }
        } else {
            false
        };
        if reload {
            engine.reload_instruments_from_file();
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn main() {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(all(debug_assertions, target_arch = "wasm32"))]
    console_error_panic_hook::set_once();

    #[cfg(not(target_arch = "wasm32"))]
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

        (None, search_params.get("gist"))
    };

    let sequencer_pattern_model = Rc::new(slint::VecModel::<_>::from(vec![
        PatternData {
            empty: true,
            active: false,
        };
        NUM_PATTERNS
    ]));
    let sequencer_step_model = Rc::new(slint::VecModel::<_>::from(vec![StepData::default(); NUM_STEPS]));
    let instruments_model = Rc::new(slint::VecModel::<_>::from(vec![
        InstrumentData::default();
        NUM_INSTRUMENTS
    ]));
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

    struct Context {
        key_release_timer: Timer,
        _stream: cpal::Stream,
        _midi: Option<Midi>,
    }

    let window = MainWindow::new();
    window.set_notes(slint::ModelRc::from(note_model.clone()));

    #[cfg(target_arch = "wasm32")]
    if !web_sys::window().unwrap().location().search().unwrap().is_empty() {
        // Show the UI directly in song mode if the URL might contain a song.
        window.set_in_song_mode(true);
    }

    let global_engine = GlobalEngine::get(&window);
    // The model set in the UI are only for development.
    // Rewrite the models and use that version.
    global_engine.set_sequencer_song_patterns(slint::ModelRc::from(Rc::new(slint::VecModel::default())));
    global_engine.set_sequencer_patterns(slint::ModelRc::from(sequencer_pattern_model));
    global_engine.set_sequencer_steps(slint::ModelRc::from(sequencer_step_model));
    global_engine.set_instruments(slint::ModelRc::from(instruments_model));
    global_engine.set_synth_trace_notes(slint::ModelRc::from(Rc::new(slint::VecModel::default())));
    global_engine.set_synth_active_notes(slint::ModelRc::from(Rc::new(slint::VecModel::default())));

    let (sound_send, sound_recv) = mpsc::channel::<Box<dyn FnOnce(&mut SoundEngine) + Send>>();
    let (notify_send, notify_recv) = mpsc::channel();

    let cloned_sound_send = sound_send.clone();
    SOUND_SENDER.with(|s| *s.borrow_mut() = Some(cloned_sound_send));

    let cloned_sound_send = sound_send.clone();
    if let Some(gist_path) = maybe_gist_path {
        let api_url = "https://api.github.com/gists/".to_owned() + gist_path.splitn(2, '/').last().unwrap();
        log!("Loading the project from gist API URL {}", api_url.to_string());
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
        cloned_sound_send
            .send(Box::new(move |se| se.load_file(&file_path)))
            .unwrap();
    } else {
        cloned_sound_send.send(Box::new(|se| se.load_default())).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    let mut watcher = notify::watcher(notify_send, Duration::from_millis(500)).unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    // FIXME: Watch the song file's folder, and update it when saving as.
    watcher.watch(".", RecursiveMode::NonRecursive).unwrap();

    let window_weak = window.as_weak();
    let initial_settings = window.global::<GlobalSettings>().get_settings();
    let cloned_sound_send = sound_send.clone();
    let context = Rc::new(Lazy::new(|| {
        let host = cpal::default_host();
        let device = host.default_output_device().unwrap();
        log!("Open the audio player: {}", device.name().unwrap());
        let config = device.default_output_config().unwrap();
        log!("Audio format {:?}", config);
        let sample_rate = config.sample_rate().0;

        let err_fn = |err| elog!("an error occurred on the output audio stream: {}", err);
        let sample_format = config.sample_format();

        // The sequencer won't produce anything faster than every 1/60th second,
        // so a buffer roughly the size of a frame should work fine for now.
        #[cfg(not(target_arch = "wasm32"))]
        let wanted_buffer_size = 512;
        // Everything happens on the same thread in wasm32, and is a bit slower,
        // so increase the buffer size there.
        #[cfg(target_arch = "wasm32")]
        let wanted_buffer_size = 2048;

        let buffer_size = match config.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => wanted_buffer_size.min(*max).max(*min),
            cpal::SupportedBufferSize::Unknown => wanted_buffer_size,
        };

        let mut stream_config: cpal::StreamConfig = config.into();
        stream_config.buffer_size = cpal::BufferSize::Fixed(buffer_size);

        let last_waveform_consumed = Arc::new(AtomicBool::new(true));

        let stream = match sample_format {
            SampleFormat::F32 => device.build_output_stream(
                &stream_config,
                move |dest: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    SOUND_ENGINE.with(|maybe_engine_cell| {
                        let mut maybe_engine = maybe_engine_cell.borrow_mut();
                        if let None = *maybe_engine {
                            *maybe_engine = Some(SoundEngine::new(
                                sample_rate,
                                window_weak.clone(),
                                initial_settings.clone(),
                            ));
                        }
                        let engine = maybe_engine.as_mut().unwrap();

                        check_if_project_changed(&notify_recv, engine);
                        // Process incoming messages from the main thread
                        while let Ok(closure) = sound_recv.try_recv() {
                            closure(engine);
                        }

                        let len = dest.len();
                        let mut di = 0;
                        while di < len {
                            let synth_output_mutex = engine.synth.output_data();
                            let mut synth_output = synth_output_mutex.lock().unwrap();
                            if synth_output.buffer.len() < (len - di) {
                                drop(synth_output);
                                engine.advance_frame();
                                synth_output = synth_output_mutex.lock().unwrap();

                                if last_waveform_consumed.load(Ordering::Relaxed) {
                                    let buffer_viz = std::mem::replace(&mut synth_output.buffer_viz, Vec::new());
                                    last_waveform_consumed.store(false, Ordering::Relaxed);
                                    let consumed_clone = last_waveform_consumed.clone();
                                    window_weak.upgrade_in_event_loop(move |handle| {
                                        update_waveform(&handle, buffer_viz, consumed_clone)
                                    });
                                }
                            }

                            let src_len = std::cmp::min(len - di, synth_output.buffer.len());
                            let part = synth_output.buffer.drain(..src_len);
                            dest[di..di + src_len].copy_from_slice(part.as_slice());

                            di += src_len;
                        }
                    });
                },
                err_fn,
            ),
            // FIXME
            SampleFormat::I16 => device.build_output_stream(&stream_config, write_silence::<i16>, err_fn),
            SampleFormat::U16 => device.build_output_stream(&stream_config, write_silence::<u16>, err_fn),
        }
        .unwrap();

        fn write_silence<T: Sample>(data: &mut [T], _: &cpal::OutputCallbackInfo) {
            for sample in data.iter_mut() {
                *sample = Sample::from(&0.0);
            }
        }

        stream.play().unwrap();

        #[cfg(not(target_arch = "wasm32"))]
        let midi = {
            let cloned_sound_send2 = cloned_sound_send.clone();
            let press = move |key| cloned_sound_send2.send(Box::new(move |se| se.press_note(key))).unwrap();
            let release = move |key| {
                cloned_sound_send
                    .send(Box::new(move |se| se.release_note(key)))
                    .unwrap()
            };
            Some(Midi::new(press, release))
        };
        #[cfg(target_arch = "wasm32")]
        // The midir web backend needs to be asynchronously initialized, but midir doesn't tell
        // us when that initialization is done and that we can start querying the list of midi
        // devices. It's also annoying for users that don't care about MIDI to get a permission
        // request, so I'll need this to be enabled explicitly for the Web version.
        // The aldio latency is still so bad with the web version though,
        // so I'm not sure if that's really worth it.
        let midi = None;

        Context {
            key_release_timer: Default::default(),
            _stream: stream,
            _midi: midi,
        }
    }));

    let window_weak = window.as_weak();
    window.on_octave_increased(move |octave_delta| {
        let window = window_weak.clone().upgrade().unwrap();
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
    let already_pressed = Rc::new(RefCell::new(HashSet::new()));
    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    window.on_global_key_event(move |text, pressed| {
        if let Some(code) = text.as_str().chars().next() {
            if pressed {
                if !already_pressed.borrow().contains(&code) {
                    already_pressed.borrow_mut().insert(code.to_owned());
                    match code {
                        // Keys.Backspace
                        '\u{8}' => {
                            Lazy::force(&*cloned_context);
                            cloned_sound_send
                                .send(Box::new(|se| se.sequencer.set_erasing(true)))
                                .unwrap();
                        }
                        _ => (),
                    }
                }
            } else {
                match code {
                    // Keys.Backspace
                    '\u{8}' => {
                        cloned_sound_send
                            .send(Box::new(|se| se.sequencer.set_erasing(false)))
                            .unwrap();
                    }
                    _ => (),
                };
                already_pressed.borrow_mut().remove(&code);
            }
        }
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_select_instrument(move |instrument| {
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(Box::new(move |se| se.select_instrument(instrument as u8)))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_toggle_mute_instrument(move |instrument| {
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(Box::new(move |se| {
                se.sequencer.toggle_mute_instrument(instrument as u8)
            }))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_note_pressed(move |note| {
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(Box::new(move |se| se.press_note(note as u32)))
            .unwrap();
    });

    let cloned_sound_send = sound_send.clone();
    global_engine.on_note_released(move |note| {
        cloned_sound_send
            .send(Box::new(move |se| se.release_note(note as u32)))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_note_key_pressed(move |note| {
        let cloned_sound_send2 = cloned_sound_send.clone();
        cloned_sound_send
            .send(Box::new(move |se| se.press_note(note as u32)))
            .unwrap();

        // We have only one timer for direct interactions, and we don't handle
        // keys being held or even multiple keys at time yet, so just visually release all notes.
        cloned_context.key_release_timer.start(
            TimerMode::SingleShot,
            std::time::Duration::from_millis(15 * 6),
            Box::new(move || {
                cloned_sound_send2
                    .send(Box::new(move |se| se.release_note(note as u32)))
                    .unwrap();
            }),
        );
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_pattern_clicked(move |pattern_num| {
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(Box::new(move |se| se.sequencer.select_pattern(pattern_num as u32)))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_toggle_step(move |step_num| {
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(Box::new(move |se| se.sequencer.toggle_step(step_num as u32)))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_toggle_step_release(move |step_num| {
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(Box::new(move |se| se.sequencer.toggle_step_release(step_num as u32)))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_manually_advance_step(move |forwards| {
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(Box::new(move |se| se.sequencer.manually_advance_step(forwards)))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    let window_weak = window.as_weak();
    global_engine.on_play_clicked(move |toggled| {
        // FIXME: Stop the sound device
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(Box::new(move |se| se.sequencer.set_playing(toggled)))
            .unwrap();
        window_weak.unwrap().set_playing(toggled);
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_record_clicked(move |toggled| {
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(Box::new(move |se| se.sequencer.set_recording(toggled)))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_append_song_pattern(move |pattern_num| {
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(Box::new(move |se| se.sequencer.append_song_pattern(pattern_num as u32)))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_remove_last_song_pattern(move || {
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(Box::new(|se| se.sequencer.remove_last_song_pattern()))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_clear_song_patterns(move || {
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(Box::new(|se| se.sequencer.clear_song_patterns()))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_save_project(move || {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(Box::new(|se| se.save_project())).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_mute_instruments(move || {
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(Box::new(|se| se.synth.mute_instruments()))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    window.global::<GlobalSettings>().on_settings_changed(move |settings| {
        println!("SET {:?}", settings);
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(Box::new(move |se| se.apply_settings(settings)))
            .unwrap();
    });

    window
        .global::<GlobalUtils>()
        .on_get_midi_note_name(|note| MidiNote(note).name().into());
    window.global::<GlobalUtils>().on_mod(|x, y| x % y);

    // For WASM we need to wait for the user to trigger the creation of the sound
    // device through an input event. For other platforms, artificially force the
    // lazy context immediately.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let cloned_context = context.clone();
        Lazy::force(&*cloned_context);
    }

    window.run();
}
