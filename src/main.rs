// Note: Game BoyTM, Game Boy PocketTM, Super Game BoyTM and Game Boy ColorTM are registered trademarks of
// Nintendo CO., LTD. © 1989 to 1999 by Nintendo CO., LTD.
use cpal::{Sample, SampleFormat};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod sequencer;
mod sound;
mod synth;

use crate::sound::SoundStuff;
use gameboy::apu::Apu;
use gameboy::memory::Memory;
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
    let sequencer_bar_model = Rc::new(sixtyfps::VecModel::default());
    for _ in 0..(sequencer::NUM_STEPS/16) {
        sequencer_bar_model.push(BarData{});
    }
    let sequencer_step_model = Rc::new(sixtyfps::VecModel::default());
    for _ in 0..16 {
        sequencer_step_model.push(StepData{empty: true,});
    }
    let note_model = Rc::new(sixtyfps::VecModel::default());
    let notes: Vec<NoteData> = (60..73_i32).map(|i| {
        let semitone = (i - 60) % 12;
        let octav = (i - 60) / 12;
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
    window.set_sequencer_bars(sixtyfps::ModelHandle::new(sequencer_bar_model.clone()));
    window.set_sequencer_steps(sixtyfps::ModelHandle::new(sequencer_step_model.clone()));
    window.set_notes(sixtyfps::ModelHandle::new(note_model.clone()));

    let window_weak = window.as_weak();
    let context = Rc::new(Lazy::new(|| {
        let host = cpal::default_host();
        let device = host.default_output_device().unwrap();
        println!("Open the audio player: {}", device.name().unwrap());
        let config = device.default_output_config().unwrap();
        println!("Audio format {:?}", config);

        let mut apu = Apu::power_up(config.sample_rate().0);
        // Already power it on.
        apu.set( 0xff26, 0x80 );
        let apu_data = apu.buffer.clone();

        let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
        let sample_format = config.sample_format();
        let config = config.into();

        let stream = match sample_format {
            SampleFormat::F32 =>
            device.build_output_stream(&config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut apu_data2 = apu_data.lock().unwrap();
                    let len = std::cmp::min(data.len() / 2, apu_data2.len());
                        for (i, (data_l, data_r)) in apu_data2.drain(..len).enumerate() {
                            data[i * 2] = data_l;
                            data[i * 2 + 1] = data_r;
                        }
                        drop (apu_data2);
                        for i in len..(data.len() / 2) {
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

        let sound_stuff = Rc::new(RefCell::new(SoundStuff::new(apu, window_weak, sequencer_step_model, note_model)));

        let apu_timer: Timer = Default::default();
        let cloned_sound = sound_stuff.clone();
        apu_timer.start(
            TimerMode::Repeated,
            std::time::Duration::from_millis(15),
            Box::new(move || {
               // FIXME: Calculate based on the sample rate
               while cloned_sound.borrow().synth.apu.buffer.lock().unwrap().len() < 1378 {
                    let mut sound = cloned_sound.borrow_mut();
                    sound.advance_frame();
                    // FIXME: Calculate from the timer rate
                    // Advance one frame (1/64th of CLOCK_FREQUENCY)
                    sound.synth.apu.next(65536);
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
    window.on_selected_instrument_changed(move |instrument| {
        let mut sound = cloned_context.sound.borrow_mut();
        sound.select_instrument(instrument as u32);
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
    let window_weak = window.as_weak();
    window.on_bar_clicked(move |bar_num| {
        let mut sound = cloned_context.sound.borrow_mut();
        let new_lock = match sound.sequencer.set_locked_bar(Some(bar_num as u32)) {
            Some(n) => n as i32,
            None => -1,
        };
        window_weak.unwrap().set_locked_sequencer_bar(new_lock);
    });

    let cloned_context = context.clone();
    window.on_step_clicked(move |step_num| {
        let mut sound = cloned_context.sound.borrow_mut();
        sound.sequencer.set_current_step(step_num as u32);
    });

    let cloned_context = context.clone();
    let window_weak = window.as_weak();
    window.on_play_clicked(move |toggled| {
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
