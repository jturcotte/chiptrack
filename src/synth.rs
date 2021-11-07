use crate::synth_script::SynthScript;
use crate::synth_script::SetSetting;
// use crate::synth::Channel::*;
use gameboy::apu::Apu;
use gameboy::memory::Memory;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use std::rc::Rc;

pub struct Synth {
    apu: Apu,
    script: SynthScript,
    settings_ring: Rc<RefCell<Vec<Vec<SetSetting>>>>,
    settings_ring_index: usize,
}

impl Synth {
    pub fn new(mut apu: Apu) -> Synth {
        // Already power it on.
        apu.set( 0xff26, 0x80 );
        let settings_ring = Rc::new(RefCell::new(vec![vec![]; 512]));
        let script = SynthScript::new(settings_ring.clone());

        Synth {
            apu: apu,
            script: script,
            settings_ring: settings_ring,
            settings_ring_index: 0,
        }
    }

    pub fn buffer(&self) -> Arc<Mutex<Vec<(f32, f32)>>> {
        self.apu.buffer.clone()
    }
    pub fn buffer_wave_start(&self) -> usize {
        self.apu.buffer_wave_start
    }
    pub fn reset_buffer_wave_start(&mut self) {
        self.apu.buffer_wave_start = usize::MAX;
    }

    // The Gameboy APU has 512 frames per second where various registers are read,
    // but all registers are eventually read at least once every 8 of those frames.
    // So clock our frame generation at 64hz, thus this function is expected
    // to be called 64x per second.
    pub fn advance_frame(&mut self) -> () {
        let mut settings_ring = self.settings_ring.borrow_mut();
        let i = self.settings_ring_index;
        for set in settings_ring[i].iter() {
            let prev = self.apu.get(set.setting.addr);
            let new = prev & !set.setting.mask | set.value as u8;
            self.apu.set(set.setting.addr, new);
            println!("Setting {:x?} Value {:x?} Prev {:x?} New {:x?}", set.setting, set.value, prev, new);
        }
        settings_ring[i].clear();
        self.settings_ring_index = (self.settings_ring_index + 1) % settings_ring.len();

        self.apu.set( 0xff24, 0xff );
        self.apu.set( 0xff25, 0xff );

        // Generate one frame of mixed output.
        // For 44100hz audio, this will put 44100/64 audio samples in self.apu.buffer.
        self.apu.next(gameboy::cpu::CLOCK_FREQUENCY / 64);
    }

    pub fn trigger_instrument(&mut self, instrument: u32, freq: f64) -> () {
        self.script.trigger_instrument(self.settings_ring_index, instrument, freq);
    }

    pub fn ready_buffer_samples(&self) -> usize {
        self.apu.buffer.lock().unwrap().len()
    }

}
