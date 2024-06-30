// Copyright © 2023 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use gba::prelude::*;

use super::SoundRendererTrait;
use crate::sound_engine::SoundEngine;
use crate::ui::MainWindow;
use crate::ui::Settings;
use crate::utils::WeakWindowWrapper;

use core::ptr::addr_of;
#[cfg(feature = "desktop")]
use std::sync::mpsc;
#[cfg(feature = "desktop")]
use std::sync::mpsc::Sender;

use slint::ComponentHandle;

const SQUARE_WAVE: u32 = 0x80_7f_80_7f;
// This must be in RAM to support a Fixed source DMA transfer
static mut ZEROES: u32 = 0x00000000;

pub struct Synth {
    sync_enabled: bool,
}

impl Synth {
    pub fn advance_frame(&mut self, _frame_number: usize, step_change: Option<u32>) {
        if self.sync_enabled {
            // The sequencer step changed, check if we need to send a pulse to sync slave devices.
            if let Some(next_step) = step_change {
                // Pocket Operator and Volca devices use 2 ppqm.
                let ppqm = 2;
                if next_step % ppqm == 0 {
                    // Reset the FIFO so that what we push gets played straight away.
                    SOUND_MIX.write(
                        SoundMix::new()
                            .with_psg(PsgMix::_25)
                            .with_sound_a_full(true)
                            .with_sound_a_left(true)
                            .with_sound_a_right(false)
                            .with_sound_a_timer(true) // true => timer1
                            .with_sound_a_reset(true),
                    );
                    // Already push 4 cycles to the FIFO, DMA will append ZEROES until the next pulse.
                    FIFO_A.write(SQUARE_WAVE);
                    FIFO_A.write(SQUARE_WAVE);
                }
            }
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

    pub fn apply_settings(&mut self, settings: &Settings) {
        self.sync_enabled = settings.sync_enabled;

        let volume_val = LeftRightVolume::new()
            .with_left_volume(7)
            .with_right_volume(7)
            .with_tone1_right(true)
            .with_tone2_right(true)
            .with_wave_right(true)
            .with_noise_right(true)
            .with_tone1_left(true)
            .with_tone2_left(true)
            .with_wave_left(true)
            .with_noise_left(true);

        if self.sync_enabled {
            // Mute PSG channels on the left.
            LEFT_RIGHT_VOLUME.write(volume_val.with_left_volume(0));

            // 8bits, not verified but 2 bits more are hopefully compensating for the 25% volume applied to the PSG above.
            SOUNDBIAS.write(
                SoundBias::new()
                    .with_bias_level(0x100)
                    .with_sample_cycle(SampleCycle::_8bit),
            );

            // Master sync signal is sent on the left channel through DirectSound channel A.
            // PSG is sent to the right channel, mixed at 25% volume to avoid clipping
            // at maximum level and be to a comparable level to instruments on the slave.
            SOUND_MIX.write(
                SoundMix::new()
                    .with_psg(PsgMix::_25)
                    .with_sound_a_full(true)
                    .with_sound_a_left(true)
                    .with_sound_a_right(false)
                    .with_sound_a_timer(true) // true => timer1
                    .with_sound_a_reset(true),
            );

            unsafe {
                // Set up the DMA to continually fill the FIFO with 00 samples so that the sound hardware keeps running.
                DMA1_SRC.write(addr_of!(ZEROES) as *const core::ffi::c_void);
                DMA1_DEST.write(FIFO_A.as_mut_ptr() as *mut core::ffi::c_void);
                DMA1_CONTROL.write(
                    DmaControl::new()
                        .with_enabled(true)
                        .with_start_time(DmaStartTime::Special)
                        .with_transfer_32bit(true)
                        .with_repeat(true)
                        .with_dest_addr_control(DestAddrControl::Fixed)
                        .with_src_addr_control(SrcAddrControl::Fixed),
                );
            }

            // The DirectSound sampling rate is configurable, this means that for a square wave we can set the
            // sampling rate to double the frequency, where a single high and then low byte are output for a cycle.
            // This reduces the CPU time spent filling the FIFO vs a regular sampling rate like 8kHz.
            TIMER1_RELOAD.write(0xffff - (16u32 * 1024 * 1024 / 64 / (300 * 2)) as u16);
            TIMER1_CONTROL.write(TimerControl::new().with_enabled(true).with_scale(TimerScale::_64));
        } else {
            // Full volume for all channels left and right
            LEFT_RIGHT_VOLUME.write(volume_val);

            // 6bit / 262.144kHz  (Best for PSG channels 1-4)
            SOUNDBIAS.write(
                SoundBias::new()
                    .with_bias_level(0x100)
                    .with_sample_cycle(SampleCycle::_6bit),
            );

            TIMER1_CONTROL.write(TimerControl::new().with_enabled(false));

            // 100% volume for the PSG, 0% for the DMA channels
            SOUND_MIX.write(SoundMix::new().with_psg(PsgMix::_100));
        }
    }

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
    // Already power it on
    SOUND_ENABLED.write(SoundEnable::new().with_enabled(true));

    let mut synth = Synth { sync_enabled: false };
    // Set-up the mixing and bias for sync disabled.
    synth.apply_settings(&Default::default());

    let sound_engine = SoundEngine::new(synth, WeakWindowWrapper::new(window.as_weak()));

    SoundRenderer { sound_engine }
}

impl SoundRendererTrait for SoundRenderer {
    fn invoke_on_sound_engine<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SoundEngine) + Send + 'static,
    {
        f(&mut self.sound_engine)
    }

    fn force(&mut self) {}
}
