// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::synth_script::Channel::*;
use rhai::plugin::*;
use rhai::{Array, Dynamic, Engine, FnPtr, Map, Scope, AST};
use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::error::Error;
use std::ops::BitOr;
use std::ops::BitOrAssign;
use std::ops::Range;
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
    };
}

macro_rules! set_multi {
    ($this_rc: ident, $frame_values: ident, $vtype: ty, $method: ident) => {{
        runtime_check!(
            $frame_values.len() == 2 && $frame_values.iter().all(|d| d.is::<Array>()),
            concat!(
                stringify!($method),
                " should only be provided frame values, but got {:?}"
            ),
            $frame_values
        );
        let timeline = $frame_values[0].clone_cast::<Array>();
        let values = $frame_values[1].clone_cast::<Array>();
        runtime_check!(
            timeline.len() == values.len(),
            concat!(
                stringify!($method),
                " should only be provided a timeline and values of the same length, but got {} vs {}"
            ),
            timeline.len(),
            values.len()
        );
        runtime_check!(
            timeline.iter().all(|d| d.is::<i32>()),
            concat!(
                stringify!($method),
                " should only be provided an i32 timeline, but got {:?}"
            ),
            timeline
        );
        runtime_check!(
            values.iter().all(|d| d.is::<$vtype>()),
            concat!(
                stringify!($method),
                " should only be provided ",
                stringify!($vtype),
                " values, but got {:?}"
            ),
            values
        );
        let mut this = $this_rc.borrow_mut();
        let mut index = 0;
        for (wait_frames, d) in timeline.iter().zip(values.iter()) {
            index += wait_frames.clone_cast::<i32>() as usize;
            this.$method(index, d.clone_cast::<$vtype>())?;
        }
        Ok(())
    }};
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
    fn frame_number(&mut self) -> Ref<'_, usize>;
    fn previous_settings_range(&mut self) -> &mut Option<Range<usize>>;
    fn active_settings_range(&mut self) -> &mut Option<Range<usize>>;

    fn end_script_run(&mut self) {
        *self.previous_settings_range() = self.active_settings_range().take()
    }

    fn reset_previous_settings_range(&mut self) {
        if let Some(mut range) = self.previous_settings_range().take() {
            let base = self.base();
            // Only clear reg settings in the present or future.
            range.start = range.start.max(*self.frame_number());
            let mut settings = self.settings_ring();
            let len = settings.len();

            // FIXME: Also clear the wave table register settings for the wave channel.
            for i in range {
                for a in base..base + 5 {
                    settings[i % len].clear_reg(a)
                }
            }
        }
    }

    fn extend_active_settings_range(&mut self, index: usize) {
        let to_frame = *self.frame_number() + index;
        let maybe_range = self.active_settings_range();
        match maybe_range.as_mut() {
            Some(range) => range.end = range.end.max(to_frame + 1),
            None => *maybe_range = Some(to_frame..to_frame + 1),
        }
    }

    fn get_reg_settings(&mut self, index: usize) -> RefMut<'_, RegSettings> {
        let i = *self.frame_number() + index;
        RefMut::map(self.settings_ring(), |s| {
            let len = s.len();
            &mut s[i % len]
        })
    }

    fn orit(&mut self, addr: u16, with: RegSetter) {
        self.reset_previous_settings_range();
        self.extend_active_settings_range(0);
        self.get_reg_settings(0).orit(addr, with)
    }
    fn orit_at_index(&mut self, index: usize, addr: u16, with: RegSetter) {
        self.reset_previous_settings_range();
        self.extend_active_settings_range(index);
        self.get_reg_settings(index).orit(addr, with)
    }

    fn set_initialize(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v == 0 || v == 1, "initialize must be 0 or 1, got {}", v);
        self.orit_at_index(index, self.base() + 4, RegSetter::new(0x80, v as u8));
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct GbSquare {
    channel: Channel,
    settings_ring: Rc<RefCell<Vec<RegSettings>>>,
    frame_number: Rc<RefCell<usize>>,
    previous_settings_range: Option<Range<usize>>,
    active_settings_range: Option<Range<usize>>,
}

impl GbSquare {
    pub fn set_sweep_time(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "sweep_time must be >= 0, got {}", v);
        runtime_check!(v < 8, "sweep_time must be < 8, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 0, RegSetter::new(0x70, v as u8));
        Ok(())
    }

    pub fn set_sweep_dir(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v == 0 || v == 1, "sweep_dir must be 0 or 1, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 0, RegSetter::new(0x08, v as u8));
        Ok(())
    }

    pub fn set_sweep_shift(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "sweep_shift must be >= 0, got {}", v);
        runtime_check!(v < 8, "sweep_shift must be < 8, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 0, RegSetter::new(0x07, v as u8));
        Ok(())
    }

    pub fn set_duty(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "duty must be >= 0, got {}", v);
        runtime_check!(v < 4, "duty must be < 4, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 1, RegSetter::new(0xC0, v as u8));
        Ok(())
    }

    pub fn set_env_start(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "env_start must be >= 0, got {}", v);
        runtime_check!(v < 16, "env_start must be < 16, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 2, RegSetter::new(0xf0, v as u8));
        Ok(())
    }

    pub fn set_env_dir(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v == 0 || v == 1, "env_dir must be 0 or 1, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 2, RegSetter::new(0x08, v as u8));
        Ok(())
    }
    pub fn set_env_period(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "env_period must be >= 0, got {}", v);
        runtime_check!(v < 8, "env_period must be < 8, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 2, RegSetter::new(0x07, v as u8));
        Ok(())
    }

    pub fn set_gb_freq(&mut self, index: usize, gb_freq: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(gb_freq >= 0, "gb_freq must be >= 0, got {}", gb_freq);
        runtime_check!(gb_freq < 2048, "gb_freq must be < 2048, got {}", gb_freq);
        self.orit_at_index(
            index,
            self.channel as u16 + 3,
            // Frequency LSB
            RegSetter::new(0xff, (gb_freq & 0xff) as u8),
        );
        self.orit_at_index(
            index,
            self.channel as u16 + 4,
            // Frequency MSB
            RegSetter::new(0x07, (gb_freq >> 8) as u8),
        );
        Ok(())
    }

    pub fn set_freq(&mut self, index: usize, freq: f64) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(freq >= 64.0, "freq must be >= 64, got {}", freq);
        self.set_gb_freq(index, GbSquare::to_gb_freq(freq))
    }

    pub fn trigger_with_length(&mut self, length: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(length >= 1, "length must be >= 1, got {}", length);
        runtime_check!(length <= 64, "length must be <= 64, got {}", length);
        self.orit(
            self.channel as u16 + 1,
            // Length load
            RegSetter::new(0x3f, 64 - length as u8),
        );
        self.orit(
            self.channel as u16 + 4,
            // Trigger
            RegSetter::new(0x80, 1)
                // Length enable
                | RegSetter::new(0x40, 1),
        );
        Ok(())
    }

    pub fn trigger(&mut self) -> Result<(), Box<EvalAltResult>> {
        self.orit(
            self.channel as u16 + 4,
            // Trigger
            RegSetter::new(0x80, 1)
                // Length enable
                | RegSetter::new(0x40, 0),
        );
        Ok(())
    }

    pub fn to_gb_freq(freq: f64) -> i32 {
        2048 - (131072.0 / freq).round() as i32
    }
}
impl ScriptChannel for GbSquare {
    fn base(&self) -> u16 {
        self.channel as u16
    }
    fn settings_ring(&mut self) -> RefMut<'_, Vec<RegSettings>> {
        self.settings_ring.borrow_mut()
    }
    fn frame_number(&mut self) -> Ref<'_, usize> {
        self.frame_number.borrow()
    }
    fn previous_settings_range(&mut self) -> &mut Option<Range<usize>> {
        &mut self.previous_settings_range
    }
    fn active_settings_range(&mut self) -> &mut Option<Range<usize>> {
        &mut self.active_settings_range
    }
}
pub type SharedGbSquare = Rc<RefCell<GbSquare>>;

#[derive(Debug, Clone)]
pub struct GbWave {
    channel: Channel,
    settings_ring: Rc<RefCell<Vec<RegSettings>>>,
    frame_number: Rc<RefCell<usize>>,
    previous_settings_range: Option<Range<usize>>,
    active_settings_range: Option<Range<usize>>,
}
impl GbWave {
    pub fn set_playing(&mut self, index: usize, v: bool) -> Result<(), Box<EvalAltResult>> {
        self.orit_at_index(index, Wave as u16 + 0, RegSetter::new(0x80, v as u8));
        Ok(())
    }

    pub fn set_volume(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "volume must be >= 0, got {}", v);
        runtime_check!(v < 4, "volume must be < 4, got {}", v);
        self.orit_at_index(index, Wave as u16 + 2, RegSetter::new(0x60, v as u8));
        Ok(())
    }

    pub fn set_table(&mut self, index: usize, hex_string: String) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(
            hex_string.len() == 32,
            "table must have a length of 32, got {}",
            hex_string.len()
        );
        runtime_check!(
            hex_string.chars().all(|c| c >= '0' && c <= '9' || c >= 'a' && c <= 'f'),
            "table must only contain characters [0-9a-f], got {}",
            hex_string
        );

        // Each hexadecimal character in the hex string is one 4 bits sample.
        for i in (0..hex_string.len()).step_by(2) {
            let byte = u8::from_str_radix(&hex_string[i..i + 2], 16).unwrap();
            self.orit_at_index(index, (0xff30 + i / 2) as u16, RegSetter::new(0xff, byte));
        }

        Ok(())
    }

    pub fn set_gb_freq(&mut self, index: usize, gb_freq: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(gb_freq >= 0, "gb_freq must be >= 0, got {}", gb_freq);
        runtime_check!(gb_freq < 2048, "gb_freq must be < 2048, got {}", gb_freq);
        self.orit_at_index(
            index,
            Wave as u16 + 3,
            // Frequency LSB
            RegSetter::new(0xff, (gb_freq & 0xff) as u8),
        );
        self.orit_at_index(
            index,
            Wave as u16 + 4,
            // Frequency MSB
            RegSetter::new(0x07, (gb_freq >> 8) as u8),
        );
        Ok(())
    }

    pub fn set_freq(&mut self, index: usize, freq: f64) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(freq >= 32.0, "freq must be >= 32, got {}", freq);
        self.set_gb_freq(index, GbWave::to_gb_freq(freq))
    }

    pub fn trigger_with_length(&mut self, length: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(length >= 1, "length must be >= 1, got {}", length);
        runtime_check!(length <= 256, "length must be <= 256, got {}", length);
        self.orit(
            Wave as u16 + 1,
            // Length load
            RegSetter::new(0xff, (256 - length) as u8),
        );
        self.orit(
            Wave as u16 + 4,
            // Trigger
            RegSetter::new(0x80, 1)
                // Length enable
                | RegSetter::new(0x40, 1),
        );
        Ok(())
    }

    pub fn trigger(&mut self) -> Result<(), Box<EvalAltResult>> {
        self.orit(
            Wave as u16 + 4,
            // Trigger
            RegSetter::new(0x80, 1)
                // Length enable
                | RegSetter::new(0x40, 0),
        );
        Ok(())
    }

    pub fn to_gb_freq(freq: f64) -> i32 {
        2048 - (65536.0 / freq).round() as i32
    }
}
impl ScriptChannel for GbWave {
    fn base(&self) -> u16 {
        self.channel as u16
    }
    fn settings_ring(&mut self) -> RefMut<'_, Vec<RegSettings>> {
        self.settings_ring.borrow_mut()
    }
    fn frame_number(&mut self) -> Ref<'_, usize> {
        self.frame_number.borrow()
    }
    fn previous_settings_range(&mut self) -> &mut Option<Range<usize>> {
        &mut self.previous_settings_range
    }
    fn active_settings_range(&mut self) -> &mut Option<Range<usize>> {
        &mut self.active_settings_range
    }
}
pub type SharedGbWave = Rc<RefCell<GbWave>>;

#[derive(Debug, Clone)]
pub struct GbNoise {
    channel: Channel,
    settings_ring: Rc<RefCell<Vec<RegSettings>>>,
    frame_number: Rc<RefCell<usize>>,
    previous_settings_range: Option<Range<usize>>,
    active_settings_range: Option<Range<usize>>,
}
impl GbNoise {
    pub fn set_env_start(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v >= 0, "env_start must be >= 0, got {}", v);
        runtime_check!(v < 16, "env_start must be < 16, got {}", v);
        self.orit_at_index(index, Noise as u16 + 2, RegSetter::new(0xf0, v as u8));
        Ok(())
    }

    pub fn set_env_dir(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v == 0 || v == 1, "env_dir must be 0 or 1, got {}", v);
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
        runtime_check!(v < 14, "clock_shift must be < 14, got {}", v);
        self.orit_at_index(index, Noise as u16 + 3, RegSetter::new(0xf0, v as u8));
        Ok(())
    }

    pub fn set_counter_width(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
        runtime_check!(v == 0 || v == 1, "counter_width must be 0 or 1, got {}", v);
        self.orit_at_index(index, Noise as u16 + 3, RegSetter::new(0x08, v as u8));
        Ok(())
    }

    pub fn set_clock_divisor(&mut self, index: usize, v: i32) -> Result<(), Box<EvalAltResult>> {
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
            RegSetter::new(0x3f, 64 - length as u8),
        );
        self.orit(
            Noise as u16 + 4,
            // Trigger
            RegSetter::new(0x80, 1)
                // Length enable
                | RegSetter::new(0x40, 1),
        );
        Ok(())
    }

    pub fn trigger(&mut self) -> Result<(), Box<EvalAltResult>> {
        self.orit(
            Noise as u16 + 4,
            // Trigger
            RegSetter::new(0x80, 1),
        );
        Ok(())
    }
}
impl ScriptChannel for GbNoise {
    fn base(&self) -> u16 {
        self.channel as u16
    }
    fn settings_ring(&mut self) -> RefMut<'_, Vec<RegSettings>> {
        self.settings_ring.borrow_mut()
    }
    fn frame_number(&mut self) -> Ref<'_, usize> {
        self.frame_number.borrow()
    }
    fn previous_settings_range(&mut self) -> &mut Option<Range<usize>> {
        &mut self.previous_settings_range
    }
    fn active_settings_range(&mut self) -> &mut Option<Range<usize>> {
        &mut self.active_settings_range
    }
}
pub type SharedGbNoise = Rc<RefCell<GbNoise>>;

#[derive(Debug, Clone)]
pub struct GbBindings {
    settings_ring: Rc<RefCell<Vec<RegSettings>>>,
    frame_number: Rc<RefCell<usize>>,
    square1: SharedGbSquare,
    square2: SharedGbSquare,
    wave: SharedGbWave,
    noise: SharedGbNoise,
}
pub type SharedGbBindings = Rc<RefCell<GbBindings>>;

impl GbBindings {
    fn set_frame_number(&mut self, v: usize) {
        *self.frame_number.borrow_mut() = v;
    }
    fn end_script_run(&mut self) {
        self.square1.borrow_mut().end_script_run();
        self.square2.borrow_mut().end_script_run();
        self.wave.borrow_mut().end_script_run();
        self.noise.borrow_mut().end_script_run();
    }
}

#[export_module]
pub mod gb_api {

    pub const SWE_INC: i32 = 0;
    pub const SWE_DEC: i32 = 1;
    pub const ENV_DEC: i32 = 0;
    pub const ENV_INC: i32 = 1;
    pub const DUT_1_8: i32 = 0;
    pub const DUT_1_4: i32 = 1;
    pub const DUT_2_4: i32 = 2;
    pub const DUT_3_4: i32 = 3;
    pub const VOL_0: i32 = 0;
    pub const VOL_100: i32 = 1;
    pub const VOL_50: i32 = 2;
    pub const VOL_25: i32 = 3;
    pub const WID_15: i32 = 0;
    pub const WID_7: i32 = 1;
    pub const DIV_8: i32 = 0;
    pub const DIV_16: i32 = 1;
    pub const DIV_32: i32 = 2;
    pub const DIV_48: i32 = 3;
    pub const DIV_64: i32 = 4;
    pub const DIV_80: i32 = 5;
    pub const DIV_96: i32 = 6;
    pub const DIV_112: i32 = 7;

    /// Just a clearer wrapper for [[t1, t2, ...], [v1, v2, ...]], which is what multi setters expect.
    #[rhai_fn(global, name = "frames", return_raw)]
    pub fn frames(timeline: Array, values: Array) -> Result<Array, Box<EvalAltResult>> {
        Ok(vec![timeline.into(), values.into()])
    }

    #[rhai_fn(global)]
    pub fn to_square_gb_freq(freq: f64) -> i32 {
        GbSquare::to_gb_freq(freq)
    }

    #[rhai_fn(global)]
    pub fn to_wave_gb_freq(freq: f64) -> i32 {
        GbWave::to_gb_freq(freq)
    }

    #[rhai_fn(get = "square1", pure)]
    pub fn get_square1(this_rc: &mut SharedGbBindings) -> SharedGbSquare {
        this_rc.borrow().square1.clone()
    }
    #[rhai_fn(get = "square2", pure)]
    pub fn get_square2(this_rc: &mut SharedGbBindings) -> SharedGbSquare {
        this_rc.borrow().square2.clone()
    }
    #[rhai_fn(get = "wave", pure)]
    pub fn get_wave(this_rc: &mut SharedGbBindings) -> SharedGbWave {
        this_rc.borrow().wave.clone()
    }
    #[rhai_fn(get = "noise", pure)]
    pub fn get_noise(this_rc: &mut SharedGbBindings) -> SharedGbNoise {
        this_rc.borrow().noise.clone()
    }

    #[rhai_fn(global, return_raw)]
    pub fn wait_frames(gb: &mut SharedGbBindings, frames: i32) -> Result<(), Box<EvalAltResult>> {
        let this = gb.borrow_mut();
        let len = this.settings_ring.borrow().len();
        // FIXME: Check that this isn't going back past the current frame
        // runtime_check!(frames >= 0, "frames must be >= 0, got {}", frames);
        runtime_check!(frames < len as i32, "frames must be < {}, got {}", len, frames);
        let mut frame_number = this.frame_number.borrow_mut();
        *frame_number = *frame_number + frames as usize;
        Ok(())
    }

    #[rhai_fn(set = "sweep_time", pure, return_raw)]
    pub fn set_sweep_time(this_rc: &mut SharedGbSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_sweep_time)
    }
    #[rhai_fn(set = "sweep_time", pure, return_raw)]
    pub fn set_multi_sweep_time(this_rc: &mut SharedGbSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_sweep_time)
    }

    #[rhai_fn(set = "sweep_dir", pure, return_raw)]
    pub fn set_sweep_dir(this_rc: &mut SharedGbSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_sweep_dir)
    }
    #[rhai_fn(set = "sweep_dir", pure, return_raw)]
    pub fn set_multi_sweep_dir(this_rc: &mut SharedGbSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_sweep_dir)
    }

    #[rhai_fn(set = "sweep_shift", pure, return_raw)]
    pub fn set_sweep_shift(this_rc: &mut SharedGbSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_sweep_shift)
    }
    #[rhai_fn(set = "sweep_shift", pure, return_raw)]
    pub fn set_multi_sweep_shift(this_rc: &mut SharedGbSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_sweep_shift)
    }

    #[rhai_fn(set = "duty", pure, return_raw)]
    pub fn set_duty(this_rc: &mut SharedGbSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_duty)
    }
    #[rhai_fn(set = "duty", pure, return_raw)]
    pub fn set_multi_duty(this_rc: &mut SharedGbSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_duty)
    }

    #[rhai_fn(set = "env_start", pure, return_raw)]
    pub fn set_square_env_start(this_rc: &mut SharedGbSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_env_start)
    }
    #[rhai_fn(set = "env_start", pure, return_raw)]
    pub fn set_multi_square_env_start(
        this_rc: &mut SharedGbSquare,
        values: Vec<Dynamic>,
    ) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_env_start)
    }

    #[rhai_fn(set = "env_dir", pure, return_raw)]
    pub fn set_square_env_dir(this_rc: &mut SharedGbSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_env_dir)
    }
    #[rhai_fn(set = "env_dir", pure, return_raw)]
    pub fn set_multi_square_env_dir(this_rc: &mut SharedGbSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_env_dir)
    }

    #[rhai_fn(set = "env_period", pure, return_raw)]
    pub fn set_square_env_period(this_rc: &mut SharedGbSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_env_period)
    }
    #[rhai_fn(set = "env_period", pure, return_raw)]
    pub fn set_multi_square_env_period(this_rc: &mut SharedGbSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_env_period)
    }

    #[rhai_fn(set = "gb_freq", pure, return_raw)]
    pub fn set_square_gb_freq(this_rc: &mut SharedGbSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_gb_freq)
    }
    #[rhai_fn(set = "gb_freq", pure, return_raw)]
    pub fn set_multi_square_gb_freq(this_rc: &mut SharedGbSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_gb_freq)
    }

    #[rhai_fn(set = "freq", pure, return_raw)]
    pub fn set_square_freq(this_rc: &mut SharedGbSquare, v: f64) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_freq)
    }
    #[rhai_fn(set = "freq", pure, return_raw)]
    pub fn set_multi_square_freq(this_rc: &mut SharedGbSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, f64, set_freq)
    }

    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_square_initialize(this_rc: &mut SharedGbSquare, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_initialize)
    }
    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_square_initialize_bool(this_rc: &mut SharedGbSquare, b: bool) -> Result<(), Box<EvalAltResult>> {
        let v = b as i32;
        set!(this_rc, v, set_initialize)
    }
    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_multi_square_initialize(this_rc: &mut SharedGbSquare, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_initialize)
    }

    #[rhai_fn(global, name = "trigger_with_length", return_raw)]
    pub fn square_trigger_with_length(this_rc: &mut SharedGbSquare, length: i32) -> Result<(), Box<EvalAltResult>> {
        this_rc.borrow_mut().trigger_with_length(length)
    }

    #[rhai_fn(global, name = "trigger", return_raw)]
    pub fn square_trigger(this_rc: &mut SharedGbSquare) -> Result<(), Box<EvalAltResult>> {
        this_rc.borrow_mut().trigger()
    }

    #[rhai_fn(set = "playing", pure, return_raw)]
    pub fn set_playing(this_rc: &mut SharedGbWave, v: bool) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_playing)
    }
    #[rhai_fn(set = "playing", pure, return_raw)]
    pub fn set_multi_playing(this_rc: &mut SharedGbWave, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, bool, set_playing)
    }

    #[rhai_fn(set = "volume", pure, return_raw)]
    pub fn set_volume(this_rc: &mut SharedGbWave, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_volume)
    }
    #[rhai_fn(set = "volume", pure, return_raw)]
    pub fn set_multi_volume(this_rc: &mut SharedGbWave, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_volume)
    }

    #[rhai_fn(set = "table", pure, return_raw)]
    pub fn set_table(this_rc: &mut SharedGbWave, v: String) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_table)
    }
    #[rhai_fn(set = "table", pure, return_raw)]
    pub fn set_multi_table(this_rc: &mut SharedGbWave, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, String, set_table)
    }

    #[rhai_fn(set = "gb_freq", pure, return_raw)]
    pub fn set_wave_gb_freq(this_rc: &mut SharedGbWave, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_gb_freq)
    }
    #[rhai_fn(set = "gb_freq", pure, return_raw)]
    pub fn set_multi_wave_gb_freq(this_rc: &mut SharedGbWave, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_gb_freq)
    }

    #[rhai_fn(set = "freq", pure, return_raw)]
    pub fn set_wave_freq(this_rc: &mut SharedGbWave, v: f64) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_freq)
    }
    #[rhai_fn(set = "freq", pure, return_raw)]
    pub fn set_multi_wave_freq(this_rc: &mut SharedGbWave, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, f64, set_freq)
    }

    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_wave_initialize(this_rc: &mut SharedGbWave, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_initialize)
    }
    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_wave_initialize_bool(this_rc: &mut SharedGbWave, b: bool) -> Result<(), Box<EvalAltResult>> {
        let v = b as i32;
        set!(this_rc, v, set_initialize)
    }
    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_multi_wave_initialize(this_rc: &mut SharedGbWave, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_initialize)
    }

    #[rhai_fn(global, name = "trigger_with_length", return_raw)]
    pub fn wave_trigger_with_length(this_rc: &mut SharedGbWave, length: i32) -> Result<(), Box<EvalAltResult>> {
        this_rc.borrow_mut().trigger_with_length(length)
    }
    #[rhai_fn(global, name = "trigger", return_raw)]
    pub fn wave_trigger(this_rc: &mut SharedGbWave) -> Result<(), Box<EvalAltResult>> {
        this_rc.borrow_mut().trigger()
    }

    #[rhai_fn(set = "env_start", pure, return_raw)]
    pub fn set_noise_env_start(this_rc: &mut SharedGbNoise, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_env_start)
    }
    #[rhai_fn(set = "env_start", pure, return_raw)]
    pub fn set_multi_noise_env_start(this_rc: &mut SharedGbNoise, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_env_start)
    }

    #[rhai_fn(set = "env_dir", pure, return_raw)]
    pub fn set_noise_env_dir(this_rc: &mut SharedGbNoise, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_env_dir)
    }
    #[rhai_fn(set = "env_dir", pure, return_raw)]
    pub fn set_multi_noise_env_dir(this_rc: &mut SharedGbNoise, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_env_dir)
    }

    #[rhai_fn(set = "env_period", pure, return_raw)]
    pub fn set_noise_env_period(this_rc: &mut SharedGbNoise, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_env_period)
    }
    #[rhai_fn(set = "env_period", pure, return_raw)]
    pub fn set_multi_noise_env_period(this_rc: &mut SharedGbNoise, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_env_period)
    }

    #[rhai_fn(set = "clock_shift", pure, return_raw)]
    pub fn set_clock_shift(this_rc: &mut SharedGbNoise, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_clock_shift)
    }
    #[rhai_fn(set = "clock_shift", pure, return_raw)]
    pub fn set_multi_clock_shift(this_rc: &mut SharedGbNoise, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_clock_shift)
    }

    #[rhai_fn(set = "counter_width", pure, return_raw)]
    pub fn set_counter_width(this_rc: &mut SharedGbNoise, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_counter_width)
    }
    #[rhai_fn(set = "counter_width", pure, return_raw)]
    pub fn set_multi_counter_width(this_rc: &mut SharedGbNoise, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_counter_width)
    }

    #[rhai_fn(set = "clock_divisor", pure, return_raw)]
    pub fn set_clock_divisor(this_rc: &mut SharedGbNoise, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_clock_divisor)
    }
    #[rhai_fn(set = "clock_divisor", pure, return_raw)]
    pub fn set_multi_clock_divisor(this_rc: &mut SharedGbNoise, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_clock_divisor)
    }

    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_noise_initialize(this_rc: &mut SharedGbNoise, v: i32) -> Result<(), Box<EvalAltResult>> {
        set!(this_rc, v, set_initialize)
    }
    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_noise_initialize_bool(this_rc: &mut SharedGbNoise, b: bool) -> Result<(), Box<EvalAltResult>> {
        let v = b as i32;
        set!(this_rc, v, set_initialize)
    }
    #[rhai_fn(set = "initialize", pure, return_raw)]
    pub fn set_multi_noise_initialize(this_rc: &mut SharedGbNoise, values: Array) -> Result<(), Box<EvalAltResult>> {
        set_multi!(this_rc, values, i32, set_initialize)
    }

    #[rhai_fn(global, name = "trigger_with_length", return_raw)]
    pub fn noise_trigger_with_length(this_rc: &mut SharedGbNoise, length: i32) -> Result<(), Box<EvalAltResult>> {
        this_rc.borrow_mut().trigger_with_length(length)
    }
    #[rhai_fn(global, name = "trigger", return_raw)]
    pub fn noise_trigger(this_rc: &mut SharedGbNoise) -> Result<(), Box<EvalAltResult>> {
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
        RegSetter {
            mask: mask,
            value: shifted,
        }
    }
    const EMPTY: RegSetter = RegSetter { mask: 0, value: 0 };
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
        RegSettings {
            registers: [RegSetter::EMPTY; 48],
        }
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
        self.registers
            .iter()
            .enumerate()
            .filter_map(|(a, &r)| {
                if r.mask != 0 {
                    Some((a as u16 + 0xff10, r))
                } else {
                    None
                }
            })
            .for_each(|(a, r)| f(a, r))
    }

    pub fn clear_reg(&mut self, addr: u16) {
        let index = addr as usize - 0xff10;
        self.registers[index] = RegSetter::EMPTY;
    }

    pub fn clear_all(&mut self) {
        self.registers = [RegSetter::EMPTY; 48];
    }
}

#[derive(Clone)]
struct InstrumentState {
    press_function: Option<FnPtr>,
    release_function: Option<FnPtr>,
    frame_function: Option<FnPtr>,
    pressed_note_and_frame: Option<(u32, usize)>,
}

pub struct SynthScript {
    script_engine: Engine,
    script_ast: AST,
    script_context: SharedGbBindings,
    instrument_ids: Rc<RefCell<Vec<String>>>,
    instrument_states: Rc<RefCell<Vec<InstrumentState>>>,
}

impl SynthScript {
    const DEFAULT_INSTRUMENTS: &'static str = include_str!("../res/default-instruments.rhai");

    pub fn new(settings_ring: Rc<RefCell<Vec<RegSettings>>>) -> SynthScript {
        let instrument_ids: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec!["".to_string(); NUM_INSTRUMENTS]));
        let instrument_states = Rc::new(RefCell::new(vec![
            InstrumentState {
                press_function: None,
                release_function: None,
                frame_function: None,
                pressed_note_and_frame: None
            };
            NUM_INSTRUMENTS
        ]));
        let instrument_ids_clone = instrument_ids.clone();
        let instrument_states_clone = instrument_states.clone();

        let mut engine = Engine::new();

        engine.set_max_expr_depths(1024, 1024);
        engine
            .register_type::<SharedGbBindings>()
            .register_type::<SharedGbSquare>();
        engine.register_result_fn("set_instrument",
            move |i: i32, instrument: Dynamic| {
                runtime_check!(i >= 1 && i <= 64,
                    "set_instrument: index must be 1 <= i <= 64, got {}",
                    i);
                runtime_check!(instrument.type_id() == TypeId::of::<Map>(),
                    "set_instrument: The instrument must be an object map, got {}",
                    instrument.type_name());

                let mut map = instrument.try_cast::<Map>().unwrap();

                instrument_ids_clone.borrow_mut()[(i - 1) as usize] =
                    if let Some(id) = map.remove("id").map(|o| o.into_string().unwrap()) {
                        id
                    } else {
                        i.to_string()
                    };

                let state = &mut instrument_states_clone.borrow_mut()[(i - 1) as usize];

                state.release_function = match map.remove("release") {
                    None =>
                        None,

                    Some(f_dyn) => {
                        runtime_check!(f_dyn.type_id() == TypeId::of::<FnPtr>(),
                            "set_instrument: The \"release\" property must be a function pointer or an anonymous function, got a {}",
                            f_dyn.type_name());

                        let f = f_dyn.try_cast::<FnPtr>().unwrap();
                        Some(f)
                    }
                };

                state.frame_function = match map.remove("frame") {
                    None =>
                        None,

                    Some(f_dyn) => {
                        runtime_check!(f_dyn.type_id() == TypeId::of::<FnPtr>(),
                            "set_instrument: The \"frame\" property must be a function pointer or an anonymous function, got a {}",
                            f_dyn.type_name());

                        let f = f_dyn.try_cast::<FnPtr>().unwrap();
                        Some(f)
                    }
                };

                match map.remove("press") {
                    None =>
                        Err(format!("set_instrument: The instrument must have a \"press\" property").into()),

                    Some(f_dyn) => {
                        runtime_check!(f_dyn.type_id() == TypeId::of::<FnPtr>(),
                            "set_instrument: The \"press\" property must be a function pointer or an anonymous function, got a {}",
                            f_dyn.type_name());

                        let f = f_dyn.try_cast::<FnPtr>().unwrap();
                        state.press_function = Some(f);
                        Ok(())
                    },
                }
            });
        engine.register_static_module("gb", exported_module!(gb_api).into());

        let frame_number = Rc::new(RefCell::new(0));
        let square1 = Rc::new(RefCell::new(GbSquare {
            channel: Square1,
            settings_ring: settings_ring.clone(),
            frame_number: frame_number.clone(),
            previous_settings_range: None,
            active_settings_range: None,
        }));
        let square2 = Rc::new(RefCell::new(GbSquare {
            channel: Square2,
            settings_ring: settings_ring.clone(),
            frame_number: frame_number.clone(),
            previous_settings_range: None,
            active_settings_range: None,
        }));
        let wave = Rc::new(RefCell::new(GbWave {
            channel: Wave,
            settings_ring: settings_ring.clone(),
            frame_number: frame_number.clone(),
            previous_settings_range: None,
            active_settings_range: None,
        }));
        let noise = Rc::new(RefCell::new(GbNoise {
            channel: Noise,
            settings_ring: settings_ring.clone(),
            frame_number: frame_number.clone(),
            previous_settings_range: None,
            active_settings_range: None,
        }));
        let gb = Rc::new(RefCell::new(GbBindings {
            settings_ring: settings_ring.clone(),
            frame_number: frame_number,
            square1: square1,
            square2: square2,
            wave: wave,
            noise: noise,
        }));

        SynthScript {
            script_engine: engine,
            script_ast: Default::default(),
            script_context: gb,
            instrument_ids: instrument_ids,
            instrument_states: instrument_states,
        }
    }

    pub fn instrument_ids(&self) -> Vec<String> {
        self.instrument_ids.borrow().clone()
    }

    fn load_default_instruments(&mut self, frame_number: usize) {
        self.script_engine
            .compile(SynthScript::DEFAULT_INSTRUMENTS)
            .map_err(|e| Box::new(e) as Box<dyn Error>)
            .and_then(|ast| {
                self.set_instruments_ast(ast, frame_number)
                    .map_err(|e| e as Box<dyn Error>)
            })
            .expect("Error loading default instruments.");
    }

    #[cfg(target_arch = "wasm32")]
    fn deserialize_instruments(&self, base64: String) -> Result<AST, Box<dyn Error>> {
        let decoded = crate::utils::decode_string(&base64)?;
        let ast = self.script_engine.compile(&decoded)?;
        Ok(ast)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load(&mut self, maybe_base64: Option<String>) {
        if let Some(base64) = maybe_base64 {
            let maybe_ast = self
                .deserialize_instruments(base64)
                .and_then(|ast| self.set_instruments_ast(ast).map_err(|e| e as Box<dyn Error>));

            match maybe_ast {
                Ok(ast) => {
                    log!("Loaded the project instruments from the URL.");
                }
                Err(e) => {
                    elog!(
                        "Couldn't load the project instruments from the URL, using default instruments.\n\tError: {:?}",
                        e
                    );
                    self.load_default_instruments();
                }
            }
        } else {
            log!("No instruments provided in the URL, using default instruments.");
            self.load_default_instruments();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(&mut self, project_instruments_path: &std::path::Path, frame_number: usize) {
        if project_instruments_path.exists() {
            let maybe_ast = self
                .script_engine
                .compile_file(project_instruments_path.to_path_buf())
                .and_then(|ast| self.set_instruments_ast(ast, frame_number));

            match maybe_ast {
                Ok(_) => {
                    log!("Loaded project instruments from file {:?}", project_instruments_path);
                }
                Err(e) => {
                    elog!(
                        "Couldn't load project instruments from file {:?}, using default instruments.\n\tError: {:?}",
                        project_instruments_path,
                        e
                    );
                    self.load_default_instruments(frame_number);
                }
            }
        } else {
            log!(
                "Project instruments file {:?} doesn't exist, using default instruments.",
                project_instruments_path
            );
            self.load_default_instruments(frame_number);
        }
    }

    pub fn press_instrument_note(&mut self, frame_number: usize, instrument: u32, note: u32) -> () {
        {
            let mut gb = self.script_context.borrow_mut();
            // The script themselves are modifying this state, so reset it.
            gb.set_frame_number(frame_number);
            gb.end_script_run();
        }

        let state = &mut self.instrument_states.borrow_mut()[instrument as usize];
        if let Some(f) = &state.press_function {
            state.pressed_note_and_frame = Some((note, frame_number));
            let result: Result<(), _> = f.call(
                &self.script_engine,
                &self.script_ast,
                (note as i32, Self::note_to_freq(note)),
            );
            if let Err(e) = result {
                elog!("{}", e)
            }
        }
    }

    pub fn release_instrument(&mut self, frame_number: usize, instrument: u32) -> () {
        // The script themselves are modifying this state, so reset it.
        self.script_context.borrow_mut().set_frame_number(frame_number);

        let state = &mut self.instrument_states.borrow_mut()[instrument as usize];
        if let (Some(f), Some((note, _))) = (&state.release_function, state.pressed_note_and_frame.take()) {
            let result: Result<(), _> = f.call(
                &self.script_engine,
                &self.script_ast,
                (note as i32, Self::note_to_freq(note)),
            );
            if let Err(e) = result {
                elog!("{}", e)
            }
        }
    }

    pub fn advance_frame(&mut self, frame_number: usize) {
        for state in &*self.instrument_states.borrow_mut() {
            // Only run the frame function on instruments currently pressed.
            if let (Some(f), Some((note, pressed_frame))) = (&state.frame_function, state.pressed_note_and_frame) {
                // The script themselves are modifying this state, so reset it.
                self.script_context.borrow_mut().set_frame_number(frame_number);

                let result: Result<(), _> = f.call(
                    &self.script_engine,
                    &self.script_ast,
                    (
                        note as i32,
                        Self::note_to_freq(note),
                        (frame_number - pressed_frame) as i32,
                    ),
                );
                if let Err(e) = result {
                    elog!("{}", e)
                }
            }
        }
    }

    fn set_instruments_ast(
        &mut self,
        ast: AST,
        frame_number: usize,
    ) -> Result<(), std::boxed::Box<rhai::EvalAltResult>> {
        self.script_ast = ast;

        // The script might also contain sound settings directly in the its root.
        {
            let mut gb = self.script_context.borrow_mut();
            gb.set_frame_number(frame_number);
            gb.end_script_run();
            // FIXME: Also reset the gb states somewhere like gbsplay does
        }

        let mut scope = Scope::new();
        scope.push("gb", self.script_context.clone());

        self.script_engine.run_ast_with_scope(&mut scope, &self.script_ast)
    }

    fn note_to_freq(note: u32) -> f64 {
        let a = 440.0; // Frequency of A
        let key_freq = (a / 32.0) * 2.0_f64.powf((note as f64 - 9.0) / 12.0);
        key_freq
    }
}
