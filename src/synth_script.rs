// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::sound_engine::NUM_INSTRUMENT_COLS;
use crate::sound_engine::NUM_INSTRUMENTS;
use crate::synth_script::Channel::*;
use crate::synth_script::test::WasmExecEnv;
use crate::synth_script::test::WasmRuntime;
use crate::utils::NOTE_FREQUENCIES;

use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::ffi::CStr;
use std::fs::File;
use std::io::Write;
use std::ops::BitOr;
use std::ops::BitOrAssign;
use std::ops::Range;
use std::rc::Rc;

pub mod test;

// Only works if the function's return type is Result<_, Box<EvalAltResult>>
macro_rules! runtime_check {
    ($cond : expr, $($err : tt) +) => {
        if !$cond {
            elog!($( $err )*);
        };
    }
}


macro_rules! setter_wrapper {
    ($gb: ident, $method: ident ( $( $arg:ident ),* )) => {{
        let _gb = $gb.clone();
        move |chan, $( $arg, )*| {
            // FIXME: Check bounds
            let this = &_gb.channels[chan as usize];
            this.$method(0 $(, $arg )*).unwrap();
        }
    }};
}
macro_rules! setter_gb_wrapper {
    ($gb: ident, $method: ident ( $( $arg:ident ),* )) => {{
        let _gb = $gb.clone();
        move |$( $arg, )*| {
            _gb.$method($( $arg, )*).unwrap();
        }
    }};
}
macro_rules! setter_table_wrapper {
    ($gb: ident, $method: ident ( $arg:ident )) => {{
        let _gb = $gb.clone();
        move |chan, $arg: &[i32]| {
            // FIXME: Check bounds
            let this = &_gb.channels[chan as usize];
            this.$method(0 , $arg).unwrap();
        }
    }};
}

macro_rules! trigger_wrapper {
    ($gb: ident, $method: ident ( $( $arg:ident ),* )) => {{
        let _gb = $gb.clone();
        move |chan, $( $arg, )*| {
            let this = &_gb.channels[chan as usize];
            this.$method($( $arg, )*).unwrap();
        }
    }};
}

macro_rules! setter_frames_wrapper {
    ($gb: ident, $method: ident ( $values:ident )) => {{
        let _gb = $gb.clone();
        move |chan, timeline: &[i32], $values: &[i32]| {
            runtime_check!(
                timeline.len() == $values.len(),
                concat!(
                    stringify!($method),
                    " should only be provided a timeline and values of the same length, but got {} vs {}"
                ),
                timeline.len(),
                $values.len()
            );
            let mut index = 0;
            let this = &_gb.channels[chan as usize];
            for (wait_frames, d) in timeline.iter().zip($values.iter()) {
                index += *wait_frames as usize;
                this.$method(index, *d).unwrap();
            }
        }
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
    fn settings_ring(&self) -> RefMut<'_, Vec<RegSettings>>;
    fn frame_number(&self) -> Ref<'_, usize>;
    fn resettable_settings_range(&self) -> RefMut<'_, Option<Range<usize>>>;
    fn pending_settings_range(&self) -> RefMut<'_, Option<Range<usize>>>;
    fn trigger(&self) -> Result<(), String>;

    fn register_addresses(&self) -> std::iter::Chain<Range<u16>, Range<u16>> {
        let base = self.base();
        (base..base + 5).into_iter().chain((0..0).into_iter())
    }

    fn mark_pending_settings_as_resettable(&self) {
        *self.resettable_settings_range() = self.pending_settings_range().take()
    }

    fn unmark_pending_settings_as_resettable(&self) {
        if let Some(range) = self.resettable_settings_range().take() {
            // Any setting marked as active should have reset the pending channel settings first.
            assert!(self.pending_settings_range().is_none());
            *self.pending_settings_range() = Some(range);
            // *self.resettable_settings_range() = self.pending_settings_range().take()
        }
    }

    fn reset_resettable_settings_range(&self) {
        if let Some(mut frame_range) = self.resettable_settings_range().take() {
            let reg_addrs = self.register_addresses();
            // Only clear reg settings in the present or future.
            frame_range.start = frame_range.start.max(*self.frame_number());
            let mut settings = self.settings_ring();
            let len = settings.len();

            for f in frame_range {
                for a in reg_addrs.clone() {
                    settings[f % len].clear_reg(a)
                }
            }
        }
    }

    fn extend_pending_settings_range(&self, index: usize) {
        let to_frame = *self.frame_number() + index;
        let mut maybe_range = self.pending_settings_range();
        match maybe_range.as_mut() {
            Some(range) => range.end = range.end.max(to_frame + 1),
            None => *maybe_range = Some(to_frame..to_frame + 1),
        }
    }

    fn get_reg_settings(&self, index: usize) -> RefMut<'_, RegSettings> {
        let i = *self.frame_number() + index;
        RefMut::map(self.settings_ring(), |s| {
            let len = s.len();
            &mut s[i % len]
        })
    }

    fn orit(&self, addr: u16, with: RegSetter) {
        self.reset_resettable_settings_range();
        self.extend_pending_settings_range(0);
        self.get_reg_settings(0).orit(addr, with)
    }
    fn orit_at_index(&self, index: usize, addr: u16, with: RegSetter) {
        self.reset_resettable_settings_range();
        self.extend_pending_settings_range(index);
        self.get_reg_settings(index).orit(addr, with)
    }

    fn set_initialize(&self, index: usize, v: i32) -> Result<(), String> {
        runtime_check!(v == 0 || v == 1, "initialize must be 0 or 1, got {}", v);
        self.orit_at_index(index, self.base() + 4, RegSetter::new(0x80, v as u8));
        Ok(())
    }
}

#[derive(Clone)]
pub struct GbSquare {
    channel: Channel,
    settings_ring: Rc<RefCell<Vec<RegSettings>>>,
    frame_number: Rc<RefCell<usize>>,
    resettable_settings_range: RefCell<Option<Range<usize>>>,
    pending_settings_range: RefCell<Option<Range<usize>>>,
}

impl GbSquare {
    pub fn set_square_sweep_time(&self, index: usize, v: i32) -> Result<(), String> {
        runtime_check!(v >= 0, "sweep_time must be >= 0, got {}", v);
        runtime_check!(v < 8, "sweep_time must be < 8, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 0, RegSetter::new(0x70, v as u8));
        Ok(())
    }

    pub fn set_square_sweep_dir(&self, index: usize, v: i32) -> Result<(), String> {
        runtime_check!(v == 0 || v == 1, "sweep_dir must be 0 or 1, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 0, RegSetter::new(0x08, v as u8));
        Ok(())
    }

    pub fn set_square_sweep_shift(&self, index: usize, v: i32) -> Result<(), String> {
        runtime_check!(v >= 0, "sweep_shift must be >= 0, got {}", v);
        runtime_check!(v < 8, "sweep_shift must be < 8, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 0, RegSetter::new(0x07, v as u8));
        Ok(())
    }

    pub fn set_square_duty(&self, index: usize, v: i32) -> Result<(), String> {
        runtime_check!(v >= 0, "duty must be >= 0, got {}", v);
        runtime_check!(v < 4, "duty must be < 4, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 1, RegSetter::new(0xC0, v as u8));
        Ok(())
    }

    pub fn set_square_env_start(&self, index: usize, v: i32) -> Result<(), String> {
        runtime_check!(v >= 0, "env_start must be >= 0, got {}", v);
        runtime_check!(v < 16, "env_start must be < 16, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 2, RegSetter::new(0xf0, v as u8));
        Ok(())
    }

    pub fn set_square_env_dir(&self, index: usize, v: i32) -> Result<(), String> {
        runtime_check!(v == 0 || v == 1, "env_dir must be 0 or 1, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 2, RegSetter::new(0x08, v as u8));
        Ok(())
    }
    pub fn set_square_env_period(&self, index: usize, v: i32) -> Result<(), String> {
        runtime_check!(v >= 0, "env_period must be >= 0, got {}", v);
        runtime_check!(v < 8, "env_period must be < 8, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 2, RegSetter::new(0x07, v as u8));
        Ok(())
    }

    pub fn set_square_gb_freq(&self, index: usize, gb_freq: i32) -> Result<(), String> {
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

    pub fn set_square_freq(&self, index: usize, freq: i32) -> Result<(), String> {
        runtime_check!(freq >= 65536, "freq must be >= 65536, got {}", freq);
        runtime_check!(freq <= (131072 * 1024), "freq must be <= 134217728, got {}", freq);
        self.set_square_gb_freq(index, GbSquare::to_square_gb_freq(freq))
    }

    pub fn trigger_square_with_length(&self, length: i32) -> Result<(), String> {
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

    pub fn to_square_gb_freq(freq: i32) -> i32 {
        2048 - ((131072 * 1024) / freq) as i32
    }

    pub fn set_wave_playing(&self, index: usize, v: i32) -> Result<(), String> {
        self.orit_at_index(index, self.channel as u16 + 0, RegSetter::new(0x80, v as u8));
        Ok(())
    }

    pub fn set_wave_volume(&self, index: usize, v: i32) -> Result<(), String> {
        runtime_check!(v >= 0, "volume must be >= 0, got {}", v);
        runtime_check!(v < 4, "volume must be < 4, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 2, RegSetter::new(0x60, v as u8));
        Ok(())
    }

    pub fn set_wave_table(&self, index: usize, table: &[i32]) -> Result<(), String> {
        runtime_check!(
            table.len() == 4,
            "wave table must have a length of 4, got {}",
            table.len()
        );

        for i in 0..table.len() {
            let v = table[i];
            self.orit_at_index(index, (0xff30 + i * 4) as u16, RegSetter::new(0xff, (v >> 24) as u8));
            self.orit_at_index(index, (0xff30 + i * 4 + 1) as u16, RegSetter::new(0xff, (v >> 16) as u8));
            self.orit_at_index(index, (0xff30 + i * 4 + 2) as u16, RegSetter::new(0xff, (v >> 8) as u8));
            self.orit_at_index(index, (0xff30 + i * 4 + 3) as u16, RegSetter::new(0xff, v as u8));
        }

        Ok(())
    }

    pub fn set_wave_gb_freq(&self, index: usize, gb_freq: i32) -> Result<(), String> {
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

    pub fn set_wave_freq(&self, index: usize, freq: i32) -> Result<(), String> {
        runtime_check!(freq >= 32768, "freq must be >= 32768, got {}", freq);
        runtime_check!(freq <= (65536 * 1024), "freq must be <= 67108864, got {}", freq);
        self.set_wave_gb_freq(index, GbSquare::to_wave_gb_freq(freq))
    }

    pub fn trigger_wave_with_length(&self, length: i32) -> Result<(), String> {
        runtime_check!(length >= 1, "length must be >= 1, got {}", length);
        runtime_check!(length <= 256, "length must be <= 256, got {}", length);
        self.orit(
            self.channel as u16 + 1,
            // Length load
            RegSetter::new(0xff, (256 - length) as u8),
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

    pub fn to_wave_gb_freq(freq: i32) -> i32 {
        2048 - ((65536 * 1024) / freq) as i32
    }

    pub fn set_noise_env_start(&self, index: usize, v: i32) -> Result<(), String> {
        runtime_check!(v >= 0, "env_start must be >= 0, got {}", v);
        runtime_check!(v < 16, "env_start must be < 16, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 2, RegSetter::new(0xf0, v as u8));
        Ok(())
    }

    pub fn set_noise_env_dir(&self, index: usize, v: i32) -> Result<(), String> {
        runtime_check!(v == 0 || v == 1, "env_dir must be 0 or 1, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 2, RegSetter::new(0x08, v as u8));
        Ok(())
    }

    pub fn set_noise_env_period(&self, index: usize, v: i32) -> Result<(), String> {
        runtime_check!(v >= 0, "env_period must be >= 0, got {}", v);
        runtime_check!(v < 8, "env_period must be < 8, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 2, RegSetter::new(0x07, v as u8));
        Ok(())
    }

    pub fn set_noise_clock_shift(&self, index: usize, v: i32) -> Result<(), String> {
        runtime_check!(v >= 0, "clock_shift must be >= 0, got {}", v);
        runtime_check!(v < 14, "clock_shift must be < 14, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 3, RegSetter::new(0xf0, v as u8));
        Ok(())
    }

    pub fn set_noise_counter_width(&self, index: usize, v: i32) -> Result<(), String> {
        runtime_check!(v == 0 || v == 1, "counter_width must be 0 or 1, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 3, RegSetter::new(0x08, v as u8));
        Ok(())
    }

    pub fn set_noise_clock_divisor(&self, index: usize, v: i32) -> Result<(), String> {
        runtime_check!(v >= 0, "clock_divisor must be >= 0, got {}", v);
        runtime_check!(v < 8, "clock_divisor must be < 8, got {}", v);
        self.orit_at_index(index, self.channel as u16 + 3, RegSetter::new(0x07, v as u8));
        Ok(())
    }

    pub fn trigger_noise_with_length(&self, length: i32) -> Result<(), String> {
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
}
impl ScriptChannel for GbSquare {
    fn base(&self) -> u16 {
        self.channel as u16
    }
    fn settings_ring(&self) -> RefMut<'_, Vec<RegSettings>> {
        self.settings_ring.borrow_mut()
    }
    fn frame_number(&self) -> Ref<'_, usize> {
        self.frame_number.borrow()
    }
    fn resettable_settings_range(&self) -> RefMut<'_, Option<Range<usize>>> {
        self.resettable_settings_range.borrow_mut()
    }
    fn pending_settings_range(&self) -> RefMut<'_, Option<Range<usize>>> {
        self.pending_settings_range.borrow_mut()
    }

    fn trigger(&self) -> Result<(), String> {
        self.orit(
            self.channel as u16 + 4,
            // Trigger
            RegSetter::new(0x80, 1)
                // Length enable
                | RegSetter::new(0x40, 0),
        );
        Ok(())
    }

}
impl std::fmt::Debug for GbSquare {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GbSquare")
    }
}


#[derive(Clone)]
pub struct GbBindings {
    settings_ring: Rc<RefCell<Vec<RegSettings>>>,
    frame_number: Rc<RefCell<usize>>,
    channels: [GbSquare; 4],
}
impl std::fmt::Debug for GbBindings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GbBindings")
    }
}

impl GbBindings {
    pub fn wait_frames(&self, frames: i32) -> Result<(), String> {
        let len = self.settings_ring.borrow().len();
        // FIXME: Check that this isn't going back past the current frame
        // runtime_check!(frames >= 0, "frames must be >= 0, got {}", frames);
        runtime_check!(frames < len as i32, "frames must be < {}, got {}", len, frames);
        let mut frame_number = self.frame_number.borrow_mut();
        *frame_number = (*frame_number as i64 + frames as i64) as usize;
        Ok(())
    }

    fn set_frame_number(&self, v: usize) {
        *self.frame_number.borrow_mut() = v;
    }
    fn mark_pending_settings_as_resettable(&self) {
        self.channels[0].mark_pending_settings_as_resettable();
        self.channels[1].mark_pending_settings_as_resettable();
        self.channels[2].mark_pending_settings_as_resettable();
        self.channels[3].mark_pending_settings_as_resettable();
    }
    fn unmark_pending_settings_as_resettable(&self) {
        self.channels[0].unmark_pending_settings_as_resettable();
        self.channels[1].unmark_pending_settings_as_resettable();
        self.channels[2].unmark_pending_settings_as_resettable();
        self.channels[3].unmark_pending_settings_as_resettable();
    }
}

fn instrument_print(v: i32) {
  println!("Instruments: {:?}", v);
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

#[derive(Clone, Copy)]
struct PressedNote {
    note: u8,
    pressed_frame: usize,
    extended_frames: Option<usize>,
}

#[derive(Clone)]
struct InstrumentState {
    press_function: Option<test::WasmFunction>,
    release_function: Option<test::WasmFunction>,
    frame_function: Option<test::WasmFunction>,
    frames_after_release: i32,
    pressed_note: Option<PressedNote>,
}

impl Default for InstrumentState {
    fn default() -> Self {
        InstrumentState {
            press_function: None,
            release_function: None,
            frame_function: None,
            frames_after_release: 0,
            pressed_note: None,
        }
    }
}

trait InstrumentColArrayExt {
    fn get_instrument(&mut self, index: u8) -> Option<&mut InstrumentState>;
}
impl InstrumentColArrayExt for [Vec<InstrumentState>; NUM_INSTRUMENT_COLS] {
    fn get_instrument(&mut self, index: u8) -> Option<&mut InstrumentState> {
        // Column index is in the two lsb
        let col = &mut self[(index & 0x3) as usize];
        // Row index in the remaining bits
        col.get_mut((index >> 2) as usize)
    }
}

pub struct SynthScript {
    wasm_runtime: Rc<WasmRuntime>,
    wasm_exec_env: Option<WasmExecEnv>,
    script_context: Rc<GbBindings>,
    instrument_ids: Rc<RefCell<Vec<String>>>,
    instrument_states: Rc<RefCell<[Vec<InstrumentState>; NUM_INSTRUMENT_COLS]>>,
}

impl SynthScript {
    const DEFAULT_INSTRUMENTS: &'static [u8; 16733] = include_bytes!("../res/default-instruments.wasm");

    pub fn new(settings_ring: Rc<RefCell<Vec<RegSettings>>>) -> SynthScript {
        let instrument_ids: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![Default::default(); NUM_INSTRUMENTS]));
        let instrument_states: Rc<RefCell<[Vec<InstrumentState>; NUM_INSTRUMENT_COLS]>> = Default::default();
        let instrument_ids_clone = instrument_ids.clone();
        let instrument_states_clone = instrument_states.clone();

        let set_instrument_at_column = move |module: &test::WasmModuleInst, cid: &CStr, col: i32, press: &CStr, release: &CStr, frame: &CStr| -> () {
            let id = cid.to_str().unwrap();
            assert!(!id.is_empty(), "set_instrument_at_column: id must not be empty, got {:?}", id);
            assert!(!instrument_ids_clone.borrow().iter().any(|i| i == id), "set_instrument_at_column: id {} must be unique, but was already set", id);
            assert!(col >= 0 && col <= NUM_INSTRUMENT_COLS as i32,
                "set_instrument_at_column: column must be 0 <= col <= {}, got {}",
                NUM_INSTRUMENT_COLS, col);

            let mut state_cols = instrument_states_clone.borrow_mut();
            let (state, index) = {
                let state_col = &mut state_cols[col as usize];
                if state_col.len() >= 16 {
                    elog!("set_instrument_at_column: column {} already contains 16 instruments", col);
                }
                state_col.push(Default::default());
                // Column index is in the two lsb
                // 0, 1, 2, 3,
                // 4, 5, 6, 7,
                // ...
                let index = ((state_col.len() - 1) << 2) + col as usize;
                (&mut state_col.last_mut().unwrap(), index)
            };

            instrument_ids_clone.borrow_mut()[index] = id.to_owned();

            state.press_function = module.lookup_function(press);
            state.release_function = module.lookup_function(release);
            state.frame_function = module.lookup_function(frame);
        };

        let frame_number = Rc::new(RefCell::new(0));
        let square1 = GbSquare {
            channel: Square1,
            settings_ring: settings_ring.clone(),
            frame_number: frame_number.clone(),
            resettable_settings_range: RefCell::new(None),
            pending_settings_range: RefCell::new(None),
        };
        let square2 = GbSquare {
            channel: Square2,
            settings_ring: settings_ring.clone(),
            frame_number: frame_number.clone(),
            resettable_settings_range: RefCell::new(None),
            pending_settings_range: RefCell::new(None),
        };
        let wave = GbSquare {
            channel: Wave,
            settings_ring: settings_ring.clone(),
            frame_number: frame_number.clone(),
            resettable_settings_range: RefCell::new(None),
            pending_settings_range: RefCell::new(None),
        };
        let noise = GbSquare {
            channel: Noise,
            settings_ring: settings_ring.clone(),
            frame_number: frame_number.clone(),
            resettable_settings_range: RefCell::new(None),
            pending_settings_range: RefCell::new(None),
        };

        let gb = Rc::new(GbBindings {
            settings_ring: settings_ring.clone(),
            frame_number: frame_number,
            channels: [square1, square2, wave, noise],
        });

        let functions: Vec<Box<dyn test::HostFunction>> = vec![
            Box::new(test::HostFunctionSISSS::new("set_instrument_at_column", set_instrument_at_column)),
            Box::new(test::HostFunctionI::new("print", instrument_print)),
            Box::new(test::HostFunctionIi::new("to_square_gb_freq", GbSquare::to_square_gb_freq)),
            Box::new(test::HostFunctionIi::new("to_wave_gb_freq", GbSquare::to_wave_gb_freq)),
            Box::new(test::HostFunctionI::new("wait_frames", setter_gb_wrapper!(gb, wait_frames(v)))),
            Box::new(test::HostFunctionI::new("trigger", trigger_wrapper!(gb, trigger()))),
            Box::new(test::HostFunctionII::new("set_square_duty", setter_wrapper!(gb, set_square_duty(v)))),
            Box::new(test::HostFunctionII::new("set_square_sweep_time", setter_wrapper!(gb, set_square_sweep_time(v)))),
            Box::new(test::HostFunctionII::new("set_square_sweep_dir", setter_wrapper!(gb, set_square_sweep_dir(v)))),
            Box::new(test::HostFunctionII::new("set_square_sweep_shift", setter_wrapper!(gb, set_square_sweep_shift(v)))),
            Box::new(test::HostFunctionII::new("set_square_duty", setter_wrapper!(gb, set_square_duty(v)))),
            Box::new(test::HostFunctionII::new("set_square_env_start", setter_wrapper!(gb, set_square_env_start(v)))),
            Box::new(test::HostFunctionII::new("set_square_env_dir", setter_wrapper!(gb, set_square_env_dir(v)))),
            Box::new(test::HostFunctionII::new("set_square_env_period", setter_wrapper!(gb, set_square_env_period(v)))),
            Box::new(test::HostFunctionII::new("set_square_gb_freq", setter_wrapper!(gb, set_square_gb_freq(v)))),
            Box::new(test::HostFunctionII::new("set_square_freq", setter_wrapper!(gb, set_square_freq(v)))),
            Box::new(test::HostFunctionII::new("trigger_square_with_length", trigger_wrapper!(gb, trigger_square_with_length(v)))),
            Box::new(test::HostFunctionII::new("set_wave_playing", setter_wrapper!(gb, set_wave_playing(v)))),
            Box::new(test::HostFunctionII::new("set_wave_volume", setter_wrapper!(gb, set_wave_volume(v)))),
            Box::new(test::HostFunctionIA::new("set_wave_table", setter_table_wrapper!(gb, set_wave_table(v)))),
            Box::new(test::HostFunctionII::new("set_wave_gb_freq", setter_wrapper!(gb, set_wave_gb_freq(v)))),
            Box::new(test::HostFunctionII::new("set_wave_freq", setter_wrapper!(gb, set_wave_freq(v)))),
            Box::new(test::HostFunctionII::new("trigger_wave_with_length", trigger_wrapper!(gb, trigger_wave_with_length(v)))),
            Box::new(test::HostFunctionII::new("set_noise_env_start", setter_wrapper!(gb, set_noise_env_start(v)))),
            Box::new(test::HostFunctionII::new("set_noise_env_dir", setter_wrapper!(gb, set_noise_env_dir(v)))),
            Box::new(test::HostFunctionII::new("set_noise_env_period", setter_wrapper!(gb, set_noise_env_period(v)))),
            Box::new(test::HostFunctionII::new("set_noise_clock_shift", setter_wrapper!(gb, set_noise_clock_shift(v)))),
            Box::new(test::HostFunctionIAA::new("set_noise_clock_shift_frames", setter_frames_wrapper!(gb, set_noise_clock_shift(v) ))),
            Box::new(test::HostFunctionII::new("set_noise_counter_width", setter_wrapper!(gb, set_noise_counter_width(v)))),
            Box::new(test::HostFunctionIAA::new("set_noise_counter_width_frames", setter_frames_wrapper!(gb, set_noise_counter_width(v) ))),
            Box::new(test::HostFunctionII::new("set_noise_clock_divisor", setter_wrapper!(gb, set_noise_clock_divisor(v)))),
            Box::new(test::HostFunctionIAA::new("set_noise_clock_divisor_frames", setter_frames_wrapper!(gb, set_noise_clock_divisor(v) ))),
            Box::new(test::HostFunctionII::new("trigger_noise_with_length", trigger_wrapper!(gb, trigger_noise_with_length(v)))),
        ];
      
        let runtime = Rc::new(test::WasmRuntime::new(functions).unwrap());

        SynthScript {
            wasm_runtime: runtime,
            wasm_exec_env: None,
            script_context: gb,
            instrument_ids: instrument_ids,
            instrument_states: instrument_states,
        }
    }

    pub fn instrument_ids<'a>(&'a self) -> Ref<'a, Vec<String>> {
        self.instrument_ids.borrow()
    }

    fn reset_instruments(&mut self) {
        for state_col in &mut *self.instrument_states.borrow_mut() {
            state_col.clear();
        }
        for id in &mut *self.instrument_ids.borrow_mut() {
            *id = Default::default();
        }
    }

    pub fn load_default(&mut self, _frame_number: usize) {
        // self.script_engine
        //     .compile(SynthScript::DEFAULT_INSTRUMENTS)
        //     .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        //     .and_then(|ast| {
        //         self.set_instruments_ast(ast, frame_number)
        //             .map_err(|e| e as Box<dyn std::error::Error>)
        //     })
        //     .expect("Error loading default instruments.");
    }

    pub fn load_str(&mut self, _encoded: &str, _frame_number: usize) -> Result<(), Box<dyn std::error::Error>> {
        self.reset_instruments();

        // self.interpreter.run_code(encoded, None)?;
        // let ast = self.script_engine.compile(encoded)?;
        // self.set_instruments_ast(ast, frame_number)?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_file(&mut self, instruments_path: &std::path::Path, _frame_number: usize) -> Result<(), Box<dyn std::error::Error>> {
        self.reset_instruments();

        if instruments_path.exists() {
            let buffer = std::fs::read(instruments_path)?;
            let module = Rc::new(test::WasmModule::new(buffer, self.wasm_runtime.clone()).unwrap());
            let module_inst = Rc::new(test::WasmModuleInst::new(module).unwrap());
            self.wasm_exec_env = Some(test::WasmExecEnv::new(module_inst).unwrap());

            // let ast = self.script_engine.compile_file(instruments_path.to_path_buf())?;
            // self.interpreter.run_file(instruments_path).unwrap();
            // self.set_instruments_ast(ast, frame_number)?;
            Ok(())
        } else {
            return Err(format!("Project instruments file {:?} doesn't exist.", instruments_path).into());
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_as(&mut self, instruments_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut f = File::create(instruments_path)?;
        f.write_all(SynthScript::DEFAULT_INSTRUMENTS)?;
        f.flush()?;
        Ok(())
    }

    pub fn press_instrument_note(&mut self, frame_number: usize, instrument: u8, note: u8) -> () {
        {
            // The script themselves are modifying this state, so reset it.
            self.script_context.set_frame_number(frame_number);
            self.script_context.mark_pending_settings_as_resettable();
        }

        let mut states = self.instrument_states.borrow_mut();
        if let Some(state) = states.get_instrument(instrument) {
            if let Some(f) = &state.press_function {
                state.pressed_note = Some(PressedNote {
                    note: note,
                    pressed_frame: frame_number,
                    extended_frames: None,
                });
                self.wasm_exec_env.as_ref().unwrap().call_ii(*f, note as i32, Self::note_to_freq(note)).unwrap();
            }
        }

        // Only a press should be able to steal a channel, so unmark the channel settings
        // of non-reset channels after the press so that any frame or release function
        // running on top of those pending settings won't be resetting them first.
        self.script_context.unmark_pending_settings_as_resettable();
    }

    pub fn release_instrument(&mut self, frame_number: usize, instrument: u8) -> () {
        // The script themselves are modifying this state, so reset it.
        self.script_context.set_frame_number(frame_number);

        let mut states = self.instrument_states.borrow_mut();
        if let Some(state) = states.get_instrument(instrument) {
            if let (Some(f), Some(PressedNote { note, .. })) = (&state.release_function, &mut state.pressed_note) {
                self.wasm_exec_env.as_ref().unwrap().call_ii(*f, *note as i32, Self::note_to_freq(*note)).unwrap();
            }
            if let Some(PressedNote {
                note: _,
                pressed_frame: _,
                extended_frames,
            }) = &mut state.pressed_note
            {
                // Since the release function might trigger an envelope that lasts a few
                // frames, the frame function would need to continue running during that time.
                // The "frames" function will be run as long as pressed_note is some,
                // so if the instrument has set frames_after_release, transfer that info
                // into a countdown that the frame function runner will decrease, and then
                // finally empty `pressed_note`.
                if state.frames_after_release > 0 {
                    *extended_frames = Some(state.frames_after_release as usize)
                } else {
                    state.pressed_note = None;
                }
            }
        }
    }

    pub fn advance_frame(&mut self, frame_number: usize) {
        for state_col in &mut *self.instrument_states.borrow_mut() {
            for state in state_col {
                // Only run the frame function on instruments currently pressed.
                if let (
                    Some(f),
                    Some(PressedNote {
                        note,
                        pressed_frame,
                        extended_frames,
                    }),
                ) = (&state.frame_function, &mut state.pressed_note)
                {
                    // The script themselves are modifying this state, so reset it.
                    self.script_context.set_frame_number(frame_number);

                    self.wasm_exec_env.as_ref().unwrap().call_iii(
                        *f,
                        *note as i32,
                        Self::note_to_freq(*note),
                        (frame_number - *pressed_frame) as i32).unwrap();
                    if let Some(remaining) = extended_frames {
                        *remaining -= 1;
                        if *remaining == 0 {
                            // Finally empty `pressed_note` to prevent further
                            // runs of the frames function.
                            state.pressed_note = None;
                        }
                    }
                }
            }
        }
    }

    // fn set_instruments_ast(
    //     &mut self,
    //     ast: AST,
    //     frame_number: usize,
    // ) -> Result<(), std::boxed::Box<rhai::EvalAltResult>> {
    //     self.script_ast = ast;

    //     // The script might also contain sound settings directly in the its root.
    //     {
    //         self.script_context.set_frame_number(frame_number);
    //         self.script_context.mark_pending_settings_as_resettable();
    //         // FIXME: Also reset the gb states somewhere like gbsplay does
    //     }

    //     let mut scope = Scope::new();
    //     scope.push("gb", self.script_context.clone());

    //     self.script_engine.run_ast_with_scope(&mut scope, &self.script_ast)
    // }

    fn note_to_freq(note: u8) -> i32 {
        // let a = 440.0; // Frequency of A
        // let key_freq = (a / 32.0) * 2.0_f64.powf((note as f64 - 9.0) / 12.0);
        // key_freq
        NOTE_FREQUENCIES[note as usize] as i32
    }
}
