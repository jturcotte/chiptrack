// Copyright © 2023 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use gba::prelude::*;

use crate::sound_engine::SoundEngine;
use crate::utils::WeakWindowWrapper;
use crate::MainWindow;
use crate::Settings;

use slint::ComponentHandle;

#[cfg(feature = "desktop")]
use std::sync::mpsc;
#[cfg(feature = "desktop")]
use std::sync::mpsc::Sender;

pub struct Synth {}

impl Synth {
    pub fn advance_frame(&mut self, _frame_number: usize, _step_change: Option<u32>) {
        // Just enable all channels for now
        unsafe {
            *(0x4000080 as *mut u16) = 0xffff;
        }
    }

    pub fn set_sound_reg_callback(&self) -> impl Fn(i32, i32) {
        // FIXME: Check the address allowed bounds
        move |addr: i32, value: i32| {
            // log!("{:#x}: {:#04x} ({:#010b})", addr, value, value);
            unsafe {
                *(addr as *mut u16) = value as u16;
            }
        }
    }

    pub fn set_wave_table_callback(&self) -> impl Fn(&[u8]) {
        move |table: &[u8]| {
            // log!("set_wave_table: {:?}", table);
            unsafe {
                // FIXME: Handle banks here or in zig
                let dst_ptr = 0x4000090 as *mut u8;
                let src_ptr = &table[0] as *const u8;
                core::ptr::copy_nonoverlapping(src_ptr, dst_ptr, table.len());
            }
        }
    }

    pub fn apply_settings(&mut self, _settings: &Settings) {}

    pub fn mute_instruments(&mut self) {
        TONE1_PATTERN.write(TonePattern::new().with_volume(0));
        TONE2_PATTERN.write(TonePattern::new().with_volume(0));
        WAVE_LEN_VOLUME.write(WaveLenVolume::new().with_volume(0));
        NOISE_LEN_ENV.write(NoiseLenEnvelope::new().with_volume(0));
    }
}

pub struct SoundRenderer {
    pub sound_engine: SoundEngine,
}

pub fn new_sound_renderer(window: &MainWindow) -> SoundRenderer {
    // Already power it on.
    SOUND_ENABLED.write(SoundEnable::new().with_enabled(true));
    // 6bit / 262.144kHz  (Best for PSG channels 1-4, we don't use DMA channels anyway)
    SOUNDBIAS.write(
        SoundBias::new()
            .with_bias_level(0x100)
            .with_sample_cycle(SampleCycle::_6bit),
    );

    let synth = Synth {};
    let sound_engine = SoundEngine::new(synth, WeakWindowWrapper::new(window.as_weak()));

    SoundRenderer { sound_engine }
}

impl SoundRenderer {
    pub fn invoke_on_sound_engine<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SoundEngine) + Send + 'static,
    {
        f(&mut self.sound_engine)
    }

    pub fn force(&mut self) {}
}
