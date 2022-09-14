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

use slint::Global;
use slint::Model;
use slint::SharedString;
use slint::VecModel;
use slint::Weak;

use std::cell::Ref;
use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

// The pass-through channel is otherwise too loud compared to mixed content.
const SYNC_GAIN: f32 = 1.0 / 3.0;
const VBLANK_CYCLES: u32 = 70224;

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

    pub fn apply_settings(&mut self, settings: &Settings) {
        let mut output_data = self.output_data.lock().unwrap();
        output_data.gain = if settings.sync_enabled { SYNC_GAIN } else { 1.0 };
        output_data.sync_pulse.enabled = settings.sync_enabled;
    }

    // GameBoy games seem to use the main loop clocked to the screen's refresh rate
    // to also drive the sound chip. To keep the song timing, also use the same 59.73hz
    // frame refresh rate.
    pub fn advance_frame(&mut self, step_change: Option<u32>) {
        self.script.advance_frame(self.frame_number);

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
        // For 44100hz audio, this will put 44100/59.73 audio samples in self.buffer.
        self.dmg.do_cycle(VBLANK_CYCLES);

        self.update_ui_channel_states();
        self.frame_number += 1;
    }

    pub fn press_instrument_note(&mut self, instrument: u8, note: u32) -> () {
        self.script.press_instrument_note(self.frame_number, instrument, note);
    }

    pub fn release_instrument(&mut self, instrument: u8) -> () {
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

    pub fn instrument_ids<'a>(&'a self) -> Ref<'a, Vec<String>> {
        self.script.instrument_ids()
    }

    fn update_instrument_ids(&self) {
        let ids = self.script.instrument_ids().clone();
        self.main_window.upgrade_in_event_loop(move |handle| {
            let model = GlobalEngine::get(&handle).get_instruments();
            for (i, id) in ids.iter().enumerate() {
                let mut row_data = model.row_data(i).unwrap();
                row_data.id = SharedString::from(id);
                model.set_row_data(i, row_data);
            }
        });
    }

    pub fn load_default(&mut self) {
        self.script.load_default(self.frame_number);
        self.update_instrument_ids();
    }

    pub fn load_str(&mut self, encoded: &str) -> Result<(), Box<dyn Error>> {
        self.script.load_str(encoded, self.frame_number)?;
        self.update_instrument_ids();
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_file(&mut self, instruments_path: &std::path::Path) -> Result<(), Box<dyn Error>> {
        self.script.load_file(instruments_path, self.frame_number)?;
        self.update_instrument_ids();
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_as(&mut self, instruments_path: &std::path::Path) -> Result<(), Box<dyn Error>> {
        self.script.save_as(instruments_path)
    }

    fn settings_ring_index(&self) -> usize {
        self.frame_number % self.settings_ring.borrow().len()
    }

    fn update_ui_channel_states(&self) {
        let frame_number = self.frame_number as i32;
        let mut states = self.dmg.chan_states();
        // Let square channels be rendered on top of the wave channel.
        states.reverse();

        self.main_window.upgrade_in_event_loop(move |handle| {
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

            // FIXME: Provide the oldest tick number to the UI or get it from it
            let visible_ticks = 6 * 16 * 2;

            // First remove traces that would disappear after being shrunk as this affects indices.
            while trace_vec_model.row_count() > 0 {
                let trace = trace_vec_model.row_data(0).unwrap();
                if frame_number - (trace.start_tick + trace.num_ticks) >= visible_ticks {
                    trace_vec_model.remove(0);
                } else {
                    break;
                }
            }

            let mut last_chan_trace_index: [Option<usize>; 4] = [None; 4];
            for i in 0..trace_vec_model.row_count() {
                let mut trace = trace_vec_model.row_data(i).unwrap();

                // Remember whether this trace is a candidate to be merged with current channel traces.
                if trace.start_tick + trace.num_ticks == frame_number {
                    last_chan_trace_index[trace.channel as usize] = Some(i);
                }

                // Shrink old traces that span past the trace history range.
                if frame_number - trace.start_tick >= visible_ticks {
                    let diff = frame_number - trace.start_tick - visible_ticks;
                    trace.start_tick += diff;
                    trace.num_ticks -= diff;
                    trace_vec_model.set_row_data(i, trace);
                }
            }

            // FIXME: Keep notes that are still active instead of re-adding?
            active_vec_model.set_vec(Vec::new());

            for (channel, &(freq, vol)) in states.iter().enumerate() {
                if vol > 0 {
                    let (trace, active) = if let Some((note, cent_adj)) = freq.map(MidiNote::from_freq) {
                        let semitone = note.semitone();
                        // Stretch the between-notes range for white notes not followed by a black note.
                        let cent_factor = if (semitone == 4 || semitone == 11) && cent_adj > 0.0
                            || (semitone == 5 || semitone == 0) && cent_adj < 0.0
                        {
                            1.0
                        } else {
                            0.5
                        };

                        let trace = ChannelTraceNote {
                            channel: channel as i32,
                            start_tick: frame_number,
                            num_ticks: 1,
                            octave: note.octave(),
                            key_pos: note.key_pos(),
                            cent_adj: cent_adj * cent_factor,
                            is_black: note.is_black(),
                            volume: vol as f32 / 15.0,
                        };
                        let active = ChannelActiveNote {
                            trace: trace.clone(),
                            note_name: note.name().into(),
                        };
                        (trace, active)
                    } else {
                        let trace = ChannelTraceNote {
                            channel: channel as i32,
                            start_tick: frame_number,
                            num_ticks: 1,
                            octave: 0,
                            key_pos: 0,
                            cent_adj: 0.0,
                            is_black: false,
                            volume: vol as f32 / 15.0,
                        };
                        let active = ChannelActiveNote {
                            trace: trace.clone(),
                            note_name: "*".into(),
                        };
                        (trace, active)
                    };

                    let maybe_last_index = last_chan_trace_index[trace.channel as usize];
                    maybe_last_index
                        .and_then(|last_index| {
                            let mut last = trace_vec_model.row_data(last_index).unwrap();
                            if last.channel == trace.channel
                                && last.octave == trace.octave
                                && last.key_pos == trace.key_pos
                                && last.cent_adj == trace.cent_adj
                                && last.is_black == trace.is_black
                                && last.volume == trace.volume
                            {
                                last.num_ticks += trace.num_ticks;
                                Some(trace_vec_model.set_row_data(last_index, last))
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| trace_vec_model.push(trace));

                    active_vec_model.push(active);
                }
            }
            global.set_current_tick_number(frame_number);
        });
    }
}
