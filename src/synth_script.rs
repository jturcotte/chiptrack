// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::synth_script::Channel::*;
use rhai::{AST, Dynamic, Engine, Scope};
use rhai::plugin::*;
use std::cell::RefCell;
use std::ops::BitOr;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct DmgSquare {
    channel: Channel,
    nrx0_set_setting: Option<SetSetting>,
    nrx1_set_setting: Option<SetSetting>,
    nrx2_set_setting: Option<SetSetting>,
    nrx3_set_setting: Option<SetSetting>,
    nrx4_set_setting: Option<SetSetting>,
}
pub type SharedDmgSquare = Rc<RefCell<DmgSquare>>;

#[derive(Debug, Clone)]
pub struct DmgWave {
    nrx0_set_setting: Option<SetSetting>,
    nrx1_set_setting: Option<SetSetting>,
    nrx2_set_setting: Option<SetSetting>,
    nrx3_set_setting: Option<SetSetting>,
    nrx4_set_setting: Option<SetSetting>,
    wave_table_set_settings: Vec<SetSetting>,
}
pub type SharedDmgWave = Rc<RefCell<DmgWave>>;

#[derive(Debug, Clone)]
pub struct DmgNoise {
    nrx1_set_setting: Option<SetSetting>,
    nrx2_set_setting: Option<SetSetting>,
    nrx3_set_setting: Option<SetSetting>,
    nrx4_set_setting: Option<SetSetting>,
}
pub type SharedDmgNoise = Rc<RefCell<DmgNoise>>;

#[derive(Debug, Clone)]
pub struct DmgBindings {
    settings_ring: Rc<RefCell<Vec<Vec<SetSetting>>>>,
    settings_ring_index: usize,
    square1: SharedDmgSquare,
    square2: SharedDmgSquare,
    wave: SharedDmgWave,
    noise: SharedDmgNoise,
}
pub type SharedDmgBindings = Rc<RefCell<DmgBindings>>;

impl DmgBindings {
    fn set_settings_ring_index(&mut self, v: usize) {
        self.settings_ring_index = v;
    }
    fn commit_to_ring(&mut self) {
        let dest = &mut self.settings_ring.borrow_mut()[self.settings_ring_index];
        let mut square1 = self.square1.borrow_mut();
        dest.extend(square1.nrx0_set_setting.take());
        dest.extend(square1.nrx1_set_setting.take());
        dest.extend(square1.nrx2_set_setting.take());
        dest.extend(square1.nrx3_set_setting.take());
        dest.extend(square1.nrx4_set_setting.take());
        let mut square2 = self.square2.borrow_mut();
        // dest.extend(square2.nrx0_set_setting.take());
        dest.extend(square2.nrx1_set_setting.take());
        dest.extend(square2.nrx2_set_setting.take());
        dest.extend(square2.nrx3_set_setting.take());
        dest.extend(square2.nrx4_set_setting.take());
        let mut wave = self.wave.borrow_mut();
        dest.extend(wave.nrx0_set_setting.take());
        dest.extend(wave.nrx1_set_setting.take());
        dest.extend(wave.nrx2_set_setting.take());
        dest.extend(wave.nrx3_set_setting.take());
        dest.extend(wave.nrx4_set_setting.take());
        dest.extend(wave.wave_table_set_settings.drain(..));
        let mut noise = self.noise.borrow_mut();
        dest.extend(noise.nrx1_set_setting.take());
        dest.extend(noise.nrx2_set_setting.take());
        dest.extend(noise.nrx3_set_setting.take());
        dest.extend(noise.nrx4_set_setting.take());
    }
}

#[derive(Debug, Clone)]
pub enum Direction {
    Dec = 0,
    Inc = 1,
}

#[derive(Debug, Clone)]
pub enum Duty {
    Duty1_8 = 0,
    Duty1_4 = 1,
    Duty2_4 = 2,
    Duty3_4 = 3,
}

#[derive(Debug, Clone)]
pub enum WaveVolume {
    Volume0 = 0,
    Volume100 = 1,
    Volume50 = 2,
    Volume25 = 3,
}

#[derive(Debug, Clone)]
pub enum CounterWidth {
    Width15 = 0,
    Width7 = 1,
}
#[derive(Debug, Clone)]
pub enum Divisor {
    Divisor8 = 0,
    Divisor16 = 1,
    Divisor32 = 2,
    Divisor48 = 3,
    Divisor64 = 4,
    Divisor80 = 5,
    Divisor96 = 6,
    Divisor112 = 7,
}

// Only works if the function's return type is Result<_, Box<EvalAltResult>>
macro_rules! runtime_check {
    ($cond : expr, $($err : tt) +) => {
        if !$cond { 
            return Err(format!($( $err )*).into());
        };
    }
}

#[export_module]
pub mod dmg_api {

    pub const DEC: Direction = Direction::Dec;
    pub const INC: Direction = Direction::Inc;
    pub const DUTY_1_8: Duty = Duty::Duty1_8;
    pub const DUTY_1_4: Duty = Duty::Duty1_4;
    pub const DUTY_2_4: Duty = Duty::Duty2_4;
    pub const DUTY_3_4: Duty = Duty::Duty3_4;
    pub const VOLUME_0: WaveVolume = WaveVolume::Volume0;
    pub const VOLUME_100: WaveVolume = WaveVolume::Volume100;
    pub const VOLUME_50: WaveVolume = WaveVolume::Volume50;
    pub const VOLUME_25: WaveVolume = WaveVolume::Volume25;
    pub const WIDTH_15: CounterWidth = CounterWidth::Width15;
    pub const WIDTH_7: CounterWidth = CounterWidth::Width7;
    pub const DIVISOR_8: Divisor = Divisor::Divisor8;
    pub const DIVISOR_16: Divisor = Divisor::Divisor16;
    pub const DIVISOR_32: Divisor = Divisor::Divisor32;
    pub const DIVISOR_48: Divisor = Divisor::Divisor48;
    pub const DIVISOR_64: Divisor = Divisor::Divisor64;
    pub const DIVISOR_80: Divisor = Divisor::Divisor80;
    pub const DIVISOR_96: Divisor = Divisor::Divisor96;
    pub const DIVISOR_112: Divisor = Divisor::Divisor112;

    #[rhai_fn(get = "square1", pure)]
    pub fn get_square1(this_rc: &mut SharedDmgBindings) -> SharedDmgSquare {
        this_rc.borrow().square1.clone()
    }
    #[rhai_fn(get = "square2", pure)]
    pub fn get_square2(this_rc: &mut SharedDmgBindings) -> SharedDmgSquare {
        this_rc.borrow().square2.clone()
    }
    #[rhai_fn(get = "wave", pure)]
    pub fn get_wave(this_rc: &mut SharedDmgBindings) -> SharedDmgWave {
        this_rc.borrow().wave.clone()
    }
    #[rhai_fn(get = "noise", pure)]
    pub fn get_noise(this_rc: &mut SharedDmgBindings) -> SharedDmgNoise {
        this_rc.borrow().noise.clone()
    }

    #[rhai_fn(global, return_raw)]
    pub fn wait_frames(dmg: &mut SharedDmgBindings, frames: i32) -> Result<(), Box<EvalAltResult>> {
        let mut this = dmg.borrow_mut();
        this.commit_to_ring();
        let len = this.settings_ring.borrow().len();
        runtime_check!(frames >= 0, "frames must be >= 0, got {}", frames);
        runtime_check!((frames as usize) < len, "frames must be < {}, got {}", len, frames);
        this.settings_ring_index = (this.settings_ring_index + frames as usize) % len;
        Ok(())
    }

    #[rhai_fn(set = "sweep_time", pure, return_raw)]
    pub fn set_sweep_time(this_rc: &mut SharedDmgSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "sweep_time must be >= 0, got {}", v);
        runtime_check!(v < 8, "sweep_time must be < 8, got {}", v);
        let channel = this_rc.borrow().channel as u16;
        orit(
            &mut this_rc.borrow_mut().nrx0_set_setting,
            SetSetting::new(Setting::new(channel + 0, 0x70), v as u8)
            );
        Ok(())
    }
    #[rhai_fn(set = "sweep_dir", pure)]
    pub fn set_sweep_dir(this_rc: &mut SharedDmgSquare, v: Direction) {
        let channel = this_rc.borrow().channel as u16;
        orit(
            &mut this_rc.borrow_mut().nrx0_set_setting,
            SetSetting::new(Setting::new(channel + 0, 0x08), match v { Direction::Inc => 0, Direction::Dec => 1 })
            );
    }
    #[rhai_fn(set = "sweep_shift", pure, return_raw)]
    pub fn set_sweep_shift(this_rc: &mut SharedDmgSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "sweep_shift must be >= 0, got {}", v);
        runtime_check!(v < 8, "sweep_shift must be < 8, got {}", v);
        let channel = this_rc.borrow().channel as u16;
        orit(
            &mut this_rc.borrow_mut().nrx0_set_setting,
            SetSetting::new(Setting::new(channel + 0, 0x07), v as u8)
            );
        Ok(())
    }

    #[rhai_fn(set = "duty", pure)]
    pub fn set_duty(this_rc: &mut SharedDmgSquare, v: Duty) {
        let channel = this_rc.borrow().channel as u16;
        orit(
            &mut this_rc.borrow_mut().nrx1_set_setting,
            SetSetting::new(Setting::new(channel + 1, 0xC0), v as u8)
            );
    }

    #[rhai_fn(set = "env_start", pure, return_raw)]
    pub fn set_square_env_start(this_rc: &mut SharedDmgSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "env_start must be >= 0, got {}", v);
        runtime_check!(v < 16, "env_start must be < 16, got {}", v);
        let channel = this_rc.borrow().channel as u16;
        orit(
            &mut this_rc.borrow_mut().nrx2_set_setting,
            SetSetting::new(Setting::new(channel + 2, 0xf0), v as u8)
            );
        Ok(())
    }
    #[rhai_fn(set = "env_dir", pure)]
    pub fn set_square_env_dir(this_rc: &mut SharedDmgSquare, v: Direction) {
        let channel = this_rc.borrow().channel as u16;
        orit(
            &mut this_rc.borrow_mut().nrx2_set_setting,
            SetSetting::new(Setting::new(channel + 2, 0x08), v as u8)
            );
    }
    #[rhai_fn(set = "env_period", pure, return_raw)]
    pub fn set_square_env_period(this_rc: &mut SharedDmgSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "env_period must be >= 0, got {}", v);
        runtime_check!(v < 8, "env_period must be < 8, got {}", v);
        let channel = this_rc.borrow().channel as u16;
        orit(
            &mut this_rc.borrow_mut().nrx2_set_setting,
            SetSetting::new(Setting::new(channel + 2, 0x07), v as u8)
            );
        Ok(())
    }

    #[rhai_fn(global, name = "trigger_with_length", return_raw)]
    pub fn square_trigger_with_length(this_rc: &mut SharedDmgSquare, freq: f64, length: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(freq >= 64.0, "freq must be >= 64, got {}", freq);
        runtime_check!(length >= 1, "length must be >= 1, got {}", length);
        runtime_check!(length <= 64, "length must be <= 64, got {}", length);
        let gb_freq = to_square_gb_freq(freq);
        let channel = this_rc.borrow().channel as u16;
        orit(
            &mut this_rc.borrow_mut().nrx1_set_setting,
            // Length load
            SetSetting::new(Setting::new(channel + 1, 0x3f), 64 - length as u8)
            );
        orit(
            &mut this_rc.borrow_mut().nrx3_set_setting,
            // Frequency LSB
            SetSetting::new(Setting::new(channel + 3, 0xff), (gb_freq & 0xff) as u8)
            );
        orit(
            &mut this_rc.borrow_mut().nrx4_set_setting,
            // Trigger
            SetSetting::new(Setting::new(channel + 4, 0x80), 1)
                // Length enable
                | SetSetting::new(Setting::new(channel + 4, 0x40), 1)
                // Frequency MSB
                | SetSetting::new(Setting::new(channel + 4, 0x07), (gb_freq >> 8) as u8)
            );
        Ok(())
    }

    #[rhai_fn(global, name = "trigger", return_raw)]
    pub fn square_trigger(this_rc: &mut SharedDmgSquare, freq: f64) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(freq >= 64.0, "freq must be >= 64, got {}", freq);
        let gb_freq = to_square_gb_freq(freq);
        let channel = this_rc.borrow().channel as u16;
        orit(
            &mut this_rc.borrow_mut().nrx3_set_setting,
            // Frequency LSB
            SetSetting::new(Setting::new(channel + 3, 0xff), (gb_freq & 0xff) as u8)
            );
        orit(
            &mut this_rc.borrow_mut().nrx4_set_setting,
            // Trigger
            SetSetting::new(Setting::new(channel + 4, 0x80), 1)
                // Length enable
                | SetSetting::new(Setting::new(channel + 4, 0x40), 0)
                // Frequency MSB
                | SetSetting::new(Setting::new(channel + 4, 0x07), (gb_freq >> 8) as u8)
            );
        Ok(())
    }

    #[rhai_fn(set = "playing", pure)]
    pub fn set_playing(this_rc: &mut SharedDmgWave, v: bool) {
        orit(
            &mut this_rc.borrow_mut().nrx0_set_setting,
            SetSetting::new(Setting::new(Wave as u16 + 0, 0x80), v as u8)
            );
    }
    #[rhai_fn(set = "volume", pure)]
    pub fn set_volume(this_rc: &mut SharedDmgWave, v: WaveVolume) {
        orit(
            &mut this_rc.borrow_mut().nrx2_set_setting,
            SetSetting::new(Setting::new(Wave as u16 + 2, 0x60), v as u8)
            );
    }
    #[rhai_fn(set = "table", pure, return_raw)]
    pub fn set_table(this_rc: &mut SharedDmgWave, hex_string: &str) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(hex_string.len() == 32, "table must have a length of 32, got {}", hex_string.len());
        runtime_check!(hex_string.chars().all(|c| c >= '0' && c <= '9' || c >= 'a' && c <= 'f' ), "table must only contain characters [0-9a-f], got {}", hex_string);

        // Each hexadecimal character in the hex string is one 4 bits sample.
        this_rc.borrow_mut().wave_table_set_settings =
            (0..hex_string.len())
                .step_by(2)
                .map(|i| {
                    let byte = u8::from_str_radix(&hex_string[i..i + 2], 16).unwrap();
                    SetSetting::new(Setting::new((0xff30 + i / 2) as u16, 0xff), byte)
                })
                .collect();
        Ok(())
    }
    #[rhai_fn(global, name = "trigger_with_length", return_raw)]
    pub fn wave_trigger_with_length(this_rc: &mut SharedDmgWave, freq: f64, length: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(freq >= 32.0, "freq must be >= 32, got {}", freq);
        runtime_check!(length >= 1, "length must be >= 1, got {}", length);
        runtime_check!(length <= 256, "length must be <= 256, got {}", length);
        let gb_freq = to_wave_gb_freq(freq);
        orit(
            &mut this_rc.borrow_mut().nrx1_set_setting,
            // Length load
            SetSetting::new(Setting::new(Wave as u16 + 1, 0xff), (256 - length) as u8)
            );
        orit(
            &mut this_rc.borrow_mut().nrx3_set_setting,
            // Frequency LSB
            SetSetting::new(Setting::new(Wave as u16 + 3, 0xff), (gb_freq & 0xff) as u8)
            );
        orit(
            &mut this_rc.borrow_mut().nrx4_set_setting,
            // Trigger
            SetSetting::new(Setting::new(Wave as u16 + 4, 0x80), 1)
                // Length enable
                | SetSetting::new(Setting::new(Wave as u16 + 4, 0x40), 1)
                // Frequency MSB
                | SetSetting::new(Setting::new(Wave as u16 + 4, 0x07), (gb_freq >> 8) as u8)
            );
        Ok(())
    }
    #[rhai_fn(global, name = "trigger", return_raw)]
    pub fn wave_trigger(this_rc: &mut SharedDmgWave, freq: f64) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(freq >= 32.0, "freq must be >= 32, got {}", freq);
        let gb_freq = to_wave_gb_freq(freq);
        orit(
            &mut this_rc.borrow_mut().nrx3_set_setting,
            // Frequency LSB
            SetSetting::new(Setting::new(Wave as u16 + 3, 0xff), (gb_freq & 0xff) as u8)
            );
        orit(
            &mut this_rc.borrow_mut().nrx4_set_setting,
            // Trigger
            SetSetting::new(Setting::new(Wave as u16 + 4, 0x80), 1)
                // Length enable
                | SetSetting::new(Setting::new(Wave as u16 + 4, 0x40), 0)
                // Frequency MSB
                | SetSetting::new(Setting::new(Wave as u16 + 4, 0x07), (gb_freq >> 8) as u8)
            );
        Ok(())
    }


    #[rhai_fn(set = "env_start", pure, return_raw)]
    pub fn set_noise_env_start(this_rc: &mut SharedDmgNoise, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "env_start must be >= 0, got {}", v);
        runtime_check!(v < 16, "env_start must be < 16, got {}", v);
        orit(
            &mut this_rc.borrow_mut().nrx2_set_setting,
            SetSetting::new(Setting::new(Noise as u16 + 2, 0xf0), v as u8)
            );
        Ok(())
    }
    #[rhai_fn(set = "env_dir", pure)]
    pub fn set_noise_env_dir(this_rc: &mut SharedDmgNoise, v: Direction) {
        orit(
            &mut this_rc.borrow_mut().nrx2_set_setting,
            SetSetting::new(Setting::new(Noise as u16 + 2, 0x08), v as u8)
            );
    }

    #[rhai_fn(set = "env_period", pure, return_raw)]
    pub fn set_noise_env_period(this_rc: &mut SharedDmgNoise, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "env_period must be >= 0, got {}", v);
        runtime_check!(v < 8, "env_period must be < 8, got {}", v);

        orit(
            &mut this_rc.borrow_mut().nrx2_set_setting,
            SetSetting::new(Setting::new(Noise as u16 + 2, 0x07), v as u8)
            );
        Ok(())
    }
    #[rhai_fn(set = "clock_shift", pure, return_raw)]
    pub fn set_clock_shift(this_rc: &mut SharedDmgNoise, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "clock_shift must be >= 0, got {}", v);
        runtime_check!(v < 16, "clock_shift must be < 16, got {}", v);
        orit(
            &mut this_rc.borrow_mut().nrx3_set_setting,
            SetSetting::new(Setting::new(Noise as u16 + 3, 0xf0), v as u8)
            );
        Ok(())
    }
    #[rhai_fn(set = "counter_width", pure)]
    pub fn set_counter_width(this_rc: &mut SharedDmgNoise, v: CounterWidth) {
        orit(
            &mut this_rc.borrow_mut().nrx3_set_setting,
            SetSetting::new(Setting::new(Noise as u16 + 3, 0x08), v as u8)
            );
    }
    #[rhai_fn(set = "clock_divisor", pure)]
    pub fn set_clock_divisor(this_rc: &mut SharedDmgNoise, v: Divisor) {
        orit(
            &mut this_rc.borrow_mut().nrx3_set_setting,
            SetSetting::new(Setting::new(Noise as u16 + 3, 0x07), v as u8)
            );
    }
    #[rhai_fn(set = "clock_divisor", pure, return_raw)]
    pub fn set_clock_divisor_i32(this_rc: &mut SharedDmgNoise, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "clock_divisor must be >= 0, got {}", v);
        runtime_check!(v < 8, "clock_divisor must be < 8, got {}", v);
        orit(
            &mut this_rc.borrow_mut().nrx3_set_setting,
            SetSetting::new(Setting::new(Noise as u16 + 3, 0x07), v as u8)
            );
        Ok(())
    }
    #[rhai_fn(global, name = "trigger_with_length", return_raw)]
    pub fn noise_trigger_with_length(this_rc: &mut SharedDmgNoise, length: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(length >= 1, "length must be >= 1, got {}", length);
        runtime_check!(length <= 64, "length must be <= 64, got {}", length);
        orit(
            &mut this_rc.borrow_mut().nrx1_set_setting,
            // Length load
            SetSetting::new(Setting::new(Noise as u16 + 1, 0x3f), 64 - length as u8)
            );
        orit(
            &mut this_rc.borrow_mut().nrx4_set_setting,
            // Trigger
            SetSetting::new(Setting::new(Noise as u16 + 4, 0x80), 1)
                // Length enable
                | SetSetting::new(Setting::new(Noise as u16 + 4, 0x40), 1)
            );
        Ok(())
    }
    #[rhai_fn(global, name = "trigger")]
    pub fn noise_trigger(this_rc: &mut SharedDmgNoise) {
        orit(
            &mut this_rc.borrow_mut().nrx4_set_setting,
            // Trigger
            SetSetting::new(Setting::new(Noise as u16 + 4, 0x80), 1)
            );
    }

    #[rhai_fn(global)]
    pub fn to_square_gb_freq(freq: f64) -> i32 {
        2048 - (131072.0/freq).round() as i32
    }

    #[rhai_fn(global)]
    pub fn to_wave_gb_freq(freq: f64) -> i32 {
        2048 - (65536.0/freq).round() as i32
    }

    fn orit(dest: &mut Option<SetSetting>, with: SetSetting) {
        *dest = match dest.take() {
            None => Some(with),
            Some(old) => Some(old | with)
        };

    }
}

#[derive(Debug, Clone)]
pub struct Setting {
    pub addr: u16,
    pub mask: u8,
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
    Square2 = 0xff15,
    Wave = 0xff1a,
    Noise = 0xff1f,
}

#[derive(Debug, Clone)]
pub struct SetSetting {
    pub setting: Setting,
    pub value: u8,
}
impl SetSetting {
    pub fn new(setting: Setting, value: u8) -> SetSetting {
        let shifted = value << setting.mask.trailing_zeros();
        assert!(shifted & setting.mask == shifted);
        SetSetting{setting: setting, value: shifted}
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
            // Any value bit masked by both self and rhs are taken from rhs.
            value: (self.value & !rhs.setting.mask) | rhs.value,
        }
    }
}

pub struct SynthScript {
    script_engine: Engine,
    script_ast: AST,
    script_scope: Scope<'static>,
    script_context: SharedDmgBindings,
}

impl SynthScript {
    const DEFAULT_INSTRUMENTS: &'static str = include_str!("../res/instruments.rhai");

    pub fn new(settings_ring: Rc<RefCell<Vec<Vec<SetSetting>>>>) -> SynthScript {
        let mut engine = Engine::new();

        engine.register_type::<SharedDmgBindings>()
              .register_type::<SharedDmgSquare>();
        engine.register_static_module("dmg", exported_module!(dmg_api).into());

        let square1 = Rc::new(RefCell::new(
            DmgSquare {
                channel: Square1,
                nrx0_set_setting: None,
                nrx1_set_setting: None,
                nrx2_set_setting: None,
                nrx3_set_setting: None,
                nrx4_set_setting: None,
            }));
        let square2 = Rc::new(RefCell::new(
            DmgSquare {
                channel: Square2,
                nrx0_set_setting: None,
                nrx1_set_setting: None,
                nrx2_set_setting: None,
                nrx3_set_setting: None,
                nrx4_set_setting: None,
            }));
        let wave = Rc::new(RefCell::new(
            DmgWave {
                nrx0_set_setting: None,
                nrx1_set_setting: None,
                nrx2_set_setting: None,
                nrx3_set_setting: None,
                nrx4_set_setting: None,
                wave_table_set_settings: vec![],
            }));
        let noise = Rc::new(RefCell::new(
            DmgNoise {
                nrx1_set_setting: None,
                nrx2_set_setting: None,
                nrx3_set_setting: None,
                nrx4_set_setting: None,
            }));
        let dmg = Rc::new(RefCell::new(DmgBindings{
            settings_ring: settings_ring.clone(),
            settings_ring_index: 0,
            square1: square1,
            square2: square2,
            wave: wave,
            noise: noise,
            }));

        let mut scope = Scope::new();
        scope.push("dmg", dmg.clone());

        SynthScript {
            script_engine: engine,
            script_ast: Default::default(),
            script_context: dmg,
            script_scope: scope,
        }
    }

    fn default_instruments(&self) -> AST {
        self.script_engine.compile(SynthScript::DEFAULT_INSTRUMENTS).unwrap()
    }

    #[cfg(target_arch = "wasm32")]
    fn deserialize_instruments(&self, base64: String) -> Result<AST, Box<dyn std::error::Error>> {
        let decoded = crate::utils::decode_string(&base64)?;
        let ast = self.script_engine.compile(&decoded)?;
        Ok(ast)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load(&mut self, maybe_base64: Option<String>) {
        if let Some(base64) = maybe_base64 {
            let maybe_ast = self.deserialize_instruments(base64);

            match maybe_ast {
                Ok(ast) => {
                    log!("Loaded the project instruments from the URL.");
                    self.script_ast = ast;
                },
                Err(e) => {
                    elog!("Couldn't load the project instruments from the URL, using default instruments.\n\tError: {:?}", e);
                    self.script_ast = self.default_instruments();
                },
            }            
        } else {
            log!("No instruments provided in the URL, using default instruments.");
            self.script_ast = self.default_instruments();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(&mut self, project_instruments_path: &std::path::Path) {
        if project_instruments_path.exists() {
            let maybe_ast = self.script_engine.compile_file(project_instruments_path.to_path_buf());

            match maybe_ast {
                Ok(ast) => {
                    log!("Loaded project instruments from file {:?}", project_instruments_path);
                    self.script_ast = ast;
                },
                Err(e) => {
                    elog!("Couldn't load project instruments from file {:?}, using default instruments.\n\tError: {:?}", project_instruments_path, e);
                    self.script_ast = self.default_instruments();
                },
            }            
        } else {
            log!("Project instruments file {:?} doesn't exist, using default instruments.", project_instruments_path);
            self.script_ast = self.default_instruments();
        }
    }

    pub fn trigger_instrument(&mut self, settings_ring_index: usize, instrument: u32, freq: f64) -> () {
        // The script themselves are modifying this state, so reset it.
        self.script_context.borrow_mut().set_settings_ring_index(settings_ring_index);

        let result: Result<(), _> = self.script_engine.call_fn(
            &mut self.script_scope,
            &self.script_ast,
            format!("instrument_{}", instrument),
            ( freq, )
            );
        if let Err(e) = result {
            elog!("{}", e)
        }

        self.script_context.borrow_mut().commit_to_ring();
    }
}