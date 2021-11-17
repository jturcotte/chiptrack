// Note: Game BoyTM, Game Boy PocketTM, Super Game BoyTM and Game Boy ColorTM are registered trademarks of
// Nintendo CO., LTD. © 1989 to 1999 by Nintendo CO., LTD.
use cpal::{Sample, SampleFormat};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod log;
mod sequencer;
mod sound_engine;
mod synth;
mod synth_script;

use crate::sound_engine::SoundEngine;
use once_cell::unsync::Lazy;
use sixtyfps::{Model, SharedPixelBuffer, Timer, TimerMode};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use tiny_skia::*;

sixtyfps::include_modules!();

thread_local! {static SOUND_ENGINE: RefCell<Option<SoundEngine>> = RefCell::new(None);}

#[derive(Debug)]
enum SoundMsg {
    PressNote(u32),
    SelectInstrument(u32),
    SelectPattern(u32),
    ToggleStep(u32),
    SetPlaying(bool),
    SetRecording(bool),
    AppendSongPattern(u32),
    RemoveLastSongPattern,
    ClearSongPatterns,
    SaveProject,
}

fn update_waveform(window: &MainWindow, samples: Vec<(f32,f32)>) {
    let was_non_zero = !window.get_waveform_is_zero();
    #[cfg(not(target_arch = "wasm32"))]
        let res_divider = 2.;
    #[cfg(target_arch = "wasm32")]
        let res_divider = 4.;
    let width = window.get_waveform_width() / res_divider;
    let height = window.get_waveform_height() / res_divider;
    let mut pb = PathBuilder::new();
    let mut non_zero = false;
    pb.move_to(0.0, height / 2.0);
    {
        for (i, (source_l, _source_r)) in samples.iter().enumerate() {
            if *source_l != 0.0 {
                non_zero = true;
            }
            // Input samples are in the range [-1.0, 1.0].
            // The gameboy emulator mixer however just use a gain of 0.25
            // per channel to avoid clipping when all channels are playing.
            // So multiply by 2.0 to amplify the visualization of single
            // channels a bit.
            pb.line_to(
                i as f32 * width / samples.len() as f32,
                (source_l * 2.0 + 1.0) * height / 2.0);
        }
    }
    // Painting this takes a lot of CPU since we need to paint, clone
    // the whole pixmap buffer, and changing the image will trigger a
    // repaint of the full viewport.
    // So at least avoig eating CPU while no sound is being output.
    if non_zero || was_non_zero {
        if let Some(path) = pb.finish() {

            let mut paint = Paint::default();
            // #a0a0a0
            paint.set_color_rgba8(160, 160, 160, 255);

            let mut pixmap = Pixmap::new(width as u32, height as u32).unwrap();
            pixmap.stroke_path(&path, &paint, &Stroke::default(), Transform::identity(), None);

            let pixel_buffer = SharedPixelBuffer::clone_from_slice(pixmap.data(), pixmap.width() as usize, pixmap.height() as usize);
            let image = sixtyfps::Image::from_rgba8_premultiplied(pixel_buffer);
            window.set_waveform_image(image);
            window.set_waveform_is_zero(!non_zero);
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn main() {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(all(debug_assertions, target_arch = "wasm32"))]
    console_error_panic_hook::set_once();

    let project_name = "default".to_owned();

    // The model set in the UI are only for development.
    // Rewrite the models and use that version.
    let sequencer_song_model = Rc::new(sixtyfps::VecModel::default());
    let sequencer_pattern_model = Rc::new(sixtyfps::VecModel::default());
    for i in 0..8 {
        sequencer_pattern_model.push(PatternData{empty: true, active: i == 0});
    }
    let sequencer_step_model = Rc::new(sixtyfps::VecModel::default());
    for _ in 0..16 {
        sequencer_step_model.push(StepData{empty: true, active: false});
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
    }

    let window = MainWindow::new();
    window.set_sequencer_song_patterns(sixtyfps::ModelHandle::new(sequencer_song_model.clone()));
    window.set_sequencer_patterns(sixtyfps::ModelHandle::new(sequencer_pattern_model.clone()));
    window.set_sequencer_steps(sixtyfps::ModelHandle::new(sequencer_step_model.clone()));
    window.set_notes(sixtyfps::ModelHandle::new(note_model.clone()));

    let (sound_send, sound_recv) = mpsc::channel();
    // let (notify_send, notify_recv) = mpsc::channel();

    // let mut watcher: RecommendedWatcher = try!(Watcher::new(notify_send, Duration::from_secs(2)));
    // try!(watcher.watch("/home/test/notify", RecursiveMode::Recursive));

    let window_weak = window.as_weak();
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
        let audio_buffer_samples = 512;
        config.buffer_size = cpal::BufferSize::Fixed(audio_buffer_samples);

        let stream = match sample_format {
            SampleFormat::F32 =>
            device.build_output_stream(&config,
                move |dest: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    SOUND_ENGINE.with(|maybe_engine_cell| {
                        let mut maybe_engine = maybe_engine_cell.borrow_mut();
                        let engine = maybe_engine.get_or_insert_with(|| SoundEngine::new(sample_rate, &project_name, window_weak.clone()));

                        while let Ok(msg) = sound_recv.try_recv() {
                            match msg {
                                SoundMsg::PressNote(note) => engine.press_note(note),
                                SoundMsg::SelectInstrument(instrument) => engine.select_instrument(instrument),
                                SoundMsg::SelectPattern(pattern_num) => engine.sequencer.select_pattern(pattern_num),
                                SoundMsg::ToggleStep(toggled) => engine.sequencer.toggle_step(toggled),
                                SoundMsg::SetPlaying(toggled) => engine.sequencer.set_playing(toggled),
                                SoundMsg::SetRecording(toggled) => engine.sequencer.set_recording(toggled),
                                SoundMsg::AppendSongPattern(pattern_num) => engine.sequencer.append_song_pattern(pattern_num),
                                SoundMsg::RemoveLastSongPattern => engine.sequencer.remove_last_song_pattern(),
                                SoundMsg::ClearSongPatterns => engine.sequencer.clear_song_patterns(),
                                SoundMsg::SaveProject => engine.save_project(&project_name),
                            }
                        }

                        let len = dest.len() / 2;
                        let mut di = 0;
                        while di < len {
                            let synth_buffer = engine.synth.buffer();
                            if engine.synth.ready_buffer_samples() < (len - di) {
                                // Reset the indicator of the start of the next note wave.
                                engine.synth.reset_buffer_wave_start();
                                engine.advance_frame();

                                let num_samples = 350;
                                let wave_start = {
                                    let start = engine.synth.buffer_wave_start();
                                    if start > num_samples {
                                        0
                                    } else {
                                        start
                                    }
                                };

                                let copy = synth_buffer.lock().unwrap()[wave_start..(wave_start+num_samples)].to_vec();
                                window_weak.clone().upgrade_in_event_loop(move |handle| {
                                    update_waveform(&handle, copy)
                                });
                            }

                            let mut source = synth_buffer.lock().unwrap();
                            let src_len = std::cmp::min(len-di, source.len());

                            for (i, (source_l, source_r)) in source.drain(..src_len).enumerate() {
                                dest[(di + i) * 2] = source_l;
                                dest[(di + i) * 2 + 1] = source_r;
                            }
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
        Context {
            key_release_timer: Default::default(),
            _stream: stream,
        }
    }));

    let cloned_context = context.clone();
    let cloned_sound_send = sound_send.clone();
    window.on_selected_instrument_changed(move |instrument| {
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
                let model = window_weak.upgrade().unwrap().get_notes();
                for row in 0..model.row_count() {
                    let mut row_data = model.row_data(row);
                    if row_data.active {
                        row_data.active = false;
                        model.set_row_data(row, row_data);
                    }
                }
            })

        );
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

    window.run();
}
