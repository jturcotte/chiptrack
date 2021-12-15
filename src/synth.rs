// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::MainWindow;
use crate::synth_script::Channel;
use crate::synth_script::SetSetting;
use crate::synth_script::SynthScript;
use sixtyfps::Model;
use sixtyfps::SharedString;
use sixtyfps::Weak;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use std::rc::Rc;

pub struct Synth {
    dmg: rboy::Sound,
    script: SynthScript,
    settings_ring: Rc<RefCell<Vec<Vec<SetSetting>>>>,
    settings_ring_index: usize,
    buffer: Arc<Mutex<Vec<f32>>>,
    buffer_wave_start: Arc<Mutex<Option<usize>>>,
    main_window: Weak<MainWindow>,
}

struct FakePlayer {
    sample_rate: u32,
    buffer: Arc<Mutex<Vec<f32>>>,
    buffer_wave_start: Arc<Mutex<Option<usize>>>,
}

impl rboy::AudioPlayer for FakePlayer {
    fn play(&mut self, left_channel: &[f32], right_channel: &[f32], buffer_wave_start: Option<usize>) {
        let mut left_iter = left_channel.iter();
        let mut right_iter = right_channel.iter();
        let mut vec = self.buffer.lock().unwrap();
        let mut wave_start = self.buffer_wave_start.lock().unwrap();
        *wave_start = wave_start.or(buffer_wave_start.map(|s| s * 2 + vec.len()));

        vec.reserve(left_channel.len() * 2);
        while let Some(left) = left_iter.next() {
            let right = right_iter.next().unwrap();
            vec.push(*left);
            vec.push(*right);
        }
    }
    fn samples_rate(&self) -> u32 {
        self.sample_rate
    }
    fn underflowed(&self) -> bool {
        // We're always underflowed, we advance frames when we need to fill the audio buffer.
        true
    }
}

impl Synth {
    pub fn new(main_window: Weak<MainWindow>, sample_rate: u32) -> Synth {
        let settings_ring = Rc::new(RefCell::new(vec![vec![]; 1048576]));
        let script = SynthScript::new(settings_ring.clone());

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let buffer_wave_start = Arc::new(Mutex::new(None));

        let player = Box::new(FakePlayer{
            sample_rate: sample_rate,
            buffer: buffer.clone(),
            buffer_wave_start: buffer_wave_start.clone(),
        });
        let mut dmg = rboy::Sound::new(player);
        // Already power it on.
        dmg.wb( 0xff26, 0x80 );

        Synth {
            dmg: dmg,
            script: script,
            settings_ring: settings_ring,
            settings_ring_index: 0,
            buffer: buffer,
            buffer_wave_start: buffer_wave_start,
            main_window: main_window,
        }
    }

    pub fn buffer(&self) -> Arc<Mutex<Vec<f32>>> {
        self.buffer.clone()
    }
    pub fn buffer_wave_start(&self) -> Arc<Mutex<Option<usize>>> {
        self.buffer_wave_start.clone()
    }
    pub fn reset_buffer_wave_start(&mut self) {
        *self.buffer_wave_start.lock().unwrap() = None;
    }

    // The Gameboy APU has 512 frames per second where various registers are read,
    // but all registers are eventually read at least once every 8 of those frames.
    // So clock our frame generation at 64hz, thus this function is expected
    // to be called 64x per second.
    pub fn advance_frame(&mut self) -> () {
        let mut settings_ring = self.settings_ring.borrow_mut();
        let i = self.settings_ring_index;
        for set in settings_ring[i].iter() {
            let prev = self.dmg.rb(set.setting.addr);
            let new = prev & !set.setting.mask | set.value as u8;
            self.dmg.wb(set.setting.addr, new);    
        }
        settings_ring[i].clear();
        self.settings_ring_index = (self.settings_ring_index + 1) % settings_ring.len();

        // Just enable all channels for now
        self.dmg.wb(0xff24, 0xff);    
        self.dmg.wb(0xff25, 0xff);    

        // Generate one frame of mixed output.
        // For 44100hz audio, this will put 44100/64 audio samples in self.buffer.
        self.dmg.do_cycle(rboy::CLOCKS_PER_SECOND / 64)
    }

    pub fn trigger_instrument(&mut self, instrument: u32, freq: f64) -> () {
        self.script.trigger_instrument(self.settings_ring_index, instrument, freq);
    }

    /// Can be used to manually mute when instruments have an infinite length and envelope.
    pub fn mute_instruments(&mut self) {
        // Set the envelopes to 0.
        self.dmg.wb(Channel::Square1 as u16 + 2, 0);    
        self.dmg.wb(Channel::Square2 as u16 + 2, 0);    
        self.dmg.wb(Channel::Wave as u16 + 2, 0);    
        self.dmg.wb(Channel::Noise as u16 + 2, 0);    
    }

    pub fn ready_buffer_samples(&self) -> usize {
        self.buffer.lock().unwrap().len()
    }

    fn update_instrument_ids(&self) {
        let ids = self.script.instrument_ids().clone();
        self.main_window.clone().upgrade_in_event_loop(move |handle| {
            let model = handle.get_instruments();
            for (i, id) in ids.iter().enumerate() {
                let mut row_data = model.row_data(i);
                row_data.id = SharedString::from(id);
                model.set_row_data(i, row_data);
            }
        });
    }
    #[cfg(target_arch = "wasm32")]
    pub fn load(&mut self, maybe_base64: Option<String>) {
        self.script.load(maybe_base64);
        self.update_instrument_ids();
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(&mut self, project_instruments_path: &std::path::Path) {
        self.script.load(project_instruments_path);
        self.update_instrument_ids();
    }

}
