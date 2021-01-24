use gameboy::apu::Apu;
use gameboy::memory::Memory;
use std::ops::BitOr;

pub struct Synth {
    pub apu: Apu,
    pub settings_ring: Vec<Vec<SetSetting>>,
    pub settings_ring_index: usize,
    pub instruments: Vec<Box<dyn Fn(&mut Vec<Vec<SetSetting>>, usize, u32) -> ()>>,
}

impl Synth {
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
    }

    pub fn trigger_instrument(&mut self, instrument: usize, freq: u32) -> () {
        let f = &self.instruments[instrument];
        f(&mut self.settings_ring, self.settings_ring_index, freq);
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

    pub fn trigger_with_length(freq: u32, length: u8) -> Vec<SetSetting> {
        vec!(
            // Length load
            SetSetting::new(Setting::new(0xff11, 0x3f), 64 - length),
            // Frequency LSB
            SetSetting::new(Setting::new(0xff13, 0xff), (freq & 0xff) as u8),
            // Trigger
            SetSetting::new(Setting::new(0xff14, 0x80), 1)
                // Length enable
                | SetSetting::new(Setting::new(0xff14, 0x40), 1)
                // Frequency MSB
                | SetSetting::new(Setting::new(0xff14, 0x07), (freq >> 8) as u8)
        )
    }

    pub fn trigger(freq: u32) -> Vec<SetSetting> {
        vec!(
            // Frequency LSB
            SetSetting::new(Setting::new(0xff13, 0xff), (freq & 0xff) as u8),
            // Trigger
            SetSetting::new(Setting::new(0xff14, 0x80), 1)
                // Length enable
                | SetSetting::new(Setting::new(0xff14, 0x40), 0)
                // Frequency MSB
                | SetSetting::new(Setting::new(0xff14, 0x07), (freq >> 8) as u8)
        )
    }

    pub fn envelope(starting_volume: u8, add_mode: u8, period: u8) -> SetSetting {
        let addr = 0xff12;
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

    pub fn duty(duty: u8) -> SetSetting {
        SetSetting::new(Setting::new(0xff11, 0xC0), duty)
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
