use crate::synth_script::Channel::*;
use rhai::{AST, Dynamic, Engine, Scope};
use rhai::plugin::*;
use std::ops::BitOr;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct DmgSquare1 {
    nrx0_set_setting: Option<SetSetting>,
    nrx1_set_setting: Option<SetSetting>,
    nrx2_set_setting: Option<SetSetting>,
    nrx3_set_setting: Option<SetSetting>,
    nrx4_set_setting: Option<SetSetting>,
}
pub type SharedDmgSquare1 = Rc<RefCell<DmgSquare1>>;

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
    square1: SharedDmgSquare1,
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
    pub fn get_square1(this_rc: &mut SharedDmgBindings) -> SharedDmgSquare1 {
        this_rc.borrow().square1.clone()
    }
    #[rhai_fn(get = "wave", pure)]
    pub fn get_wave(this_rc: &mut SharedDmgBindings) -> SharedDmgWave {
        this_rc.borrow().wave.clone()
    }
    #[rhai_fn(get = "noise", pure)]
    pub fn get_noise(this_rc: &mut SharedDmgBindings) -> SharedDmgNoise {
        this_rc.borrow().noise.clone()
    }

    #[rhai_fn(global)]
    pub fn wait_frames(dmg: &mut SharedDmgBindings, frames: i32) {
        let mut this = dmg.borrow_mut();
        this.commit_to_ring();
        let len = this.settings_ring.borrow().len();
        this.settings_ring_index = (this.settings_ring_index + frames as usize) % len;
    }

    #[rhai_fn(set = "sweep_time", pure)]
    pub fn set_sweep_time(this_rc: &mut SharedDmgSquare1, v: i32) {
        assert!(v >= 0);
        assert!(v <= 7);
        orit(
            &mut this_rc.borrow_mut().nrx0_set_setting,
            SetSetting::new(Setting::new(Square1 as u16 + 0, 0x70), v as u8)
            );
    }
    #[rhai_fn(set = "sweep_dir", pure)]
    pub fn set_sweep_dir(this_rc: &mut SharedDmgSquare1, v: Direction) {
        orit(
            &mut this_rc.borrow_mut().nrx0_set_setting,
            SetSetting::new(Setting::new(Square1 as u16 + 0, 0x08), match v { Direction::Inc => 0, Direction::Dec => 1 })
            );
    }
    #[rhai_fn(set = "sweep_shift", pure)]
    pub fn set_sweep_shift(this_rc: &mut SharedDmgSquare1, v: i32) {
        assert!(v >= 0);
        assert!(v <= 7);
        orit(
            &mut this_rc.borrow_mut().nrx0_set_setting,
            SetSetting::new(Setting::new(Square1 as u16 + 0, 0x07), v as u8)
            );
    }

    #[rhai_fn(set = "duty", pure)]
    pub fn set_duty(this_rc: &mut SharedDmgSquare1, v: Duty) {
        orit(
            &mut this_rc.borrow_mut().nrx1_set_setting,
            SetSetting::new(Setting::new(Square1 as u16 + 1, 0xC0), v as u8)
            );
    }

    #[rhai_fn(set = "env_start", pure)]
    pub fn set_noise_env_start(this_rc: &mut SharedDmgSquare1, v: i32) {
        orit(
            &mut this_rc.borrow_mut().nrx2_set_setting,
            SetSetting::new(Setting::new(Square1 as u16 + 2, 0xf0), v as u8)
            );
    }
    #[rhai_fn(set = "env_dir", pure)]
    pub fn set_noise_env_dir(this_rc: &mut SharedDmgSquare1, v: Direction) {
        orit(
            &mut this_rc.borrow_mut().nrx2_set_setting,
            SetSetting::new(Setting::new(Square1 as u16 + 2, 0x08), v as u8)
            );
    }
    #[rhai_fn(set = "env_period", pure)]
    pub fn set_noise_env_period(this_rc: &mut SharedDmgSquare1, v: i32) {
        orit(
            &mut this_rc.borrow_mut().nrx2_set_setting,
            SetSetting::new(Setting::new(Square1 as u16 + 2, 0x07), v as u8)
            );
    }

    #[rhai_fn(global, name = "trigger_with_length")]
    pub fn square_trigger_with_length(this_rc: &mut SharedDmgSquare1, freq: f64, length: i32) {
        assert!(length >= 1);
        assert!(length <= 64);
        let gb_freq = to_gb_freq(freq);
        orit(
            &mut this_rc.borrow_mut().nrx1_set_setting,
            // Length load
            SetSetting::new(Setting::new(Square1 as u16 + 1, 0x3f), 64 - length as u8)
            );
        orit(
            &mut this_rc.borrow_mut().nrx3_set_setting,
            // Frequency LSB
            SetSetting::new(Setting::new(Square1 as u16 + 3, 0xff), (gb_freq & 0xff) as u8)
            );
        orit(
            &mut this_rc.borrow_mut().nrx4_set_setting,
            // Trigger
            SetSetting::new(Setting::new(Square1 as u16 + 4, 0x80), 1)
                // Length enable
                | SetSetting::new(Setting::new(Square1 as u16 + 4, 0x40), 1)
                // Frequency MSB
                | SetSetting::new(Setting::new(Square1 as u16 + 4, 0x07), (gb_freq >> 8) as u8)
            );
    }

    #[rhai_fn(global, name = "trigger")]
    pub fn square_trigger(this_rc: &mut SharedDmgSquare1, freq: f64) {
        let gb_freq = to_gb_freq(freq);
        orit(
            &mut this_rc.borrow_mut().nrx3_set_setting,
            // Frequency LSB
            SetSetting::new(Setting::new(Square1 as u16 + 3, 0xff), (gb_freq & 0xff) as u8)
            );
        orit(
            &mut this_rc.borrow_mut().nrx4_set_setting,
            // Trigger
            SetSetting::new(Setting::new(Square1 as u16 + 4, 0x80), 1)
                // Length enable
                | SetSetting::new(Setting::new(Square1 as u16 + 4, 0x40), 0)
                // Frequency MSB
                | SetSetting::new(Setting::new(Square1 as u16 + 4, 0x07), (gb_freq >> 8) as u8)
            );
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
    #[rhai_fn(set = "table", pure)]
    pub fn set_table(this_rc: &mut SharedDmgWave, hex_string: &str) {
        // Each hexadecimal character in the hex string is one 4 bits sample.
        this_rc.borrow_mut().wave_table_set_settings =
            (0..hex_string.len())
                .step_by(2)
                .map(|i| {
                    let byte = u8::from_str_radix(&hex_string[i..i + 2], 16).unwrap();
                    SetSetting::new(Setting::new((0xff30 + i / 2) as u16, 0xff), byte)
                })
                .collect();
    }
    #[rhai_fn(global, name = "trigger_with_length")]
    pub fn wave_trigger_with_length(this_rc: &mut SharedDmgWave, freq: f64, length: i32) {
        assert!(length >= 1);
        assert!(length <= 256);
        let gb_freq = to_gb_freq(freq);
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
    }
    #[rhai_fn(global, name = "trigger")]
    pub fn wave_trigger(this_rc: &mut SharedDmgWave, freq: f64) {
        let gb_freq = to_gb_freq(freq);
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
    }


    #[rhai_fn(set = "env_start", pure)]
    pub fn set_square_env_start(this_rc: &mut SharedDmgNoise, v: i32) {
        assert!(v >= 0);
        assert!(v <= 15);
        orit(
            &mut this_rc.borrow_mut().nrx2_set_setting,
            SetSetting::new(Setting::new(Noise as u16 + 2, 0xf0), v as u8)
            );
    }
    #[rhai_fn(set = "env_dir", pure)]
    pub fn set_square_env_dir(this_rc: &mut SharedDmgNoise, v: Direction) {
        orit(
            &mut this_rc.borrow_mut().nrx2_set_setting,
            SetSetting::new(Setting::new(Noise as u16 + 2, 0x08), v as u8)
            );
    }
    #[rhai_fn(set = "env_period", pure)]
    pub fn set_square_env_period(this_rc: &mut SharedDmgNoise, v: i32) {
        assert!(v >= 0);
        assert!(v <= 7);
        orit(
            &mut this_rc.borrow_mut().nrx2_set_setting,
            SetSetting::new(Setting::new(Noise as u16 + 2, 0x07), v as u8)
            );
    }
    #[rhai_fn(set = "clock_shift", pure)]
    pub fn set_clock_shift(this_rc: &mut SharedDmgNoise, v: i32) {
        assert!(v >= 0);
        assert!(v <= 15);
        orit(
            &mut this_rc.borrow_mut().nrx3_set_setting,
            SetSetting::new(Setting::new(Noise as u16 + 3, 0xf0), v as u8)
            );
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
    #[rhai_fn(set = "clock_divisor", pure)]
    pub fn set_clock_divisor_i32(this_rc: &mut SharedDmgNoise, v: i32) {
        assert!(v >= 0);
        assert!(v <= 7);
        orit(
            &mut this_rc.borrow_mut().nrx3_set_setting,
            SetSetting::new(Setting::new(Noise as u16 + 3, 0x07), v as u8)
            );
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
    pub fn to_gb_freq(freq: f64) -> i32 {
        2048 - (131072.0/freq).round() as i32
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
    _Square2 = 0xff15,
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
              .register_type::<SharedDmgSquare1>();
        engine.register_static_module("dmg", exported_module!(dmg_api).into());

        let square1 = Rc::new(RefCell::new(
            DmgSquare1 {
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
            wave: wave,
            noise: noise,
            }));

        let ast = engine.compile(SynthScript::DEFAULT_INSTRUMENTS).unwrap();
        let mut scope = Scope::new();
        scope.push("dmg", dmg.clone());

        SynthScript {
            script_engine: engine,
            script_ast: ast,
            script_context: dmg,
            script_scope: scope,
        }
    }

    pub fn trigger_instrument(&mut self, settings_ring_index: usize, instrument: u32, freq: f64) -> () {
        // The script themselves are modifying this state, so reset it.
        self.script_context.borrow_mut().set_settings_ring_index(settings_ring_index);

        let _result: () = self.script_engine.call_fn(
            &mut self.script_scope,
            &self.script_ast,
            format!("instrument_{}", instrument),
            ( freq, )
            ).unwrap();

        self.script_context.borrow_mut().commit_to_ring();
    }
}