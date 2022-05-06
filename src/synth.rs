// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::synth_script::Channel;
use crate::synth_script::RegSettings;
use crate::synth_script::SynthScript;
use crate::utils::MidiNote;
use crate::ChannelActiveNote;
use crate::ChannelTraceNote;
use crate::GlobalEngine;
use crate::MainWindow;
use crate::Settings;
use slint::Color;
use slint::Global;
use slint::Model;
use slint::SharedString;
use slint::VecModel;
use slint::Weak;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

// The pass-through channel is otherwise too loud compared to mixed content.
const SYNC_GAIN: f32 = 1.0 / 3.0;

enum PulseState {
    Zero,
    Up(u32, u32),
    Down(u32, u32),
}

struct SyncPulse {
    enabled: bool,
    state: PulseState,
    period: u32,
    loops: u32,
}

pub struct OutputData {
    pub buffer: Vec<f32>,
    pub buffer_viz: Vec<f32>,
    pub gain: f32,
    sync_pulse: SyncPulse,
}

struct FakePlayer {
    sample_rate: u32,
    state: Arc<Mutex<OutputData>>,
}

pub struct Synth {
    dmg: rboy::Sound,
    script: SynthScript,
    settings_ring: Rc<RefCell<Vec<RegSettings>>>,
    frame_number: usize,
    output_data: Arc<Mutex<OutputData>>,
    main_window: Weak<MainWindow>,
}

impl SyncPulse {
    fn new(enabled: bool, sample_rate: u32, tone_freq: u32, loops: u32) -> SyncPulse {
        let period = sample_rate / tone_freq;
        SyncPulse {
            enabled: enabled,
            state: PulseState::Up(period, 2),
            period: period,
            loops: loops,
        }
    }

    fn pulse(&mut self) {
        if self.enabled {
            self.state = PulseState::Up(self.period, self.loops);
        }
    }
}

impl Iterator for SyncPulse {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if !self.enabled {
            None
        } else {
            match self.state {
                PulseState::Zero => Some(0.0),
                PulseState::Up(p, l) => {
                    self.state = if p == 0 {
                        PulseState::Down(self.period, l)
                    } else {
                        PulseState::Up(p - 1, l)
                    };
                    Some(1.0)
                }
                PulseState::Down(p, l) => {
                    self.state = if p == 0 {
                        if l == 0 {
                            PulseState::Zero
                        } else {
                            PulseState::Up(self.period, l - 1)
                        }
                    } else {
                        PulseState::Down(p - 1, l)
                    };
                    Some(-1.0)
                }
            }
        }
    }
}

impl rboy::AudioPlayer for FakePlayer {
    fn play(&mut self, left_channel: &[f32], right_channel: &[f32], viz_channel: &[f32]) {
        let mut left_iter = left_channel.iter();
        let mut right_iter = right_channel.iter();
        let mut state = self.state.lock().unwrap();
        let gain = state.gain;
        state.buffer_viz.clear();
        state.buffer_viz.extend_from_slice(viz_channel);

        state.buffer.reserve(left_channel.len() * 2);
        while let Some(left) = left_iter.next() {
            if let Some(pulse_sample) = state.sync_pulse.next() {
                let right = (left + right_iter.next().unwrap()) / 2.0;
                state.buffer.push(pulse_sample);
                state.buffer.push(right * gain);
            } else {
                state.buffer.push(*left * gain);
                state.buffer.push(*right_iter.next().unwrap() * gain);
            }
        }
    }
    fn samples_rate(&self) -> u32 {
        self.sample_rate
    }
    fn underflowed(&self) -> bool {
        // We're always underflowed, we advance frames when we need to fill the audio buffer.
        true
    }
}

impl Synth {
    pub fn new(main_window: Weak<MainWindow>, sample_rate: u32, settings: Settings) -> Synth {
        let settings_ring = Rc::new(RefCell::new(vec![RegSettings::new(); 512]));
        let script = SynthScript::new(settings_ring.clone());

        let gain = if settings.sync_enabled { SYNC_GAIN } else { 1.0 };

        let output_data = Arc::new(Mutex::new(OutputData {
            buffer: Vec::new(),
            buffer_viz: Vec::new(),
            gain: gain,
            sync_pulse: SyncPulse::new(settings.sync_enabled, sample_rate, 300, 2),
        }));

        let player = Box::new(FakePlayer {
            sample_rate: sample_rate,
            state: output_data.clone(),
        });
        let mut dmg = rboy::Sound::new(player);
        // Already power it on.
        dmg.wb(0xff26, 0x80);

        Synth {
            dmg: dmg,
            script: script,
            settings_ring: settings_ring,
            frame_number: 0,
            output_data: output_data,
            main_window: main_window,
        }
    }

    pub fn output_data(&self) -> Arc<Mutex<OutputData>> {
        self.output_data.clone()
    }

    pub fn apply_settings(&mut self, settings: Settings) {
        let mut output_data = self.output_data.lock().unwrap();
        output_data.gain = if settings.sync_enabled { SYNC_GAIN } else { 1.0 };
        output_data.sync_pulse.enabled = settings.sync_enabled;
    }

    // The Gameboy APU has 512 frames per second where various registers are read,
    // but all registers are eventually read at least once every 8 of those frames.
    // So clock our frame generation at 64hz, thus this function is expected
    // to be called 64x per second.
    pub fn advance_frame(&mut self, step_change: Option<u32>) {
        {
            let i = self.settings_ring_index();
            let mut settings_ring = self.settings_ring.borrow_mut();
            let dmg = &mut self.dmg;
            settings_ring[i].for_each_setting(|addr, set| {
                // Trying to read the memory wouldn't give us the value we wrote last,
                // so overwrite any state previously set in bits outside of RegSetter.mask
                // with zeros.
                dmg.wb(addr, set.value);
            });
            settings_ring[i].clear_all();
        }

        // Just enable all channels for now
        self.dmg.wb(0xff24, 0xff);
        self.dmg.wb(0xff25, 0xff);

        // The sequencer step changed, check if we need to send a pulse to sync downstream devices.
        if let Some(next_step) = step_change {
            // Pocket Operator and Volca devices use 2 ppqm.
            let ppqm = 2;
            if next_step % ppqm == 0 {
                self.output_data.lock().unwrap().sync_pulse.pulse();
            }
        }

        // Generate one frame of mixed output.
        // For 44100hz audio, this will put 44100/64 audio samples in self.buffer.
        self.dmg.do_cycle(rboy::CLOCKS_PER_SECOND / 64);

        self.update_ui_channel_states();
        self.frame_number += 1;
    }

    pub fn press_instrument_note(&mut self, instrument: u32, note: u32) -> () {
        self.script.press_instrument_note(self.frame_number, instrument, note);
    }

    pub fn release_instrument(&mut self, instrument: u32) -> () {
        self.script.release_instrument(self.frame_number, instrument);
    }

    /// Can be used to manually mute when instruments have an infinite length and envelope.
    pub fn mute_instruments(&mut self) {
        // Set the envelopes to 0.
        self.dmg.wb(Channel::Square1 as u16 + 2, 0);
        self.dmg.wb(Channel::Square2 as u16 + 2, 0);
        self.dmg.wb(Channel::Wave as u16 + 2, 0);
        self.dmg.wb(Channel::Noise as u16 + 2, 0);
    }

    fn update_instrument_ids(&self) {
        let ids = self.script.instrument_ids();
        self.main_window.clone().upgrade_in_event_loop(move |handle| {
            let model = GlobalEngine::get(&handle).get_instruments();
            for (i, id) in ids.iter().enumerate() {
                let mut row_data = model.row_data(i).unwrap();
                row_data.id = SharedString::from(id);
                model.set_row_data(i, row_data);
            }
        });
    }
    #[cfg(target_arch = "wasm32")]
    pub fn load(&mut self, maybe_base64: Option<String>) {
        self.script.load(maybe_base64);
        self.update_instrument_ids();
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(&mut self, project_instruments_path: &std::path::Path) {
        self.script.load(project_instruments_path);
        self.update_instrument_ids();
    }

    fn settings_ring_index(&self) -> usize {
        self.frame_number % self.settings_ring.borrow().len()
    }

    fn update_ui_channel_states(&self) {
        let frame_number = self.frame_number as i32;
        let mut states = self.dmg.chan_states();
        // Let square channels be rendered on top of the wave channel.
        states.reverse();
        let colors = [
            Color::from_rgb_u8(192, 0, 0),
            Color::from_rgb_u8(0, 192, 0),
            Color::from_rgb_u8(64, 0, 192),
            Color::from_rgb_u8(0, 64, 192),
        ];

        self.main_window.clone().upgrade_in_event_loop(move |handle| {
            let global = GlobalEngine::get(&handle);
            let trace_model = global.get_synth_trace_notes();
            let trace_vec_model = trace_model
                .as_any()
                .downcast_ref::<VecModel<ChannelTraceNote>>()
                .unwrap();
            let active_model = global.get_synth_active_notes();
            let active_vec_model = active_model
                .as_any()
                .downcast_ref::<VecModel<ChannelActiveNote>>()
                .unwrap();

            while trace_vec_model.row_count() > 0 {
                // FIXME: Provide the oldest tick number to the UI or get it from it
                if frame_number - trace_vec_model.row_data(0).unwrap().tick_number >= 6 * 16 * 2 {
                    trace_vec_model.remove(0);
                } else {
                    break;
                }
            }
            // FIXME: Keep notes that are still active instead of re-adding?
            active_vec_model.set_vec(Vec::new());

            for (&color, &(freq, vol)) in colors.iter().zip(states.iter()) {
                if vol > 0 {
                    let (trace, active) = if let Some((note, _cents)) = freq.map(MidiNote::from_freq) {
                        let trace = ChannelTraceNote {
                            tick_number: frame_number,
                            octave: note.octave(),
                            key_pos: note.key_pos(),
                            is_black: note.is_black(),
                            volume: vol as f32 / 15.0,
                            color: color,
                        };
                        let active = ChannelActiveNote {
                            trace: trace.clone(),
                            note_name: note.name(),
                        };
                        (trace, active)
                    } else {
                        let trace = ChannelTraceNote {
                            tick_number: frame_number,
                            octave: 0,
                            key_pos: 0,
                            is_black: false,
                            volume: vol as f32 / 15.0,
                            color: color,
                        };
                        let active = ChannelActiveNote {
                            trace: trace.clone(),
                            note_name: "*".into(),
                        };
                        (trace, active)
                    };
                    trace_vec_model.push(trace);
                    active_vec_model.push(active);
                }
            }
            global.set_current_tick_number(frame_number);
        });
    }
}
