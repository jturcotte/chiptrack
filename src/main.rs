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

use crate::sound::SoundStuff;
use gameboy::apu::Apu;
use once_cell::unsync::Lazy;
use sixtyfps::{Timer, TimerMode};
use std::cell::RefCell;
use std::rc::Rc;

sixtyfps::include_modules!();

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn main() {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(all(debug_assertions, target_arch = "wasm32"))]
    console_error_panic_hook::set_once();

    // The model set in the UI are only for development.
    // Rewrite the models and use that version.
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
        sound: Rc<RefCell<SoundStuff>>,
        key_release_timer: Timer,
        _stream: cpal::Stream,
        _apu_timer: Timer,
    }

    let window = MainWindow::new();
    window.set_sequencer_patterns(sixtyfps::ModelHandle::new(sequencer_pattern_model.clone()));
    window.set_sequencer_steps(sixtyfps::ModelHandle::new(sequencer_step_model.clone()));
    window.set_notes(sixtyfps::ModelHandle::new(note_model.clone()));

    let context = Rc::new(Lazy::new(|| {
        let host = cpal::default_host();
        let device = host.default_output_device().unwrap();
        log!("Open the audio player: {}", device.name().unwrap());
        let config = device.default_output_config().unwrap();
        log!("Audio format {:?}", config);

        let audio_buffer_samples = 512;
        assert!(config.sample_rate().0 / 64 > audio_buffer_samples, "We only pre-fill one APU frame.");
        let apu = Apu::power_up(config.sample_rate().0);
        let apu_data = apu.buffer.clone();

        let err_fn = |err| elog!("an error occurred on the output audio stream: {}", err);
        let sample_format = config.sample_format();
        let mut config: cpal::StreamConfig = config.into();
        config.buffer_size = cpal::BufferSize::Fixed(audio_buffer_samples);

        let stream = match sample_format {
            SampleFormat::F32 =>
            device.build_output_stream(&config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // FIXME: Try a 0ms times instead of a repeated one that checks the buffer.
                    let mut apu_data2 = apu_data.lock().unwrap();
                    let len = std::cmp::min(data.len() / 2, apu_data2.len());
                    for (i, (data_l, data_r)) in apu_data2.drain(..len).enumerate() {
                        data[i * 2] = data_l;
                        data[i * 2 + 1] = data_r;
                    }

                    for i in len..(data.len() / 2) {
                        // Buffer underrun!! At least fill with zeros to reduce the glitching.
                        data[i * 2] = 0.0;
                        data[i * 2 + 1] = 0.0;
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

        let sound_stuff = Rc::new(RefCell::new(SoundStuff::new(apu, sequencer_pattern_model, sequencer_step_model, note_model)));

        let apu_timer: Timer = Default::default();
        let cloned_sound = sound_stuff.clone();
        apu_timer.start(
            TimerMode::Repeated,
            std::time::Duration::from_millis(5),
            Box::new(move || {
               // Make sure to always have at least one audio frame in the synth buffer
               // in case cpal calls back to fill the audio output buffer.
               let pre_filled_audio_frames = 44100 / 64;
               while cloned_sound.borrow().synth.ready_buffer_samples() < pre_filled_audio_frames {
                    cloned_sound.borrow_mut().advance_frame();
               }
            })
        );

        stream.play().unwrap();
        Context {
            sound: sound_stuff,
            key_release_timer: Default::default(),
            _stream: stream,
            _apu_timer: apu_timer,
        }
    }));

    let cloned_context = context.clone();
    let window_weak = window.as_weak();
    window.on_selected_instrument_changed(move |instrument| {
        let mut sound = cloned_context.sound.borrow_mut();
        sound.select_instrument(instrument as u32);
        // Just forward it back to the UI, it doesn't change otherwise.
        window_weak.unwrap().set_selected_instrument(instrument);
    });

    let cloned_context = context.clone();
    window.on_note_pressed(move |note| {
        let mut sound = cloned_context.sound.borrow_mut();
        sound.press_note(note as u32);

        let cloned_sound = cloned_context.sound.clone();
        cloned_context.key_release_timer.start(
            TimerMode::SingleShot,
            std::time::Duration::from_millis(15 * 6),
            Box::new(move || cloned_sound.borrow_mut().release_notes())
        );
    });

    let cloned_context = context.clone();
    window.on_pattern_clicked(move |pattern_num| {
        let mut sound = cloned_context.sound.borrow_mut();
        sound.sequencer.select_pattern(pattern_num as u32);
    });

    let cloned_context = context.clone();
    window.on_step_clicked(move |step_num| {
        let mut sound = cloned_context.sound.borrow_mut();
        sound.sequencer.toggle_step(step_num as u32);
    });

    let cloned_context = context.clone();
    let window_weak = window.as_weak();
    window.on_play_clicked(move |toggled| {
        // FIXME: Stop the timer
        let mut sound = cloned_context.sound.borrow_mut();
        sound.sequencer.set_playing(toggled);
        window_weak.unwrap().set_playing(toggled);
    });

    let cloned_context = context.clone();
    window.on_record_clicked(move |toggled| {
        let mut sound = cloned_context.sound.borrow_mut();
        sound.sequencer.set_recording(toggled);
    });

    window.run();
}
