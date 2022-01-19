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
use cpal::{Sample, SampleFormat};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use notify::{DebouncedEvent, RecursiveMode, Watcher};
use once_cell::unsync::Lazy;
use sixtyfps::{Model, Rgba8Pixel, SharedPixelBuffer, Timer, TimerMode};
use std::cell::RefCell;
use std::collections::HashSet;
use std::env;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;
use tiny_skia::*;

sixtyfps::include_modules!();

thread_local! {static SOUND_ENGINE: RefCell<Option<SoundEngine>> = RefCell::new(None);}

#[derive(Debug)]
enum SoundMsg {
    PressNote(u32),
    SelectInstrument(u32),
    SelectPattern(u32),
    ToggleStep(u32),
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

fn update_waveform(window: &MainWindow, samples: Vec<f32>, gain: f32, consumed: Arc<AtomicBool>) {
    let was_non_zero = !window.get_waveform_is_zero();
    let res_divider = 2.;

    // Already let the audio thread know that it can send us a new waveform.
    consumed.store(true, Ordering::Relaxed);

    let width = window.get_waveform_width() / res_divider;
    let height = window.get_waveform_height() / res_divider;
    let mut pb = PathBuilder::new();
    let mut non_zero = false;
    pb.move_to(0.0, height / 2.0);
    {
        for (i, source) in samples.iter().enumerate() {
            // Plot only the right channel as the left one might be used as a sync channel.
            if i % 2 == 0 {
                continue;
            }
            if *source != 0.0 {
                non_zero = true;
            }
            // Input samples are in the range [-1.0, 1.0].
            // The gameboy emulator mixer however just use a gain of 0.25
            // per channel to avoid clipping when all channels are playing.
            // So multiply by 2.0 to amplify the visualization of single
            // channels a bit.
            pb.line_to(
                i as f32 * width / (samples.len() / 2) as f32,
                (source / gain * 2.0 + 1.0) * height / 2.0);
        }
    }
    // Painting this takes a lot of CPU since we need to paint, clone
    // the whole pixmap buffer, and changing the image will trigger a
    // repaint of the full viewport.
    // So at least avoig eating CPU while no sound is being output.
    if non_zero || was_non_zero {
        if let Some(path) = pb.finish() {
            let mut pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::new(width as usize, height as usize);
            if let Some(mut pixmap) = PixmapMut::from_bytes(pixel_buffer.make_mut_bytes(), width as u32, height as u32) {
                pixmap.fill(tiny_skia::Color::TRANSPARENT);
                let mut paint = Paint::default();
                paint.blend_mode = BlendMode::Source;
                // #a0a0a0
                paint.set_color_rgba8(160, 160, 160, 255);

                let mut stroke = Stroke::default();
                // Use hairline stroking, faster.
                stroke.width = 0.0;
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);

                let image = sixtyfps::Image::from_rgba8_premultiplied(pixel_buffer);
                window.set_waveform_image(image);
                window.set_waveform_is_zero(!non_zero);
            }
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
    let sequencer_song_model = Rc::new(sixtyfps::VecModel::default());
    let sequencer_pattern_model = Rc::new(sixtyfps::VecModel::default());
    for i in 0..16 {
        sequencer_pattern_model.push(PatternData{empty: true, active: i == 0});
    }
    let sequencer_step_model = Rc::new(sixtyfps::VecModel::default());
    for _ in 0..16 {
        sequencer_step_model.push(StepData{empty: true, active: false, note_name: "".into()});
    }
    let note_model = Rc::new(sixtyfps::VecModel::default());
    let start: i32 = 60;
    let notes: Vec<NoteData> = (start..(start+13)).map(|i| {
        let semitone = (i - start) % 12;
        let octav = (i - start) / 12;
        let major_scale = [0, 2, 4, 5, 7, 9, 11];
        let r = major_scale.binary_search(&semitone);
        let (scale_pos, is_black) = match r {
            Ok(p) => (p, false),
            Err(p) => (p - 1, true),
        };
        let pos = scale_pos as i32 + octav * 7;
        NoteData{note_number: i, scale_pos: pos, is_black: is_black, active: false}
    }).collect();
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
    window.set_sequencer_song_patterns(sixtyfps::ModelHandle::new(sequencer_song_model.clone()));
    window.set_sequencer_patterns(sixtyfps::ModelHandle::new(sequencer_pattern_model.clone()));
    window.set_sequencer_steps(sixtyfps::ModelHandle::new(sequencer_step_model.clone()));
    window.set_notes(sixtyfps::ModelHandle::new(note_model.clone()));

    let (sound_send, sound_recv) = mpsc::channel();
    let (notify_send, notify_recv) = mpsc::channel();

    #[cfg(not(target_arch = "wasm32"))]
    let mut watcher = notify::watcher(notify_send, Duration::from_millis(500)).unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    watcher.watch(".", RecursiveMode::NonRecursive).unwrap();

    let window_weak = window.as_weak();
    let initial_settings = window.global::<SettingsGlobal>().get_settings();
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
        let mut config: cpal::StreamConfig = config.into();

        // The sequencer won't produce anything faster than every 1/64th second,
        // and live notes should probably eventually be quantized onto that,
        // so a buffer roughly the size of a frame should work fine.
        #[cfg(not(target_arch = "wasm32"))]
            let audio_buffer_samples = 512;
        // Everything happens on the same thread in wasm32, and is a bit slower,
        // so increase the buffer size there.
        #[cfg(target_arch = "wasm32")]
            let audio_buffer_samples = 2048;

        config.buffer_size = cpal::BufferSize::Fixed(audio_buffer_samples);

        let last_waveform_consumed  = Arc::new(AtomicBool::new(true));

        let stream = match sample_format {
            SampleFormat::F32 =>
            device.build_output_stream(&config,
                move |dest: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    SOUND_ENGINE.with(|maybe_engine_cell| {
                        let mut maybe_engine = maybe_engine_cell.borrow_mut();
                        if let None = *maybe_engine {
                            *maybe_engine = Some(SoundEngine::new(sample_rate, &project_name, window_weak.clone(), initial_settings.clone()));
                        }
                        let engine = maybe_engine.as_mut().unwrap();

                        while let Ok(msg) = notify_recv.try_recv() {
                            let instruments_path = SoundEngine::project_instruments_path(&project_name);
                            let instruments = instruments_path.file_name();
                            let reload = match msg {
                                DebouncedEvent::Write(path) if path.file_name() == instruments => true,
                                DebouncedEvent::Create(path) if path.file_name() == instruments => true,
                                DebouncedEvent::Remove(path) if path.file_name() == instruments => true,
                                DebouncedEvent::Rename(from, to) if from.file_name() == instruments || to.file_name() == instruments => true,
                                _ => false,
                            };
                            if reload {
                                #[cfg(not(target_arch = "wasm32"))]
                                engine.synth.load(instruments_path.as_path());
                            }
                        }
                        while let Ok(msg) = sound_recv.try_recv() {
                            match msg {
                                SoundMsg::PressNote(note) => engine.press_note(note),
                                SoundMsg::SelectInstrument(instrument) => engine.select_instrument(instrument),
                                SoundMsg::SelectPattern(pattern_num) => engine.sequencer.select_pattern(pattern_num),
                                SoundMsg::ToggleStep(toggled) => engine.sequencer.toggle_step(toggled),
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
                                    let num_samples = sample_rate as usize / 64 * 2 / 3;
                                    let wave_start = synth_output.buffer_wave_start.unwrap_or(0);

                                    let end = synth_output.buffer.len().min(wave_start + num_samples * 2);
                                    let copy = synth_output.buffer[wave_start..end].to_vec();
                                    let gain = synth_output.gain;
                                    last_waveform_consumed.store(false, Ordering::Relaxed);
                                    let consumed_clone = last_waveform_consumed.clone();
                                    window_weak.clone().upgrade_in_event_loop(move |handle| {
                                        update_waveform(&handle, copy, gain, consumed_clone)
                                    });
                                }
                            }

                            let src_len = std::cmp::min(len-di, synth_output.buffer.len());
                            let part = synth_output.buffer.drain(..src_len);
                            dest[di..di+src_len].copy_from_slice(part.as_slice());

                            di += src_len;
                        }
                    });
                }, err_fn),
            // FIXME
            SampleFormat::I16 => device.build_output_stream(&config, write_silence::<i16>, err_fn),
            SampleFormat::U16 => device.build_output_stream(&config, write_silence::<u16>, err_fn),
        }.unwrap();

        fn write_silence<T: Sample>(data: &mut [T], _: &cpal::OutputCallbackInfo) {
            for sample in data.iter_mut() {
                *sample = Sample::from(&0.0);
            }
        }

        stream.play().unwrap();

        #[cfg(not(target_arch = "wasm32"))]
            let midi = Some(Midi::new(move |key| cloned_sound_send.send(SoundMsg::PressNote(key)).unwrap()));
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

    window.on_get_midi_note_name(move |note| {
        utils::midi_note_name(note as u32)
    });

    window.on_mod(|x, y| {
        x % y
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    window.on_select_instrument(move |instrument| {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::SelectInstrument(instrument as u32)).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    let window_weak = window.as_weak();
    window.on_note_pressed(move |note| {
        cloned_sound_send.send(SoundMsg::PressNote(note as u32)).unwrap();

        let model = window_weak.clone().upgrade().unwrap().get_notes();
        for row in 0..model.row_count() {
            let mut row_data = model.row_data(row);
            if row_data.note_number == note {
                row_data.active = true;
                model.set_row_data(row, row_data);
            }
        }

        // We have only one timer for direct interactions, and we don't handle
        // keys being held or even multiple keys at time yet, so just visually release all notes.
        let window_weak = window_weak.clone();
        cloned_context.key_release_timer.start(
            TimerMode::SingleShot,
            std::time::Duration::from_millis(15 * 6),
            Box::new(move || {
                let handle = window_weak.upgrade().unwrap();
                let notes_model = handle.get_notes();
                for row in 0..notes_model.row_count() {
                    let mut row_data = notes_model.row_data(row);
                    if row_data.active {
                        row_data.active = false;
                        notes_model.set_row_data(row, row_data);
                    }
                }

                let instruments_model = handle.get_instruments();
                for row in 0..instruments_model.row_count() {
                    let mut row_data = instruments_model.row_data(row);
                    if row_data.active {
                        row_data.active = false;
                        instruments_model.set_row_data(row, row_data);
                    }
                }
            })

        );
    });

    let window_weak = window.as_weak();
    window.on_octave_increased(move |octave_delta| {
        let window = window_weak.clone().upgrade().unwrap();
        let first_note = window.get_first_note();
        if first_note <= 24 && octave_delta < 0
            || first_note >= 96 && octave_delta > 0 {
            return;
        }
        window.set_first_note(first_note + octave_delta * 12);
        let model = window.get_notes();
        for row in 0..model.row_count() {
            let mut row_data = model.row_data(row);
            row_data.note_number += octave_delta * 12;
            // The note_number changed and thus the sequencer release events
            // won't see that note anymore, so release it already while we're here here.
            row_data.active = false;
            model.set_row_data(row, row_data);
        }
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    window.on_pattern_clicked(move |pattern_num| {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::SelectPattern(pattern_num as u32)).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    window.on_step_clicked(move |step_num| {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::ToggleStep(step_num as u32)).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    window.on_manually_advance_step(move |forwards| {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::ManuallyAdvanceStep(forwards)).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    let window_weak = window.as_weak();
    window.on_play_clicked(move |toggled| {
        // FIXME: Stop the sound device
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::SetPlaying(toggled)).unwrap();
        window_weak.unwrap().set_playing(toggled);
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    window.on_record_clicked(move |toggled| {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::SetRecording(toggled)).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    window.on_append_song_pattern(move |pattern_num| {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::AppendSongPattern(pattern_num as u32)).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    window.on_remove_last_song_pattern(move || {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::RemoveLastSongPattern).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    window.on_clear_song_patterns(move || {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::ClearSongPatterns).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    window.on_save_project(move || {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::SaveProject).unwrap();
    });

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    window.on_mute_instruments(move || {
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::MuteInstruments).unwrap();
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
                        '\u{7}' => {
                            Lazy::force(&*cloned_context);
                            cloned_sound_send.send(SoundMsg::SetErasing(true)).unwrap();
                        },
                        _ => (),
                    }
                }
            } else {
                match code {
                    '\u{7}' => {                    
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
    window.global::<SettingsGlobal>().on_settings_changed(move |settings| {
        println!("SET {:?}", settings);
        Lazy::force(&*cloned_context);
        cloned_sound_send.send(SoundMsg::ApplySettings(settings)).unwrap();
    });

    window.run();
}
