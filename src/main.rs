// Note: Game BoyTM, Game Boy PocketTM, Super Game BoyTM and Game Boy ColorTM are registered trademarks of
// Nintendo CO., LTD. © 1989 to 1999 by Nintendo CO., LTD.
use cpal::{Sample, SampleFormat};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod log;
mod sequencer;
mod sound;
mod synth;
mod synth_script;

use crate::sound::SoundStuff;
use gameboy::apu::Apu;
use once_cell::unsync::Lazy;
use sixtyfps::{SharedPixelBuffer, Timer, TimerMode};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use tiny_skia::*;

sixtyfps::include_modules!();

fn update_waveform(window: &MainWindow, samples: Vec<(f32,f32)>) {
    let was_non_zero = !window.get_waveform_is_zero();
    let width = window.get_waveform_width() / 2.;
    let height = window.get_waveform_height() / 2.;
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
        sound: Arc<Mutex<SoundStuff>>,
        key_release_timer: Timer,
        _stream: cpal::Stream,
    }

    let window = MainWindow::new();
    window.set_sequencer_song_patterns(sixtyfps::ModelHandle::new(sequencer_song_model.clone()));
    window.set_sequencer_patterns(sixtyfps::ModelHandle::new(sequencer_pattern_model.clone()));
    window.set_sequencer_steps(sixtyfps::ModelHandle::new(sequencer_step_model.clone()));
    window.set_notes(sixtyfps::ModelHandle::new(note_model.clone()));

    let window_weak = window.as_weak();
    let context = Rc::new(Lazy::new(|| {
        let host = cpal::default_host();
        let device = host.default_output_device().unwrap();
        log!("Open the audio player: {}", device.name().unwrap());
        let config = device.default_output_config().unwrap();
        log!("Audio format {:?}", config);

        let audio_buffer_samples = 512;
        // assert!(config.sample_rate().0 / 64 > audio_buffer_samples, "We only pre-fill one APU frame.");
        let apu = Apu::power_up(config.sample_rate().0);

        let err_fn = |err| elog!("an error occurred on the output audio stream: {}", err);
        let sample_format = config.sample_format();
        let mut config: cpal::StreamConfig = config.into();
        config.buffer_size = cpal::BufferSize::Fixed(audio_buffer_samples);

        let apu_data = apu.buffer.clone();
        let sound_stuff = Arc::new(Mutex::new(SoundStuff::new(apu, window_weak.clone())));

        let cloned_sound = sound_stuff.clone();
        let stream = match sample_format {
            SampleFormat::F32 =>
            device.build_output_stream(&config,
                move |dest: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let len = dest.len() / 2;
                    let mut di = 0;
                    while di < len {
                        {
                            let mut sound = cloned_sound.lock().unwrap();
                            if sound.synth.ready_buffer_samples() < (len - di) {
                                // Reset the indicator of the start of the next note wave.
                                sound.synth.reset_buffer_wave_start();
                                sound.advance_frame();

                                let num_samples = 350;
                                let wave_start = {
                                    let start = sound.synth.buffer_wave_start();
                                    if start > num_samples {
                                        0
                                    } else {
                                        start
                                    }
                                };
                                drop(sound); // unlock

                                let copy = apu_data.lock().unwrap()[wave_start..(wave_start+num_samples)].to_vec();
                                window_weak.clone().upgrade_in_event_loop(move |handle| {
                                    update_waveform(&handle, copy)
                                });
                            }
                        }
                        let mut source = apu_data.lock().unwrap();
                        let src_len = std::cmp::min(len-di, source.len());

                        for (i, (source_l, source_r)) in source.drain(..src_len).enumerate() {
                            dest[(di + i) * 2] = source_l;
                            dest[(di + i) * 2 + 1] = source_r;
                        }
                        di += src_len;
                    }                     
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
            sound: sound_stuff,
            key_release_timer: Default::default(),
            _stream: stream,
        }
    }));

    let cloned_context = context.clone();
    let window_weak = window.as_weak();
    window.on_selected_instrument_changed(move |instrument| {
        let mut sound = cloned_context.sound.lock().unwrap();
        sound.select_instrument(instrument as u32);
        // Just forward it back to the UI, it doesn't change otherwise.
        window_weak.unwrap().set_selected_instrument(instrument);
    });

    let cloned_context = context.clone();
    window.on_note_pressed(move |note| {
        let mut sound = cloned_context.sound.lock().unwrap();
        sound.press_note(note as u32);

        let cloned_sound = cloned_context.sound.clone();
        cloned_context.key_release_timer.start(
            TimerMode::SingleShot,
            std::time::Duration::from_millis(15 * 6),
            Box::new(move || cloned_sound.lock().unwrap().release_notes())
        );
    });

    let cloned_context = context.clone();
    window.on_pattern_clicked(move |pattern_num| {
        let mut sound = cloned_context.sound.lock().unwrap();
        sound.sequencer.select_pattern(pattern_num as u32);
    });

    let cloned_context = context.clone();
    window.on_step_clicked(move |step_num| {
        let mut sound = cloned_context.sound.lock().unwrap();
        sound.sequencer.toggle_step(step_num as u32);
    });

    let cloned_context = context.clone();
    let window_weak = window.as_weak();
    window.on_play_clicked(move |toggled| {
        // FIXME: Stop the sound device
        let mut sound = cloned_context.sound.lock().unwrap();
        sound.sequencer.set_playing(toggled);
        window_weak.unwrap().set_playing(toggled);
    });

    let cloned_context = context.clone();
    window.on_record_clicked(move |toggled| {
        let mut sound = cloned_context.sound.lock().unwrap();
        sound.sequencer.set_recording(toggled);
    });

    let cloned_context = context.clone();
    window.on_append_song_pattern(move |pattern_num| {
        let mut sound = cloned_context.sound.lock().unwrap();
        sound.sequencer.append_song_pattern(pattern_num as u32);
    });

    let cloned_context = context.clone();
    window.on_remove_last_song_pattern(move || {
        let mut sound = cloned_context.sound.lock().unwrap();
        sound.sequencer.remove_last_song_pattern();
    });

    let cloned_context = context.clone();
    window.on_clear_song_patterns(move || {
        let mut sound = cloned_context.sound.lock().unwrap();
        sound.sequencer.clear_song_patterns();
    });

    window.run();
}
