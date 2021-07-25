use gameboy::apu::Apu;
use gameboy::memory::Memory;
use std::ops::BitOr;

pub struct Synth {
    apu: Apu,
    settings_ring: Vec<Vec<SetSetting>>,
    settings_ring_index: usize,
    instruments: Vec<Box<dyn Fn(&mut Vec<Vec<SetSetting>>, usize, u32) -> ()>>,
}

impl Synth {
    pub fn new(mut apu: Apu, instruments: Vec<Box<dyn Fn(&mut Vec<Vec<SetSetting>>, usize, u32) -> ()>>) -> Synth {
        // Already power it on.
        apu.set( 0xff26, 0x80 );

        Synth {
            apu: apu,
            settings_ring: vec![vec![]; 512],
            settings_ring_index: 0,
            instruments: instruments,
        }
    }

    // The Gameboy APU has 512 frames per second where various registers are read,
    // but all registers are eventually read at least once every 8 of those frames.
    // So clock our frame generation at 64hz, thus this function is expected
    // to be called 64x per second.
    pub fn advance_frame(&mut self) -> () {
        let i = self.settings_ring_index;
        for set in self.settings_ring[i].iter() {
            let prev = self.apu.get(set.setting.addr);
            let new = prev & !set.setting.mask | set.value as u8;
            self.apu.set(set.setting.addr, new);
            println!("Setting {:x?} Value {:x?} Prev {:x?} New {:x?}", set.setting, set.value, prev, new);
        }
        self.settings_ring[i].clear();
        self.settings_ring_index = (self.settings_ring_index + 1) % self.settings_ring.len();

        self.apu.set( 0xff24, 0xff );
        self.apu.set( 0xff25, 0xff );

        // Generate one frame of mixed output.
        // For 44100hz audio, this will put 44100/64 audio samples in self.apu.buffer.
        self.apu.next(gameboy::cpu::CLOCK_FREQUENCY / 64);
    }

    pub fn trigger_instrument(&mut self, instrument: u32, freq: f64) -> () {
        let gb_freq = 2048 - (131072.0/freq).round() as u32;
        let f = &self.instruments[instrument as usize];
        f(&mut self.settings_ring, self.settings_ring_index, gb_freq);
    }

    pub fn ready_buffer_samples(&self) -> usize {
        self.apu.buffer.lock().unwrap().len()
    }

}

#[derive(Debug, Clone)]
pub struct Setting {
    addr: u16, mask: u8,
}
impl Setting {
    pub fn new(addr: u16, mask: u8) -> Setting {
        Setting {
            addr: addr, mask: mask,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Channel {
    Square1 = 0xff10,
    // Square2 = 0xff15,
    Wave = 0xff1a,
    // Noise = 0xff1f,
}

#[derive(Clone)]
pub struct SetSetting {
    setting: Setting,
    value: u8,
}
impl SetSetting {
    pub fn new(setting: Setting, value: u8) -> SetSetting {
        let shifted = value << setting.mask.trailing_zeros();
        assert!(shifted & setting.mask == shifted);
        SetSetting{setting: setting, value: shifted}
    }

    pub fn trigger_with_length(freq: u32, length: u8, channel: Channel) -> Vec<SetSetting> {
        vec!(
            // Length load
            SetSetting::new(Setting::new(channel as u16 + 1, 0x3f), 64 - length),
            // Frequency LSB
            SetSetting::new(Setting::new(channel as u16 + 3, 0xff), (freq & 0xff) as u8),
            // Trigger
            SetSetting::new(Setting::new(channel as u16 + 4, 0x80), 1)
                // Length enable
                | SetSetting::new(Setting::new(channel as u16 + 4, 0x40), 1)
                // Frequency MSB
                | SetSetting::new(Setting::new(channel as u16 + 4, 0x07), (freq >> 8) as u8)
        )
    }

    pub fn trigger(freq: u32, channel: Channel) -> Vec<SetSetting> {
        vec!(
            // Frequency LSB
            SetSetting::new(Setting::new(channel as u16 + 3, 0xff), (freq & 0xff) as u8),
            // Trigger
            SetSetting::new(Setting::new(channel as u16 + 4, 0x80), 1)
                // Length enable
                | SetSetting::new(Setting::new(channel as u16 + 4, 0x40), 0)
                // Frequency MSB
                | SetSetting::new(Setting::new(channel as u16 + 4, 0x07), (freq >> 8) as u8)
        )
    }

    pub fn envelope(starting_volume: u8, add_mode: u8, period: u8, channel: Channel) -> SetSetting {
        let addr = channel as u16 + 2;
        SetSetting::new(Setting::new(addr, 0xf0), starting_volume)
        | SetSetting::new(Setting::new(addr, 0x08), add_mode)
        | SetSetting::new(Setting::new(addr, 0x07), period)
    }

    pub fn sweep(period: u8, negate: u8, shift: u8) -> SetSetting {
        let addr = 0xff10;
        SetSetting::new(Setting::new(addr, 0x70), period)
        | SetSetting::new(Setting::new(addr, 0x08), negate)
        | SetSetting::new(Setting::new(addr, 0x07), shift)
    }

    pub fn duty(duty: u8, channel: Channel) -> SetSetting {
        SetSetting::new(Setting::new(channel as u16 + 1, 0xC0), duty)
    }


    pub fn wave_power(on: u8) -> SetSetting {
        SetSetting::new(Setting::new(0xff1a, 0x80), on)
    }

    pub fn wave_volume_code(volume_code: u8) -> SetSetting {
        SetSetting::new(Setting::new(0xff1c, 0x60), volume_code)
    }

    // Each hexadecimal character in the hex string is one 4 bits sample.
    pub fn wave_table(hex_string: &str) -> Vec<SetSetting> {
        (0..hex_string.len())
            .step_by(2)
            .map(|i| {
                let byte = u8::from_str_radix(&hex_string[i..i + 2], 16).unwrap();
                SetSetting::new(Setting::new((0xff30 + i / 2) as u16, 0xff), byte)
            })
            .collect()
    }
}
impl BitOr for SetSetting {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        assert!(self.setting.addr == rhs.setting.addr);
        SetSetting {
            setting: Setting {
                addr: self.setting.addr | rhs.setting.addr,
                mask: self.setting.mask | rhs.setting.mask,
            },
            value: self.value | rhs.value,
        }
    }
}
