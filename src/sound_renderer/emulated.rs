// Copyright © 2023 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::sound_engine::SoundEngine;
use crate::ui::ChannelActiveNote;
use crate::ui::ChannelTraceNote;
use crate::ui::GlobalEngine;
use crate::ui::GlobalSettings;
use crate::ui::MainWindow;
use crate::ui::Settings;
use crate::utils::MidiNote;
use crate::utils::WeakWindowWrapper;
use core::iter::repeat;

use alloc::collections::VecDeque;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use notify::{DebouncedEvent, RecursiveMode, Watcher};
use once_cell::unsync::Lazy;
use rboy::VizChunk;
use slint::{ComponentHandle, Global, Model, SharedString, VecModel};

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use super::SoundRendererTrait;

thread_local! {static SOUND_ENGINE: RefCell<Option<SoundEngine>> = RefCell::new(None);}
thread_local! {static SOUND_SENDER: RefCell<Option<std::sync::mpsc::Sender<Box<dyn FnOnce(&mut SoundEngine) + Send>>>> = RefCell::new(None);}

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
    pub viz_chunk: Option<VizChunk>,
    pub gain: f32,
    sync_pulse: SyncPulse,
}

struct FakePlayer {
    sample_rate: u32,
    state: Arc<Mutex<OutputData>>,
}

pub struct Synth {
    dmg: Rc<RefCell<rboy::Sound>>,
    output_data: Arc<Mutex<OutputData>>,
    main_window: WeakWindowWrapper,
}

impl SyncPulse {
    fn new(enabled: bool, sample_rate: u32, tone_freq: u32, loops: u32) -> SyncPulse {
        let period = sample_rate / tone_freq;
        SyncPulse {
            enabled,
            state: PulseState::Up(period, 2),
            period,
            loops,
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
    fn play(&mut self, left_channel: &[f32], right_channel: &[f32], viz_chunk: VizChunk) {
        let left_iter = left_channel.iter();
        let mut right_iter = right_channel.iter();
        let mut state = self.state.lock().unwrap();
        let gain = state.gain;
        state.viz_chunk = Some(viz_chunk);

        state.buffer.reserve(left_channel.len() * 2);
        for left in left_iter {
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

pub enum Channel {
    Square1 = 0xff10,
    Square2 = 0xff15,
    Wave = 0xff1a,
    Noise = 0xff1f,
}

impl Synth {
    pub fn new(main_window: WeakWindowWrapper, sample_rate: u32, settings: Settings) -> Synth {
        let gain = if settings.sync_enabled { SYNC_GAIN } else { 1.0 };

        let output_data = Arc::new(Mutex::new(OutputData {
            buffer: Vec::new(),
            viz_chunk: None,
            gain,
            sync_pulse: SyncPulse::new(settings.sync_enabled, sample_rate, 300, 2),
        }));

        let player = Box::new(FakePlayer {
            sample_rate,
            state: output_data.clone(),
        });
        let mut dmg = rboy::Sound::new_cgb(player);
        // Already power it on.
        dmg.wb(0xff26, 0x80);
        // And enable all channels
        dmg.wb(0xff24, 0xff);
        dmg.wb(0xff25, 0xff);

        Synth {
            dmg: Rc::new(RefCell::new(dmg)),
            output_data,
            main_window,
        }
    }

    // GameBoy games seem to use the main loop clocked to the screen's refresh rate
    // to also drive the sound chip. To keep the song timing, also use the same 59.73hz
    // frame refresh rate.
    pub fn advance_frame(&mut self, frame_number: usize, step_change: Option<u32>) {
        {
            let dmg = &mut self.dmg.borrow_mut();

            // The sequencer step changed, check if we need to send a pulse to sync downstream devices.
            if let Some(next_step) = step_change {
                // Pocket Operator and Volca devices use 2 ppqm.
                let ppqm = 2;
                if next_step % ppqm == 0 {
                    self.output_data.lock().unwrap().sync_pulse.pulse();
                }
            }

            // Generate one frame of mixed output.
            // For 44100hz audio, this will put 44100/59.73 audio samples in output_data.buffer.
            dmg.do_cycle(VBLANK_CYCLES);
        }

        self.update_ui_channel_states(frame_number as i32);
    }

    pub fn set_sound_reg_callback(&self) -> impl Fn(i32, i32) {
        let dmg_cell = self.dmg.clone();
        move |addr: i32, value: i32| {
            let (maybe_lsb, maybe_msb) = Synth::gba_to_gb_addr(addr);
            let mut dmg = dmg_cell.borrow_mut();
            if let Some(a) = maybe_lsb {
                dmg.wb(a, value as u8);
            }
            if let Some(a) = maybe_msb {
                dmg.wb(a, (value >> 8) as u8);
            }
        }
    }

    pub fn set_wave_table_callback(&self) -> impl Fn(&[u8]) {
        let dmg_cell = self.dmg.clone();
        move |table: &[u8]| {
            let mut dmg = dmg_cell.borrow_mut();
            for (i, v) in table.iter().take(16).enumerate() {
                dmg.wb((0xff30 + i) as u16, *v);
            }
        }
    }

    pub fn apply_settings(&mut self, settings: &Settings) {
        let mut output_data = self.output_data.lock().unwrap();
        output_data.gain = if settings.sync_enabled { SYNC_GAIN } else { 1.0 };
        output_data.sync_pulse.enabled = settings.sync_enabled;
    }

    pub fn mute_instruments(&mut self) {
        let dmg = &mut self.dmg.borrow_mut();
        // Set the envelopes to 0.
        dmg.wb(Channel::Square1 as u16 + 2, 0);
        dmg.wb(Channel::Square2 as u16 + 2, 0);
        dmg.wb(Channel::Wave as u16 + 2, 0);
        dmg.wb(Channel::Noise as u16 + 2, 0);
    }

    pub fn output_data(&self) -> Arc<Mutex<OutputData>> {
        self.output_data.clone()
    }

    fn update_ui_channel_states(&self, frame_number: i32) {
        let mut states = self.dmg.borrow().chan_states();
        // Let square channels be rendered on top of the wave channel.
        states.reverse();

        self.main_window
            .upgrade_in_event_loop(move |handle| {
                let global = GlobalEngine::get(&handle);
                global.set_last_synth_tick(frame_number);

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
                                    trace_vec_model.set_row_data(last_index, last);
                                    Some(())
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_else(|| trace_vec_model.push(trace));

                        active_vec_model.push(active);
                    }
                }
            })
            .unwrap();
    }

    fn gba_to_gb_addr(gba_addr: i32) -> (Option<u16>, Option<u16>) {
        match gba_addr {
            0x4000060 => (Some(0xFF10), None),         // NR10
            0x4000062 => (Some(0xFF11), Some(0xFF12)), // NR11, NR12
            0x4000064 => (Some(0xFF13), Some(0xFF14)), // NR13, NR14
            0x4000068 => (Some(0xFF16), Some(0xFF17)), // NR21, NR22
            0x400006C => (Some(0xFF18), Some(0xFF19)), // NR23, NR24
            0x4000070 => (Some(0xFF1A), None),         // NR30
            0x4000072 => (Some(0xFF1B), Some(0xFF1C)), // NR31, NR32
            0x4000074 => (Some(0xFF1D), Some(0xFF1E)), // NR33, NR34
            0x4000078 => (Some(0xFF20), Some(0xFF21)), // NR41, NR42
            0x400007C => (Some(0xFF22), Some(0xFF23)), // NR43, NR44
            0x4000080 => (Some(0xFF24), Some(0xFF25)), // NR50, NR51
            _ => (None, None),
        }
    }
}

pub fn invoke_on_sound_engine<F>(f: F)
where
    F: FnOnce(&mut SoundEngine) + Send + 'static,
{
    SOUND_SENDER
        .with(|s| {
            s.borrow_mut()
                .as_ref()
                .expect("Should be initialized")
                .send(Box::new(f))
        })
        .unwrap();
}

pub struct Context {
    sample_rate: u32,
    _stream: cpal::Stream,
}

pub struct SoundRenderer<LazyF: FnOnce() -> Context> {
    sound_send: Sender<Box<dyn FnOnce(&mut SoundEngine) + Send>>,
    context: Rc<Lazy<Context, LazyF>>,
    #[cfg(not(target_arch = "wasm32"))]
    watcher: notify::RecommendedWatcher,
    #[cfg(not(target_arch = "wasm32"))]
    watched_path: Option<PathBuf>,
    viz_chunks: Arc<Mutex<VecDeque<VizChunk>>>,
    // How many samples in viz_chunks are buffered for future waveform rendering frames.
    viz_tail_len: Arc<Mutex<usize>>,
    // The rendering tick that was used to render the last waveform.
    last_viz_chunk_tick: f32,
}

impl<LazyF: FnOnce() -> Context> SoundRendererTrait for SoundRenderer<LazyF> {
    fn invoke_on_sound_engine<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SoundEngine) + Send + 'static,
    {
        Lazy::force(&*self.context);
        self.sound_send.send(Box::new(f)).unwrap();
    }

    fn force(&mut self) {
        Lazy::force(&*self.context);
    }
}

impl<LazyF: FnOnce() -> Context> SoundRenderer<LazyF> {
    pub fn invoke_on_sound_engine_no_force<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SoundEngine) + Send + 'static,
    {
        self.sound_send.send(Box::new(f)).unwrap();
    }

    pub fn sender(&self) -> Sender<Box<dyn FnOnce(&mut SoundEngine) + Send>> {
        self.sound_send.clone()
    }

    pub fn set_song_path(&mut self, path: PathBuf) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(old_path) = self.watched_path.take() {
                self.watcher.unwatch(old_path).unwrap();
            }

            debug_assert!(path.is_file());
            self.watcher
                .watch(path.parent().unwrap(), RecursiveMode::NonRecursive)
                .unwrap();
            self.watched_path = Some(path);
        }
    }

    #[cfg(feature = "desktop")]
    pub fn update_waveform(&mut self, tick: f32, width: f32, height: f32) -> SharedString {
        let sample_rate = match Lazy::get(&*self.context) {
            Some(context) => context.sample_rate,
            None => return Default::default(),
        };

        let viz_chunks = self.viz_chunks.lock().unwrap();
        let mut viz_tail_len = self.viz_tail_len.lock().unwrap();

        let viz_len = viz_chunks.iter().map(|vc| vc.channels[0].len()).sum::<usize>();
        // How many samples we visualize, take 3/2 frames when available.
        let render_len = (viz_len - *viz_tail_len).min(sample_rate as usize * 3 / 2 / 60);
        // Find the visualization mid-point where we'll align wave starts for square and wave channels.
        let render_mid_len = render_len / 2;
        // The mid point is half a screen past the viz_tail_len, starting from the most recent (but buffered) sample.
        let render_mid_offset = *viz_tail_len + render_mid_len;

        let find_wave_start_at_mid_offset = |viz_chunks: &VecDeque<VizChunk>, chan_num: usize| -> usize {
            // Iterate chunks backwards, from the freshest to the oldest
            viz_chunks
                .iter()
                .rev()
                // For each chunk, accumulate the offset relative to the freshest VizChunk
                .scan(0usize, |state, vc| {
                    let offset = *state;
                    *state += vc.channels[chan_num].len();
                    Some((vc, offset))
                })
                // Skip chunks that won't contain the mid-point anyway
                .skip_while(|(vc, vc_o)| vc_o + vc.channels[chan_num].len() < render_mid_offset)
                // Add the chunk's absolute offset to each wave_start_offsets (which is relative to its chunk)
                .flat_map(|(vc, vc_o)| {
                    let l = vc.channels[chan_num].len();
                    // Also make sure to flatmap the reverse of the offsets iterator.
                    vc.wave_start_offsets[chan_num]
                        .iter()
                        .map(move |o| (l - o) + vc_o)
                        .rev()
                })
                // Look for a wave start right after our rendering mid-point, but not for more than one screen after.
                // TODO: I should probably base this decision on the lowest supported frequency (32hz vs sample rate)
                .take_while(|o| *o < render_mid_offset + render_len)
                // Take the first offset past the mid-point, we want to align that offset of the waveform to the screen's middle.
                .find(|o| *o > render_mid_offset)
                // If the channel is silent or that the wave start is too far before or after, don't offset the waveform.
                .unwrap_or(render_mid_offset)
        };

        let chan0_render_midpoint = find_wave_start_at_mid_offset(&viz_chunks, 0);
        let chan1_render_midpoint = find_wave_start_at_mid_offset(&viz_chunks, 1);
        let chan2_render_midpoint = find_wave_start_at_mid_offset(&viz_chunks, 2);

        let aligned_chan_samples = |chan_num: usize, chan_render_midpoint| {
            viz_chunks
                .iter()
                .flat_map(move |vc| vc.channels[chan_num].iter().copied())
                .rev()
                .skip(chan_render_midpoint - render_mid_len)
                .chain(repeat(0.0))
                .take(render_len)
        };

        let chan0_samples = aligned_chan_samples(0, chan0_render_midpoint);
        let chan1_samples = aligned_chan_samples(1, chan1_render_midpoint);
        let chan2_samples = aligned_chan_samples(2, chan2_render_midpoint);
        let chan3_samples = viz_chunks
            .iter()
            .flat_map(|vc| vc.channels[3].iter().copied())
            .rev()
            .skip(*viz_tail_len)
            .take(render_len);

        const INTRO_OUTRO_LEN: usize = 75;

        // Unfortunately for now we can only dynamically update a Path by constructing a string SVG command list.
        // https://github.com/slint-ui/slint/issues/754
        // With around 3-digits integer x and y coordinates, SVG commands will be on average around 8 chars each.
        let mut commands = String::with_capacity(INTRO_OUTRO_LEN + render_len * 8);
        let iters = chan0_samples.zip(chan1_samples).zip(chan2_samples).zip(chan3_samples);
        let mid_height = (height / 2.0).floor();
        let radius = (height / 8.0).floor();
        let wave_width = width - radius * 2.0;

        use std::fmt::Write;
        // Start on the right of the waveform, at mid-height to give room above and under for the waveform.
        write!(
            commands,
            "M{right_of_wave},{mid_height}",
            right_of_wave = width - radius
        )
        .unwrap();

        for (i, (((s0, s1), s2), s3)) in iters.enumerate() {
            // "Mix" all channels
            let source = s0 + s1 + s2 + s3;

            // Start from the right
            let normalized_pos = (render_len - 1 - i) as f32 / render_len as f32;
            let x = (radius + normalized_pos * wave_width) as u32;

            let side_factor = if normalized_pos < 0.05 {
                normalized_pos / 0.05
            } else if normalized_pos > 0.95 {
                (1.0 - normalized_pos) / 0.05
            } else {
                1.0
            };
            // Input samples are in the range [-1.0, 1.0].
            // The gameboy emulator mixer however just use a gain of 0.25
            // per channel to avoid clipping when all channels are playing.
            // So multiply by 1.5 to amplify the visualization of single
            // channels a bit.
            let y = ((source * 1.5 * side_factor + 1.0) * mid_height) as u32;
            write!(commands, "H{x}V{y}").unwrap();
        }
        // - Line to the left of the waveform (in case there were no samples)
        // - Left rounded corner
        write!(
            commands,
            " L{left_of_wave},{mid_height} A{radius},{radius} 0 0 0 0,{under_rounded_corner}",
            left_of_wave = radius,
            under_rounded_corner = mid_height + radius
        )
        .unwrap();
        // - Line down by mid-height past the bottom by an extra radius to avoid gaps
        // - Line up to the start of the right rounded corner
        // - Right rounded corner
        // - Close the counter-clockwise waveform shape (but we don't fill it anyway)
        write!(
            commands,
            " v{mid_height} H{width} v-{mid_height} a{radius},{radius} 0 0 0 -{radius},-{radius} z"
        )
        .unwrap();

        // The sound thread might buffer more than one frame (e.g. in WASM), so we have to advance our position
        // within the visualization samples that it produced so that the next frame shows where we expect the sound
        // buffer to have been sent to the speakers.
        let offset_advance = (sample_rate as f32 / 1000.0 * (tick - self.last_viz_chunk_tick)).round() as usize;
        *viz_tail_len = viz_tail_len.saturating_sub(offset_advance);
        self.last_viz_chunk_tick = tick;

        commands.into()
    }
}

fn check_if_project_changed(notify_recv: &mpsc::Receiver<DebouncedEvent>, engine: &mut SoundEngine) {
    #[cfg(not(target_arch = "wasm32"))]
    while let Ok(msg) = notify_recv.try_recv() {
        let instruments = engine.instruments_path().and_then(|ip| ip.canonicalize().ok());
        let reload = match msg {
            DebouncedEvent::Write(path) if path.canonicalize().ok() == instruments => true,
            DebouncedEvent::Create(path) if path.canonicalize().ok() == instruments => true,
            DebouncedEvent::Remove(path) if path.canonicalize().ok() == instruments => true,
            DebouncedEvent::Rename(from, to)
                if from.canonicalize().ok() == instruments || to.canonicalize().ok() == instruments =>
            {
                true
            }
            _ => false,
        };
        if reload {
            engine.reload_instruments_from_file();
        }
    }
}

pub fn new_sound_renderer(window: &MainWindow) -> SoundRenderer<impl FnOnce() -> Context> {
    let (sound_send, sound_recv) = mpsc::channel::<Box<dyn FnOnce(&mut SoundEngine) + Send>>();
    let (notify_send, notify_recv) = mpsc::channel();

    let cloned_sound_send = sound_send.clone();
    SOUND_SENDER.with(|s| *s.borrow_mut() = Some(cloned_sound_send));

    #[cfg(not(target_arch = "wasm32"))]
    let watcher = notify::watcher(notify_send, Duration::from_millis(500)).unwrap();
    let viz_chunks: Arc<Mutex<VecDeque<VizChunk>>> = Arc::new(Mutex::new(VecDeque::with_capacity(32)));
    let viz_tail_len: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let viz_chunks_s = viz_chunks.clone();
    let viz_tail_len_s = viz_tail_len.clone();

    let window_weak = WeakWindowWrapper::new(window.as_weak());
    let initial_settings = window.global::<GlobalSettings>().get_settings();
    let context: Rc<Lazy<Context, _>> = Rc::new(Lazy::new(|| {
        let host = cpal::default_host();
        let device = host.default_output_device().unwrap();
        log!("Open the audio player: {}", device.name().unwrap());
        let config = device.default_output_config().unwrap();
        log!("Audio format {:?}", config);
        let sample_rate = config.sample_rate().0;

        let err_fn = |err| elog!("an error occurred on the output audio stream: {}", err);
        let sample_format = config.sample_format();

        // The sequencer won't produce anything faster than every 1/60th second,
        // so a buffer roughly the size of a frame should work fine for now.
        #[cfg(not(target_arch = "wasm32"))]
        let wanted_buffer_size = 512;
        // Everything happens on the same thread in wasm32, and is a bit slower,
        // so increase the buffer size there.
        #[cfg(target_arch = "wasm32")]
        let wanted_buffer_size = 2048;

        let buffer_size = match config.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => wanted_buffer_size.min(*max).max(*min),
            cpal::SupportedBufferSize::Unknown => wanted_buffer_size,
        };

        let mut stream_config: cpal::StreamConfig = config.into();
        stream_config.buffer_size = cpal::BufferSize::Fixed(buffer_size);

        let stream = match sample_format {
            SampleFormat::F32 => device.build_output_stream(
                &stream_config,
                move |dest: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    SOUND_ENGINE.with(|maybe_engine_cell| {
                        let mut maybe_engine = maybe_engine_cell.borrow_mut();
                        if maybe_engine.is_none() {
                            let synth = Synth::new(window_weak.clone(), sample_rate, initial_settings.clone());
                            *maybe_engine = Some(SoundEngine::new(synth, window_weak.clone()));
                        }
                        let engine = maybe_engine.as_mut().unwrap();

                        check_if_project_changed(&notify_recv, engine);
                        // Process incoming messages from the main thread
                        while let Ok(closure) = sound_recv.try_recv() {
                            closure(engine);
                        }

                        // The instruments are loaded asynchronously in the web version,
                        // keep returning until they are loaded and that main() was executed.
                        if !engine.is_ready() {
                            return;
                        }

                        const NUM_CHANNELS: usize = 2;

                        let dest_len = dest.len();
                        let mut transfer_len = 0;
                        let synth_output_mutex = engine.synth.output_data();
                        let mut synth_output = synth_output_mutex.lock().unwrap();
                        while transfer_len < dest_len {
                            if synth_output.buffer.len() < (dest_len - transfer_len) {
                                let internal_buf_len_before = synth_output.buffer.len();
                                // Unlock before executing instruments and rendering the sound
                                drop(synth_output);
                                engine.advance_frame();
                                // Lock again to pick up the output
                                synth_output = synth_output_mutex.lock().unwrap();

                                let mut viz_chunks = viz_chunks_s.lock().unwrap();
                                if viz_chunks.len() == viz_chunks.capacity() {
                                    viz_chunks.pop_front();
                                }
                                let viz_chunk = synth_output.viz_chunk.take().unwrap();
                                debug_assert!(
                                    viz_chunk.channels[0].len()
                                        == (synth_output.buffer.len() - internal_buf_len_before) / 2
                                );
                                viz_chunks.push_back(viz_chunk);
                            }

                            let frame_transfer_len = std::cmp::min(dest_len - transfer_len, synth_output.buffer.len());
                            {
                                let part = synth_output.buffer.drain(..frame_transfer_len);
                                dest[transfer_len..transfer_len + frame_transfer_len].copy_from_slice(part.as_slice());
                            }

                            transfer_len += frame_transfer_len;
                        }

                        // Position the visualization where the sound buffer was before this callback.
                        // The rendering will then advance it frame by frame until the next callback.
                        *viz_tail_len_s.lock().unwrap() = (dest_len + synth_output.buffer.len()) / NUM_CHANNELS;
                    });
                },
                err_fn,
                None,
            ),
            _ => todo!(),
        }
        .unwrap();

        stream.play().unwrap();

        Context {
            sample_rate,
            _stream: stream,
        }
    }));

    SoundRenderer {
        sound_send,
        context,
        #[cfg(not(target_arch = "wasm32"))]
        watcher,
        #[cfg(not(target_arch = "wasm32"))]
        watched_path: None,
        viz_chunks,
        viz_tail_len,
        last_viz_chunk_tick: 0.0,
    }
}
