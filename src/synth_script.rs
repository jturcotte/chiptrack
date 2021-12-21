// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::synth_script::Channel::*;
use rhai::{AST, Array, Dynamic, Engine, Scope};
use rhai::EvalAltResult::ErrorFunctionNotFound;
use rhai::plugin::*;
use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::ops::BitOr;
use std::ops::BitOrAssign;
use std::rc::Rc;

use crate::sound_engine::NUM_INSTRUMENTS;

// Only works if the function's return type is Result<_, Box<EvalAltResult>>
macro_rules! runtime_check {
    ($cond : expr, $($err : tt) +) => {
        if !$cond { 
            return Err(format!($( $err )*).into());
        };
    }
}

macro_rules! set {
    ($this_rc: ident, $v: ident, $method: ident) => {
        $this_rc.borrow_mut().$method(0, $v)
    }
}

macro_rules! set_multi {
    ($this_rc: ident, $frame_values: ident, $vtype: ty, $method: ident) => {{
        runtime_check!($frame_values.len() == 2 && $frame_values.iter().all(|d| d.is::<Array>()),
            concat!(stringify!($method), " should only be provided frame values, but got {:?}"), $frame_values);
        let timeline = $frame_values[0].clone_cast::<Array>();
        let values = $frame_values[1].clone_cast::<Array>();
        runtime_check!(timeline.len() == values.len(),
            concat!(stringify!($method), " should only be provided a timeline and values of the same length, but got {} vs {}"), timeline.len(), values.len());
        runtime_check!(timeline.iter().all(|d| d.is::<i32>()),
            concat!(stringify!($method), " should only be provided an i32 timeline, but got {:?}"), timeline);
        runtime_check!(values.iter().all(|d| d.is::<$vtype>()),
            concat!(stringify!($method), " should only be provided ", stringify!($vtype)," values, but got {:?}"), values);
        let mut this = $this_rc.borrow_mut();
        let mut index = 0;
        for (wait_frames, d) in timeline.iter().zip(values.iter()) {
            index += wait_frames.clone_cast::<i32>() as usize;
            this.$method(index, d.clone_cast::<$vtype>())?;
        }
        Ok(())
    }}
}

#[derive(Debug, Clone, Copy)]
pub enum Channel {
    Square1 = 0xff10,
    Square2 = 0xff15,
    Wave = 0xff1a,
    Noise = 0xff1f,
}

trait ScriptChannel {
    fn base(&self) -> u16;
    fn settings_ring(&mut self) -> RefMut<'_, Vec<RegSettings>>;
    fn settings_ring_index(&mut self) -> Ref<'_, usize>;

    fn get_reg_settings(&mut self, index: usize) -> RefMut<'_, RegSettings> {
        let i = *self.settings_ring_index() + index;
        RefMut::map(self.settings_ring(), |s| {
            let len = s.len();
            &mut s[i % len]
        })
    }

    fn orit(&mut self, addr: u16, with: RegSetter) {
        self.get_reg_settings(0).orit(addr, with)
    }
    fn orit_at_index(&mut self, index: usize, addr: u16, with: RegSetter) {
        self.get_reg_settings(index).orit(addr, with)
    }

    fn set_initialize(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v == 0 || v == 1, "initialize must be 0 or 1, got {}", v);        
        self.orit_at_index(index, self.base() + 4, RegSetter::new(0x80, v as u8));
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DmgSquare {
    channel: Channel,
    settings_ring: Rc<RefCell<Vec<RegSettings>>>,
    settings_ring_index: Rc<RefCell<usize>>,
}
impl DmgSquare {
    pub fn set_sweep_time(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "sweep_time must be >= 0, got {}", v);
        runtime_check!(v < 8, "sweep_time must be < 8, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 0, RegSetter::new(0x70, v as u8));
        Ok(())
    }

    pub fn set_sweep_dir(&mut self, index: usize, v: Direction) -> Result<(), Box<EvalAltResult>> {
        self.orit_at_index(index, self.channel as u16 + 0, RegSetter::new(0x08, match v { Direction::Inc => 0, Direction::Dec => 1 }));
        Ok(())
    }

    pub fn set_sweep_shift(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "sweep_shift must be >= 0, got {}", v);
        runtime_check!(v < 8, "sweep_shift must be < 8, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 0, RegSetter::new(0x07, v as u8));
        Ok(())
    }

    pub fn set_duty(&mut self, index: usize, v: Duty) -> Result<(), Box<EvalAltResult>> {
        self.orit_at_index(index, self.channel as u16 + 1, RegSetter::new(0xC0, v as u8));
        Ok(())
    }

    pub fn set_env_start(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "env_start must be >= 0, got {}", v);
        runtime_check!(v < 16, "env_start must be < 16, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 2, RegSetter::new(0xf0, v as u8));
        Ok(())
    }

    pub fn set_env_dir(&mut self, index: usize, v: Direction) -> Result<(), Box<EvalAltResult>> {
        self.orit_at_index(index, self.channel as u16 + 2, RegSetter::new(0x08, v as u8));
        Ok(())
    }
    pub fn set_env_period(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "env_period must be >= 0, got {}", v);
        runtime_check!(v < 8, "env_period must be < 8, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 2, RegSetter::new(0x07, v as u8));
        Ok(())
    }

    pub fn set_freq(&mut self, index: usize, freq: f64) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(freq >= 64.0, "freq must be >= 64, got {}", freq);
        let gb_freq = DmgSquare::to_gb_freq(freq);
        self.orit_at_index(
            index,
            self.channel as u16 + 3,
            // Frequency LSB
            RegSetter::new(0xff, (gb_freq & 0xff) as u8)
            );
        self.orit_at_index(
            index,
            self.channel as u16 + 4,
            // Frequency MSB
            RegSetter::new(0x07, (gb_freq >> 8) as u8)
            );
        Ok(())
    }

    pub fn trigger_with_length(&mut self, length: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(length >= 1, "length must be >= 1, got {}", length);
        runtime_check!(length <= 64, "length must be <= 64, got {}", length);
        self.orit(
            self.channel as u16 + 1,
            // Length load
            RegSetter::new(0x3f, 64 - length as u8)
            );
        self.orit(
            self.channel as u16 + 4,
            // Trigger
            RegSetter::new(0x80, 1)
                // Length enable
                | RegSetter::new(0x40, 1)
            );
        Ok(())
    }

    pub fn trigger(&mut self) -> Result<(), Box<EvalAltResult>> {
        self.orit(
            self.channel as u16 + 4,
            // Trigger
            RegSetter::new(0x80, 1)
                // Length enable
                | RegSetter::new(0x40, 0)
            );
        Ok(())
    }

    pub fn to_gb_freq(freq: f64) -> i32 {
        2048 - (131072.0/freq).round() as i32
    }
}
impl ScriptChannel for DmgSquare {
    fn base(&self) -> u16 { self.channel as u16 }
    fn settings_ring(&mut self) -> RefMut<'_, Vec<RegSettings>> { self.settings_ring.borrow_mut() }
    fn settings_ring_index(&mut self) -> Ref<'_, usize> { self.settings_ring_index.borrow() }
}
pub type SharedDmgSquare = Rc<RefCell<DmgSquare>>;


#[derive(Debug, Clone)]
pub struct DmgWave {
    channel: Channel,
    settings_ring: Rc<RefCell<Vec<RegSettings>>>,
    settings_ring_index: Rc<RefCell<usize>>,
}
impl DmgWave {
    pub fn set_playing(&mut self, index: usize, v: bool) -> Result<(), Box<EvalAltResult>> {
        self.orit_at_index(index, Wave as u16 + 0, RegSetter::new(0x80, v as u8));
        Ok(())
    }

    pub fn set_volume(&mut self, index: usize, v: WaveVolume) -> Result<(), Box<EvalAltResult>> {
        self.orit_at_index(index, Wave as u16 + 2, RegSetter::new(0x60, v as u8));
        Ok(())
    }

    pub fn set_table(&mut self, index: usize, hex_string: String) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(hex_string.len() == 32, "table must have a length of 32, got {}", hex_string.len());
        runtime_check!(hex_string.chars().all(|c| c >= '0' && c <= '9' || c >= 'a' && c <= 'f' ), "table must only contain characters [0-9a-f], got {}", hex_string);

        // Each hexadecimal character in the hex string is one 4 bits sample.
        for i in (0..hex_string.len()).step_by(2) {
            let byte = u8::from_str_radix(&hex_string[i..i + 2], 16).unwrap();
            self.orit_at_index(index, (0xff30 + i / 2) as u16, RegSetter::new(0xff, byte));
        }

        Ok(())
    }

    pub fn set_freq(&mut self, index: usize, freq: f64) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(freq >= 32.0, "freq must be >= 32, got {}", freq);
        let gb_freq = DmgWave::to_gb_freq(freq);
        self.orit_at_index(
            index,
            Wave as u16 + 3,
            // Frequency LSB
            RegSetter::new(0xff, (gb_freq & 0xff) as u8)
            );
        self.orit_at_index(
            index,
            Wave as u16 + 4,
            // Frequency MSB
            RegSetter::new(0x07, (gb_freq >> 8) as u8)
            );
        Ok(())
    }

    pub fn trigger_with_length(&mut self, length: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(length >= 1, "length must be >= 1, got {}", length);
        runtime_check!(length <= 256, "length must be <= 256, got {}", length);
        self.orit(
            Wave as u16 + 1,
            // Length load
            RegSetter::new(0xff, (256 - length) as u8)
            );
        self.orit(
            Wave as u16 + 4,
            // Trigger
            RegSetter::new(0x80, 1)
                // Length enable
                | RegSetter::new(0x40, 1)
            );
        Ok(())
    }

    pub fn trigger(&mut self) -> Result<(), Box<EvalAltResult>> {
        self.orit(
            Wave as u16 + 4,
            // Trigger
            RegSetter::new(0x80, 1)
                // Length enable
                | RegSetter::new(0x40, 0)
            );
        Ok(())
    }

    pub fn to_gb_freq(freq: f64) -> i32 {
        2048 - (65536.0/freq).round() as i32
    }
}
impl ScriptChannel for DmgWave {
    fn base(&self) -> u16 { self.channel as u16 }
    fn settings_ring(&mut self) -> RefMut<'_, Vec<RegSettings>> { self.settings_ring.borrow_mut() }
    fn settings_ring_index(&mut self) -> Ref<'_, usize> { self.settings_ring_index.borrow() }
}
pub type SharedDmgWave = Rc<RefCell<DmgWave>>;

#[derive(Debug, Clone)]
pub struct DmgNoise {
    channel: Channel,
    settings_ring: Rc<RefCell<Vec<RegSettings>>>,
    settings_ring_index: Rc<RefCell<usize>>,
}
impl DmgNoise {
    pub fn set_env_start(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "env_start must be >= 0, got {}", v);
        runtime_check!(v < 16, "env_start must be < 16, got {}", v);
        self.orit_at_index(index, Noise as u16 + 2, RegSetter::new(0xf0, v as u8));
        Ok(())
    }

    pub fn set_env_dir(&mut self, index: usize, v: Direction) -> Result<(), Box<EvalAltResult>> {
        self.orit_at_index(index, Noise as u16 + 2, RegSetter::new(0x08, v as u8));
        Ok(())
    }

    pub fn set_env_period(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "env_period must be >= 0, got {}", v);
        runtime_check!(v < 8, "env_period must be < 8, got {}", v);
        self.orit_at_index(index, Noise as u16 + 2, RegSetter::new(0x07, v as u8));
        Ok(())
    }

    pub fn set_clock_shift(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "clock_shift must be >= 0, got {}", v);
        runtime_check!(v < 16, "clock_shift must be < 16, got {}", v);
        self.orit_at_index(index, Noise as u16 + 3, RegSetter::new(0xf0, v as u8));
        Ok(())
    }

    pub fn set_counter_width(&mut self, index: usize, v: CounterWidth) -> Result<(), Box<EvalAltResult>> {
        self.orit_at_index(index, Noise as u16 + 3, RegSetter::new(0x08, v as u8));
        Ok(())
    }

    pub fn set_clock_divisor(&mut self, index: usize, v: Divisor) -> Result<(), Box<EvalAltResult>> {
        self.orit_at_index(index, Noise as u16 + 3, RegSetter::new(0x07, v as u8));
        Ok(())
    }

    pub fn set_clock_divisor_i32(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "clock_divisor must be >= 0, got {}", v);
        runtime_check!(v < 8, "clock_divisor must be < 8, got {}", v);
        self.orit_at_index(index, Noise as u16 + 3, RegSetter::new(0x07, v as u8));
        Ok(())
    }

    pub fn trigger_with_length(&mut self, length: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(length >= 1, "length must be >= 1, got {}", length);
        runtime_check!(length <= 64, "length must be <= 64, got {}", length);
        self.orit(
            Noise as u16 + 1,
            // Length load
            RegSetter::new(0x3f, 64 - length as u8)
            );
        self.orit(
            Noise as u16 + 4,
            // Trigger
            RegSetter::new(0x80, 1)
                // Length enable
                | RegSetter::new(0x40, 1)
            );
        Ok(())
    }

    pub fn trigger(&mut self) -> Result<(), Box<EvalAltResult>> {
        self.orit(
            Noise as u16 + 4,
            // Trigger
            RegSetter::new(0x80, 1)
            );
        Ok(())
    }
}    
impl ScriptChannel for DmgNoise {
    fn base(&self) -> u16 { self.channel as u16 }
    fn settings_ring(&mut self) -> RefMut<'_, Vec<RegSettings>> { self.settings_ring.borrow_mut() }
    fn settings_ring_index(&mut self) -> Ref<'_, usize> { self.settings_ring_index.borrow() }
}
pub type SharedDmgNoise = Rc<RefCell<DmgNoise>>;

#[derive(Debug, Clone)]
pub struct DmgBindings {
    settings_ring: Rc<RefCell<Vec<RegSettings>>>,
    settings_ring_index: Rc<RefCell<usize>>,
    square1: SharedDmgSquare,
    square2: SharedDmgSquare,
    wave: SharedDmgWave,
    noise: SharedDmgNoise,
}
pub type SharedDmgBindings = Rc<RefCell<DmgBindings>>;

impl DmgBindings {
    fn set_settings_ring_index(&mut self, v: usize) {
        *self.settings_ring_index.borrow_mut() = v;
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


    /// Just a clearer wrapper for [[t1, t2, ...], [v1, v2, ...]], which is what multi setters expect.
    #[rhai_fn(global, name = "frames", return_raw)]
    pub fn frames(timeline: Array, values: Array) -> Result<Array, Box<EvalAltResult>> {
        Ok(vec![timeline.into(), values.into()])
    }

    #[rhai_fn(global)]
    pub fn to_square_gb_freq(freq: f64) -> i32 {
        DmgSquare::to_gb_freq(freq)
    }

    #[rhai_fn(global)]
    pub fn to_wave_gb_freq(freq: f64) -> i32 {
        DmgSquare::to_gb_freq(freq)
    }

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
        let this = dmg.borrow_mut();
        let len = this.settings_ring.borrow().len();
        runtime_check!(frames >= 0, "frames must be >= 0, got {}", frames);
        runtime_check!((frames as usize) < len, "frames must be < {}, got {}", len, frames);
        let mut settings_ring_index = this.settings_ring_index.borrow_mut();
        *settings_ring_index = (*settings_ring_index + frames as usize) % len;
        Ok(())
    }

    #[rhai_fn(set = "sweep_time", pure, return_raw)]
    pub fn set_sweep_time(this_rc: &mut SharedDmgSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_sweep_time)
    }
    #[rhai_fn(set = "sweep_time", pure, return_raw)]
    pub fn set_multi_sweep_time(this_rc: &mut SharedDmgSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_sweep_time)
    }

    #[rhai_fn(set = "sweep_dir", pure, return_raw)]
    pub fn set_sweep_dir(this_rc: &mut SharedDmgSquare, v: Direction) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_sweep_dir)
    }
    #[rhai_fn(set = "sweep_dir", pure, return_raw)]
    pub fn set_multi_sweep_dir(this_rc: &mut SharedDmgSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, Direction, set_sweep_dir)
    }

    #[rhai_fn(set = "sweep_shift", pure, return_raw)]
    pub fn set_sweep_shift(this_rc: &mut SharedDmgSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_sweep_shift)
    }
    #[rhai_fn(set = "sweep_shift", pure, return_raw)]
    pub fn set_multi_sweep_shift(this_rc: &mut SharedDmgSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_sweep_shift)
    }

    #[rhai_fn(set = "duty", pure, return_raw)]
    pub fn set_duty(this_rc: &mut SharedDmgSquare, v: Duty) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_duty)
    }
    #[rhai_fn(set = "duty", pure, return_raw)]
    pub fn set_multi_duty(this_rc: &mut SharedDmgSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, Duty, set_duty)
    }

    #[rhai_fn(set = "env_start", pure, return_raw)]
    pub fn set_square_env_start(this_rc: &mut SharedDmgSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_env_start)
    }
    #[rhai_fn(set = "env_start", pure, return_raw)]
    pub fn set_multi_square_env_start(this_rc: &mut SharedDmgSquare, values: Vec<Dynamic>) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_env_start)
    }

    #[rhai_fn(set = "env_dir", pure, return_raw)]
    pub fn set_square_env_dir(this_rc: &mut SharedDmgSquare, v: Direction) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_env_dir)
    }
    #[rhai_fn(set = "env_dir", pure, return_raw)]
    pub fn set_multi_square_env_dir(this_rc: &mut SharedDmgSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, Direction, set_env_dir)
    }

    #[rhai_fn(set = "env_period", pure, return_raw)]
    pub fn set_square_env_period(this_rc: &mut SharedDmgSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_env_period)
    }
    #[rhai_fn(set = "env_period", pure, return_raw)]
    pub fn set_multi_square_env_period(this_rc: &mut SharedDmgSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_env_period)
    }

    #[rhai_fn(set = "freq", pure, return_raw)]
    pub fn set_square_freq(this_rc: &mut SharedDmgSquare, v: f64) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_freq)
    }
    #[rhai_fn(set = "freq", pure, return_raw)]
    pub fn set_multi_square_freq(this_rc: &mut SharedDmgSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, f64, set_freq)
    }

    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_square_initialize(this_rc: &mut SharedDmgSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_initialize)
    }
    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_square_initialize_bool(this_rc: &mut SharedDmgSquare, b: bool) -> Result<(), Box<EvalAltResult>> {
        let v = b as i32;
        set!(this_rc, v, set_initialize)
    }
    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_multi_square_initialize(this_rc: &mut SharedDmgSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_initialize)
    }

    #[rhai_fn(global, name = "trigger_with_length", return_raw)]
    pub fn square_trigger_with_length(this_rc: &mut SharedDmgSquare, length: i32) -> Result<(), Box<EvalAltResult>> {
        this_rc.borrow_mut().trigger_with_length(length)
    }

    #[rhai_fn(global, name = "trigger", return_raw)]
    pub fn square_trigger(this_rc: &mut SharedDmgSquare) -> Result<(), Box<EvalAltResult>> {
        this_rc.borrow_mut().trigger()
    }

    #[rhai_fn(set = "playing", pure, return_raw)]
    pub fn set_playing(this_rc: &mut SharedDmgWave, v: bool) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_playing)
    }
    #[rhai_fn(set = "playing", pure, return_raw)]
    pub fn set_multi_playing(this_rc: &mut SharedDmgWave, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, bool, set_playing)
    }

    #[rhai_fn(set = "volume", pure, return_raw)]
    pub fn set_volume(this_rc: &mut SharedDmgWave, v: WaveVolume) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_volume)
    }
    #[rhai_fn(set = "volume", pure, return_raw)]
    pub fn set_multi_volume(this_rc: &mut SharedDmgWave, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, WaveVolume, set_volume)
    }

    #[rhai_fn(set = "table", pure, return_raw)]
    pub fn set_table(this_rc: &mut SharedDmgWave, v: String) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_table)
    }
    #[rhai_fn(set = "table", pure, return_raw)]
    pub fn set_multi_table(this_rc: &mut SharedDmgWave, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, String, set_table)
    }

    #[rhai_fn(set = "freq", pure, return_raw)]
    pub fn set_wave_freq(this_rc: &mut SharedDmgWave, v: f64) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_freq)
    }
    #[rhai_fn(set = "freq", pure, return_raw)]
    pub fn set_multi_wave_freq(this_rc: &mut SharedDmgWave, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, f64, set_freq)
    }

    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_wave_initialize(this_rc: &mut SharedDmgWave, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_initialize)
    }
    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_wave_initialize_bool(this_rc: &mut SharedDmgWave, b: bool) -> Result<(), Box<EvalAltResult>> {
        let v = b as i32;
        set!(this_rc, v, set_initialize)
    }
    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_multi_wave_initialize(this_rc: &mut SharedDmgWave, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_initialize)
    }

    #[rhai_fn(global, name = "trigger_with_length", return_raw)]
    pub fn wave_trigger_with_length(this_rc: &mut SharedDmgWave, length: i32) -> Result<(), Box<EvalAltResult>> {
        this_rc.borrow_mut().trigger_with_length(length)
    }
    #[rhai_fn(global, name = "trigger", return_raw)]
    pub fn wave_trigger(this_rc: &mut SharedDmgWave) -> Result<(), Box<EvalAltResult>> {
        this_rc.borrow_mut().trigger()
    }


    #[rhai_fn(set = "env_start", pure, return_raw)]
    pub fn set_noise_env_start(this_rc: &mut SharedDmgNoise, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_env_start)
    }
    #[rhai_fn(set = "env_start", pure, return_raw)]
    pub fn set_multi_noise_env_start(this_rc: &mut SharedDmgNoise, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_env_start)
    }

    #[rhai_fn(set = "env_dir", pure, return_raw)]
    pub fn set_noise_env_dir(this_rc: &mut SharedDmgNoise, v: Direction) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_env_dir)
    }
    #[rhai_fn(set = "env_dir", pure, return_raw)]
    pub fn set_multi_noise_env_dir(this_rc: &mut SharedDmgNoise, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, Direction, set_env_dir)
    }

    #[rhai_fn(set = "env_period", pure, return_raw)]
    pub fn set_noise_env_period(this_rc: &mut SharedDmgNoise, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_env_period)
    }
    #[rhai_fn(set = "env_period", pure, return_raw)]
    pub fn set_multi_noise_env_period(this_rc: &mut SharedDmgNoise, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_env_period)
    }

    #[rhai_fn(set = "clock_shift", pure, return_raw)]
    pub fn set_clock_shift(this_rc: &mut SharedDmgNoise, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_clock_shift)
    }
    #[rhai_fn(set = "clock_shift", pure, return_raw)]
    pub fn set_multi_clock_shift(this_rc: &mut SharedDmgNoise, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_clock_shift)
    }

    #[rhai_fn(set = "counter_width", pure, return_raw)]
    pub fn set_counter_width(this_rc: &mut SharedDmgNoise, v: CounterWidth) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_counter_width)
    }
    #[rhai_fn(set = "counter_width", pure, return_raw)]
    pub fn set_multi_counter_width(this_rc: &mut SharedDmgNoise, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, CounterWidth, set_counter_width)
    }

    #[rhai_fn(set = "clock_divisor", pure, return_raw)]
    pub fn set_clock_divisor(this_rc: &mut SharedDmgNoise, v: Divisor) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_clock_divisor)
    }
    #[rhai_fn(set = "clock_divisor", pure, return_raw)]
    pub fn set_multi_clock_divisor(this_rc: &mut SharedDmgNoise, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, Divisor, set_clock_divisor)
    }
    #[rhai_fn(set = "clock_divisor", pure, return_raw)]
    pub fn set_clock_divisor_i32(this_rc: &mut SharedDmgNoise, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_clock_divisor_i32)
    }

    // FIXME: Should it be i32 or should it be bool?
    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_noise_initialize(this_rc: &mut SharedDmgNoise, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_initialize)
    }
    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_noise_initialize_bool(this_rc: &mut SharedDmgNoise, b: bool) -> Result<(), Box<EvalAltResult>> {
        let v = b as i32;
        set!(this_rc, v, set_initialize)
    }
    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_multi_noise_initialize(this_rc: &mut SharedDmgNoise, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_initialize)
    }

    #[rhai_fn(global, name = "trigger_with_length", return_raw)]
    pub fn noise_trigger_with_length(this_rc: &mut SharedDmgNoise, length: i32) -> Result<(), Box<EvalAltResult>> {
        this_rc.borrow_mut().trigger_with_length(length)
    }
    #[rhai_fn(global, name = "trigger", return_raw)]
    pub fn noise_trigger(this_rc: &mut SharedDmgNoise) -> Result<(), Box<EvalAltResult>> {
        this_rc.borrow_mut().trigger()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RegSetter {
    pub mask: u8,
    pub value: u8,
}
impl RegSetter {
    pub fn new(mask: u8, value: u8) -> RegSetter {
        let shifted = value << mask.trailing_zeros();
        assert!(shifted & mask == shifted);
        RegSetter {mask: mask, value: shifted}
    }
    const EMPTY: RegSetter = RegSetter {mask: 0, value: 0};
}
impl BitOr for RegSetter {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        RegSetter {
            mask: self.mask | rhs.mask,
            // Any value bit masked by both self and rhs are taken from rhs.
            value: (self.value & !rhs.mask) | rhs.value,
        }
    }
}
impl BitOrAssign for RegSetter {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs
    }
}

#[derive(Debug, Clone)]
pub struct RegSettings {
    // FF10 to FF3F
    registers: [RegSetter; 48],
}

impl RegSettings {
    pub fn new() -> RegSettings {
        RegSettings { registers: [RegSetter::EMPTY; 48] }
    }

    pub fn orit(&mut self, addr: u16, with: RegSetter) {
        let index = addr as usize - 0xff10;
        let dest = &mut self.registers[index];
        *dest |= with;
    }

    pub fn for_each_setting<F>(&self, mut f: F)
    where
        F: FnMut(u16, RegSetter),
    {
        self.registers.iter()
            .enumerate()
            .filter_map(|(a, &r)| if r.mask != 0 { Some((a as u16 + 0xff10, r)) } else { None } )
            .for_each(|(a, r)| f(a, r))
    }

    pub fn clear(&mut self) {
        self.registers = [RegSetter::EMPTY; 48];
    }
}

pub struct SynthScript {
    script_engine: Engine,
    script_ast: AST,
    script_scope: Scope<'static>,
    script_context: SharedDmgBindings,
    instrument_ids: Vec<String>,
}

impl SynthScript {
    const DEFAULT_INSTRUMENTS: &'static str = include_str!("../res/instruments.rhai");

    pub fn new(settings_ring: Rc<RefCell<Vec<RegSettings>>>) -> SynthScript {
        let mut engine = Engine::new();

        engine.register_type::<SharedDmgBindings>()
              .register_type::<SharedDmgSquare>();
        engine.register_static_module("dmg", exported_module!(dmg_api).into());

        let settings_ring_index = Rc::new(RefCell::new(0));
        let square1 = Rc::new(RefCell::new(
            DmgSquare {
                channel: Square1,
                settings_ring: settings_ring.clone(),
                settings_ring_index: settings_ring_index.clone(),
            }));
        let square2 = Rc::new(RefCell::new(
            DmgSquare {
                channel: Square2,
                settings_ring: settings_ring.clone(),
                settings_ring_index: settings_ring_index.clone(),
            }));
        let wave = Rc::new(RefCell::new(
            DmgWave {
                channel: Wave,
                settings_ring: settings_ring.clone(),
                settings_ring_index: settings_ring_index.clone(),
            }));
        let noise = Rc::new(RefCell::new(
            DmgNoise {
                channel: Noise,
                settings_ring: settings_ring.clone(),
                settings_ring_index: settings_ring_index.clone(),
            }));
        let dmg = Rc::new(RefCell::new(DmgBindings{
            settings_ring: settings_ring.clone(),
            settings_ring_index: settings_ring_index,
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
            instrument_ids: Vec::new(),
        }
    }

    pub fn instrument_ids(&self) -> &Vec<String> {
        &self.instrument_ids
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
        self.extract_instrument_ids();
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
        self.extract_instrument_ids();
    }

    pub fn trigger_instrument(&mut self, settings_ring_index: usize, instrument: u32, freq: f64) -> () {
        // The script themselves are modifying this state, so reset it.
        self.script_context.borrow_mut().set_settings_ring_index(settings_ring_index);

        let result: Result<(), _> = self.script_engine.call_fn(
            &mut self.script_scope,
            &self.script_ast,
            format!("instrument_{}", instrument + 1),
            ( freq, )
            );
        if let Err(e) = result {
            elog!("{}", e)
        }
    }

    pub fn extract_instrument_ids(&mut self) {
        let defined_instruments: Vec<usize> =
            self.script_ast.iter_functions()
                .filter_map(|f|
                    if f.name.starts_with("instrument_") {
                        f.name.get(11..).and_then(|s| s.parse().ok())
                    } else {
                        None
                    }
                ).collect();
        self.instrument_ids = (1 .. NUM_INSTRUMENTS + 1).map(|i| {
            let function_name = format!("instrument_id_{}", i);

            #[cfg(not(target_arch = "wasm32"))]
            let id_result = self.script_engine.call_fn(
                &mut self.script_scope,
                &self.script_ast,
                &function_name,
                ()
                );
            // FIXME: Emojis don't seem to show up in the browser,
            //        use the default number IDs.
            #[cfg(target_arch = "wasm32")]
            let id_result: Result<String, Box<rhai::EvalAltResult>> =
                Err(Box::new(ErrorFunctionNotFound(function_name.clone(), Position::NONE)));

            match id_result {
                    Ok(id) => id,
                    Err(e) => {
                        match *e {
                            ErrorFunctionNotFound(f, _) if f == function_name => {
                                if defined_instruments.contains(&i) {
                                    i.to_string()
                                } else {
                                    "".to_string()
                                }
                            }
                            other => {
                                elog!("Error calling {}:\n\t{:?}", function_name, other);
                                i.to_string()                                
                            }
                        }
                    },
                }
            })
            .collect();
    }
}