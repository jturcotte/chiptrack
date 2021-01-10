// Note: Game BoyTM, Game Boy PocketTM, Super Game BoyTM and Game Boy ColorTM are registered trademarks of
// Nintendo CO., LTD. © 1989 to 1999 by Nintendo CO., LTD.
use cpal::{Sample, SampleFormat};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod sequencer;
mod sound;

use crate::sequencer::Sequencer;
use crate::sound::SoundStuff;
use crate::sound::SetSetting;
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

    let sequencer_bar_model = Rc::new(sixtyfps::VecModel::default());
    for _ in 0..(sequencer::NUM_STEPS/16) {
        sequencer_bar_model.push(BarData{});
    }
    let sequencer_step_model = Rc::new(sixtyfps::VecModel::default());
    for _ in 0..16 {
        sequencer_step_model.push(StepData{empty: true,});
    }


    struct Context {
        sound: Rc<RefCell<SoundStuff>>,
        _stream: cpal::Stream,
        _apu_timer: Timer,
    }

    let window = MainWindow::new();
    window.set_sequencer_bars(sixtyfps::ModelHandle::new(sequencer_bar_model.clone()));
    window.set_sequencer_steps(sixtyfps::ModelHandle::new(sequencer_step_model.clone()));

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

        let instruments: Vec<Box<dyn Fn(&mut Vec<Vec<SetSetting>>, usize, u32) -> ()>> =
            vec!(
                Box::new(move |settings_ring, i, freq| {
                    let len = settings_ring.len();
                    let f = |frame: usize| { (i + frame) % len };
                    settings_ring[f(0)].push(SetSetting::duty(0x2));
                    settings_ring[f(0)].push(SetSetting::envelope(0xf, 0x0, 0x0));
                    settings_ring[f(0)].push(SetSetting::sweep(0x0, 0x0, 0x0));
                    settings_ring[f(0)].extend(SetSetting::trigger_with_length(freq, 64));
                }),
                Box::new(move |settings_ring, i, freq| {
                    let len = settings_ring.len();
                    let f = |frame: usize| { (i + frame) % len };
                    // 2:bipp e:a:d:1 f:0:d:2 g
                    settings_ring[f(0)].push(SetSetting::duty(0x0));
                    settings_ring[f(0)].push(SetSetting::envelope(0xa, 0x0, 0x1));
                    settings_ring[f(0)].push(SetSetting::sweep(0x0, 0x1, 0x2));
                    settings_ring[f(0)].extend(SetSetting::trigger(freq));
                    // e:0:D:0 g e
                    settings_ring[f(2)].push(SetSetting::envelope(0x0, 0x0, 0x0));
                    settings_ring[f(2)].extend(SetSetting::trigger(freq));
                }),
                Box::new(move |settings_ring, i, freq| {
                    let len = settings_ring.len();
                    let f = |frame: usize| { (i + frame) % len };
                    // r:1 e:f:d:0 f:0:d:0 g
                    settings_ring[f(0)].push(SetSetting::duty(0x1));
                    settings_ring[f(0)].push(SetSetting::envelope(0xf, 0x0, 0x0));
                    settings_ring[f(0)].push(SetSetting::sweep(0x0, 0x1, 0x0));
                    settings_ring[f(0)].extend(SetSetting::trigger(freq));
                    // r:3
                    settings_ring[f(2)].push(SetSetting::duty(0x1));
                    // r:0
                    settings_ring[f(4)].push(SetSetting::duty(0x0));
                    // r:3
                    settings_ring[f(6)].push(SetSetting::duty(0x3));
                    // r:1
                    settings_ring[f(8)].push(SetSetting::duty(0x1));
                    // r:3
                    settings_ring[f(10)].push(SetSetting::duty(0x3));
                    // r:1
                    settings_ring[f(12)].push(SetSetting::duty(0x1));
                    // r:3
                    settings_ring[f(14)].push(SetSetting::duty(0x3));
                    // r:0
                    settings_ring[f(16)].push(SetSetting::duty(0x0));
                    // e:0:d:0 g
                    settings_ring[f(18)].push(SetSetting::envelope(0x0, 0x0, 0x0));
                    settings_ring[f(18)].extend(SetSetting::trigger(freq));
                }),
                Box::new(move |settings_ring, i, freq| {
                    let len = settings_ring.len();
                    let f = |frame: usize| { (i + frame) % len };
                    // 1:superdrum e:d:d:2 f:2:d:2 g e
                    settings_ring[f(0)].push(SetSetting::duty(0x0));
                    settings_ring[f(0)].push(SetSetting::envelope(0xd, 0x0, 0x2));
                    settings_ring[f(0)].push(SetSetting::sweep(0x2, 0x1, 0x2));
                    settings_ring[f(0)].extend(SetSetting::trigger(freq));
                }),
            );

        let sound_stuff = Rc::new(RefCell::new(SoundStuff {
                apu: apu,
                sequencer: Sequencer::new(window_weak, sequencer_step_model),
                settings_ring: vec![vec![]; 512],
                settings_ring_index: 0,
                selected_instrument: 0,
                instruments: instruments,
            }));

        let apu_timer: Timer = Default::default();
        let cloned_sound = sound_stuff.clone();
        apu_timer.start(
            TimerMode::Repeated,
            std::time::Duration::from_millis(15),
            Box::new(move || {
               // FIXME: Calculate based on the sample rate
               while cloned_sound.borrow().apu.buffer.lock().unwrap().len() < 1378 {
                    let mut sound = cloned_sound.borrow_mut();
                    sound.advance_frame();
                    // FIXME: Calculate from the timer rate
                    // Advance one frame (1/64th of CLOCK_FREQUENCY)
                    sound.apu.next(65536);
               }
            })
        );

        stream.play().unwrap();
        Context {
            sound: sound_stuff,
            _stream: stream,
            _apu_timer: apu_timer,
        }
    }));

    let cloned_context = context.clone();
    window.on_selected_instrument_changed(move |instrument| {
        let mut sound = cloned_context.sound.borrow_mut();
        sound.selected_instrument = instrument as usize;
    });
    let cloned_context = context.clone();
    window.on_note_pressed(move |note| {
        // let key_freq = vk * 440 / 10 + 440;
        let mut sound = cloned_context.sound.borrow_mut();
        let a = 440.0; //frequency of A (coomon value is 440Hz)
        let key_freq = (a / 32.0) * 2.0_f64.powf((note as f64 - 9.0) / 12.0);
        println!("NOTE {:?} {:?}", note, key_freq);
        let freq: u32 = 2048 - (131072.0/key_freq).round() as u32;

        sound.trigger_selected_instrument(freq);
        // FIXME: Looks like this should be delegated in a soundstuff method
        let instrument = sound.selected_instrument;
        sound.sequencer.record_trigger(instrument, freq);
    });
    let cloned_context = context.clone();
    window.on_play_pressed(move |toggled| {
        let mut sound = cloned_context.sound.borrow_mut();
        sound.sequencer.set_playing(toggled);
    });
    let cloned_context = context.clone();
    window.on_record_pressed(move |toggled| {
        let mut sound = cloned_context.sound.borrow_mut();
        sound.sequencer.set_recording(toggled);
    });

    window.run();
}
