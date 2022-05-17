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
use crate::utils::MidiNote;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat};
use notify::{DebouncedEvent, RecursiveMode, Watcher};
use once_cell::unsync::Lazy;
use slint::{Model, Rgba8Pixel, SharedPixelBuffer, Timer, TimerMode};
use std::cell::RefCell;
use std::collections::HashSet;
use std::env;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use tiny_skia::*;

slint::include_modules!();

thread_local! {static SOUND_ENGINE: RefCell<Option<SoundEngine>> = RefCell::new(None);}

#[derive(Debug)]
enum SoundMsg {
    PressNote(u32),
    ReleaseNote(u32),
    SelectInstrument(u32),
    ToggleMuteInstrument(u32),
    SelectPattern(u32),
    ToggleStep(u32),
    ToggleStepRelease(u32),
    ManuallyAdvanceStep(bool),
    SetPlaying(bool),
    SetRecording(bool),
    SetErasing(bool),
    AppendSongPattern(u32),
    RemoveLastSongPattern,
    ClearSongPatterns,
    SaveProject,
    MuteInstruments,
    ApplySettings(Settings),
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

fn check_if_project_changed(
    project_name: &str,
    notify_recv: &mpsc::Receiver<DebouncedEvent>,
    engine: &mut SoundEngine,
) -> () {
    while let Ok(msg) = notify_recv.try_recv() {
        let instruments_path = SoundEngine::project_instruments_path(&project_name);
        let instruments = instruments_path.file_name();
        let reload = match msg {
            DebouncedEvent::Write(path) if path.file_name() == instruments => true,
            DebouncedEvent::Create(path) if path.file_name() == instruments => true,
            DebouncedEvent::Remove(path) if path.file_name() == instruments => true,
            DebouncedEvent::Rename(from, to) if from.file_name() == instruments || to.file_name() == instruments => {
                true
            }
            _ => false,
        };
        if reload {
            #[cfg(not(target_arch = "wasm32"))]
            engine.synth.load(instruments_path.as_path());
        }
    }
}

fn process_audio_messages(project_name: &str, sound_recv: &mpsc::Receiver<SoundMsg>, engine: &mut SoundEngine) -> () {
    while let Ok(msg) = sound_recv.try_recv() {
        match msg {
            SoundMsg::PressNote(note) => engine.press_note(note),
            SoundMsg::ReleaseNote(note) => engine.release_note(note),
            SoundMsg::SelectInstrument(instrument) => engine.select_instrument(instrument),
            SoundMsg::ToggleMuteInstrument(instrument) => engine.sequencer.toggle_mute_instrument(instrument),
            SoundMsg::SelectPattern(pattern_num) => engine.sequencer.select_pattern(pattern_num),
            SoundMsg::ToggleStep(toggled) => engine.sequencer.toggle_step(toggled),
            SoundMsg::ToggleStepRelease(toggled) => engine.sequencer.toggle_step_release(toggled),
            SoundMsg::ManuallyAdvanceStep(forwards) => engine.sequencer.manually_advance_step(forwards),
            SoundMsg::SetPlaying(toggled) => engine.sequencer.set_playing(toggled),
            SoundMsg::SetRecording(toggled) => engine.sequencer.set_recording(toggled),
            SoundMsg::SetErasing(toggled) => engine.sequencer.set_erasing(toggled),
            SoundMsg::AppendSongPattern(pattern_num) => engine.sequencer.append_song_pattern(pattern_num),
            SoundMsg::RemoveLastSongPattern => engine.sequencer.remove_last_song_pattern(),
            SoundMsg::ClearSongPatterns => engine.sequencer.clear_song_patterns(),
            SoundMsg::SaveProject => engine.save_project(&project_name),
            SoundMsg::MuteInstruments => engine.synth.mute_instruments(),
            SoundMsg::ApplySettings(settings) => engine.apply_settings(settings),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn main() {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(all(debug_assertions, target_arch = "wasm32"))]
    console_error_panic_hook::set_once();

    let project_name = env::args().nth(1).unwrap_or("default".to_owned());

    // The model set in the UI are only for development.
    // Rewrite the models and use that version.
    let sequencer_song_model = Rc::new(slint::VecModel::default());
    let sequencer_pattern_model = Rc::new(slint::VecModel::default());
    for i in 0..16 {
        sequencer_pattern_model.push(PatternData {
            empty: true,
            active: i == 0,
        });
    }
    let sequencer_step_model = Rc::new(slint::VecModel::default());
    for _ in 0..16 {
        sequencer_step_model.push(StepData {
            press: false,
            release: false,
            active: false,
            note_name: "".into(),
        });
    }
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
    let global_engine = GlobalEngine::get(&window);
    global_engine.set_sequencer_song_patterns(slint::ModelRc::from(sequencer_song_model.clone()));
    global_engine.set_sequencer_patterns(slint::ModelRc::from(sequencer_pattern_model.clone()));
    global_engine.set_sequencer_steps(slint::ModelRc::from(sequencer_step_model.clone()));
    global_engine.set_synth_trace_notes(slint::ModelRc::from(Rc::new(slint::VecModel::default())));

    let (sound_send, sound_recv) = mpsc::channel();
    let (notify_send, notify_recv) = mpsc::channel();

    #[cfg(not(target_arch = "wasm32"))]
    let mut watcher = notify::watcher(notify_send, Duration::from_millis(500)).unwrap();
    #[cfg(not(target_arch = "wasm32"))]
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

        // The sequencer won't produce anything faster than every 1/64th second,
        // and live notes should probably eventually be quantized onto that,
        // so a buffer roughly the size of a frame should work fine.
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
                                &project_name,
                                window_weak.clone(),
                                initial_settings.clone(),
                            ));
                        }
                        let engine = maybe_engine.as_mut().unwrap();

                        check_if_project_changed(&project_name, &notify_recv, engine);
                        process_audio_messages(&project_name, &sound_recv, engine);

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
            let press = move |key| cloned_sound_send2.send(SoundMsg::PressNote(key)).unwrap();
            let release = move |key| cloned_sound_send.send(SoundMsg::ReleaseNote(key)).unwrap();
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
                            cloned_sound_send.send(SoundMsg::SetErasing(true)).unwrap();
                        }
                        _ => (),
                    }
                }
            } else {
                match code {
                    // Keys.Backspace
                    '\u{8}' => {
                        cloned_sound_send.send(SoundMsg::SetErasing(false)).unwrap();
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
            .send(SoundMsg::SelectInstrument(instrument as u32))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_toggle_mute_instrument(move |instrument| {
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(SoundMsg::ToggleMuteInstrument(instrument as u32))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_note_pressed(move |note| {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::PressNote(note as u32)).unwrap();
    });

    let cloned_sound_send = sound_send.clone();
    global_engine.on_note_released(move |note| {
        cloned_sound_send.send(SoundMsg::ReleaseNote(note as u32)).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_note_key_pressed(move |note| {
        let cloned_sound_send2 = cloned_sound_send.clone();
        cloned_sound_send.send(SoundMsg::PressNote(note as u32)).unwrap();

        // We have only one timer for direct interactions, and we don't handle
        // keys being held or even multiple keys at time yet, so just visually release all notes.
        cloned_context.key_release_timer.start(
            TimerMode::SingleShot,
            std::time::Duration::from_millis(15 * 6),
            Box::new(move || {
                cloned_sound_send2.send(SoundMsg::ReleaseNote(note as u32)).unwrap();
            }),
        );
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_pattern_clicked(move |pattern_num| {
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(SoundMsg::SelectPattern(pattern_num as u32))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_toggle_step(move |step_num| {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::ToggleStep(step_num as u32)).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_toggle_step_release(move |step_num| {
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(SoundMsg::ToggleStepRelease(step_num as u32))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_manually_advance_step(move |forwards| {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::ManuallyAdvanceStep(forwards)).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    let window_weak = window.as_weak();
    global_engine.on_play_clicked(move |toggled| {
        // FIXME: Stop the sound device
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::SetPlaying(toggled)).unwrap();
        window_weak.unwrap().set_playing(toggled);
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_record_clicked(move |toggled| {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::SetRecording(toggled)).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_append_song_pattern(move |pattern_num| {
        Lazy::force(&*cloned_context);
        cloned_sound_send
            .send(SoundMsg::AppendSongPattern(pattern_num as u32))
            .unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_remove_last_song_pattern(move || {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::RemoveLastSongPattern).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_clear_song_patterns(move || {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::ClearSongPatterns).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_save_project(move || {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::SaveProject).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    global_engine.on_mute_instruments(move || {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::MuteInstruments).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    window.global::<GlobalSettings>().on_settings_changed(move |settings| {
        println!("SET {:?}", settings);
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::ApplySettings(settings)).unwrap();
    });

    window
        .global::<GlobalUtils>()
        .on_get_midi_note_name(|note| MidiNote(note).name());
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
