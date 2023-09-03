// Copyright © 2023 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::sound_engine::SoundEngine;
use crate::utils::MidiNote;
use crate::utils::WeakWindowWrapper;
use crate::ChannelActiveNote;
use crate::ChannelTraceNote;
use crate::GlobalEngine;
use crate::GlobalSettings;
use crate::MainWindow;
use crate::Settings;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat};
use notify::{DebouncedEvent, RecursiveMode, Watcher};
use once_cell::unsync::Lazy;
use slint::{ComponentHandle, Global, Model, Rgba8Pixel, SharedPixelBuffer, VecModel};
use tiny_skia::*;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

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
    pub buffer_viz: Vec<f32>,
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
    fn play(&mut self, left_channel: &[f32], right_channel: &[f32], viz_channel: &[f32]) {
        let left_iter = left_channel.iter();
        let mut right_iter = right_channel.iter();
        let mut state = self.state.lock().unwrap();
        let gain = state.gain;
        state.buffer_viz.clear();
        state.buffer_viz.extend_from_slice(viz_channel);

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
            buffer_viz: Vec::new(),
            gain,
            sync_pulse: SyncPulse::new(settings.sync_enabled, sample_rate, 300, 2),
        }));

        let player = Box::new(FakePlayer {
            sample_rate,
            state: output_data.clone(),
        });
        let mut dmg = rboy::Sound::new(player);
        // Already power it on.
        dmg.wb(0xff26, 0x80);

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
            // Just enable all channels for now
            dmg.wb(0xff24, 0xff);
            dmg.wb(0xff25, 0xff);

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
            // FIXME: Set playing off and then on
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
    _stream: cpal::Stream,
}

pub struct SoundRenderer<LazyF: FnOnce() -> Context> {
    sound_send: Sender<Box<dyn FnOnce(&mut SoundEngine) + Send>>,
    context: Rc<Lazy<Context, LazyF>>,
    #[cfg(not(target_arch = "wasm32"))]
    watcher: notify::RecommendedWatcher,
    #[cfg(not(target_arch = "wasm32"))]
    watched_path: Option<PathBuf>,
}

impl<LazyF: FnOnce() -> Context> SoundRenderer<LazyF> {
    pub fn invoke_on_sound_engine<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SoundEngine) + Send + 'static,
    {
        Lazy::force(&*self.context);
        self.sound_send.send(Box::new(f)).unwrap();
    }

    pub fn invoke_on_sound_engine_no_force<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SoundEngine) + Send + 'static,
    {
        self.sound_send.send(Box::new(f)).unwrap();
    }

    pub fn sender(&self) -> Sender<Box<dyn FnOnce(&mut SoundEngine) + Send>> {
        self.sound_send.clone()
    }

    pub fn force(&mut self) {
        Lazy::force(&*self.context);
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

#[cfg(feature = "desktop")]
fn update_waveform(window: &MainWindow, samples: Vec<f32>, consumed: Arc<AtomicBool>) {
    let was_non_zero = !window.get_waveform_is_zero();
    let res_divider = 2.;

    // Already let the audio thread know that it can send us a new waveform.
    consumed.store(true, Ordering::Relaxed);

    let width = window.get_waveform_width() / res_divider;
    let height = window.get_waveform_height() / res_divider;
    let mut pb = PathBuilder::new();
    let mut non_zero = false;
    {
        for (i, source) in samples.iter().enumerate() {
            if *source != 0.0 {
                non_zero = true;
            }
            // Input samples are in the range [-1.0, 1.0].
            // The gameboy emulator mixer however just use a gain of 0.25
            // per channel to avoid clipping when all channels are playing.
            // So multiply by 2.0 to amplify the visualization of single
            // channels a bit.
            let x = i as f32 * width / samples.len() as f32;
            let y = (source * 2.0 + 1.0) * height / 2.0;
            if i == 0 {
                pb.move_to(x, y);
            } else {
                pb.line_to(x, y);
            }
        }
    }
    // Painting this takes a lot of CPU since we need to paint, clone
    // the whole pixmap buffer, and changing the image will trigger a
    // repaint of the full viewport.
    // So at least avoig eating CPU while no sound is being output.
    if non_zero || was_non_zero {
        if let Some(path) = pb.finish() {
            let mut pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::new(width as u32, height as u32);
            if let Some(mut pixmap) = PixmapMut::from_bytes(pixel_buffer.make_mut_bytes(), width as u32, height as u32)
            {
                pixmap.fill(tiny_skia::Color::TRANSPARENT);
                let mut paint = Paint::default();
                paint.blend_mode = BlendMode::Source;
                // #a0a0a0
                paint.set_color_rgba8(160, 160, 160, 255);

                let mut stroke = Stroke::default();
                // Use hairline stroking, faster.
                stroke.width = 0.0;
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);

                let image = slint::Image::from_rgba8_premultiplied(pixel_buffer);
                window.set_waveform_image(image);
                window.set_waveform_is_zero(!non_zero);
            }
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

        let last_waveform_consumed = Arc::new(AtomicBool::new(true));

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

                        let len = dest.len();
                        let mut di = 0;
                        while di < len {
                            let synth_output_mutex = engine.synth.output_data();
                            let mut synth_output = synth_output_mutex.lock().unwrap();
                            if synth_output.buffer.len() < (len - di) {
                                drop(synth_output);
                                engine.advance_frame();
                                synth_output = synth_output_mutex.lock().unwrap();

                                if last_waveform_consumed.load(Ordering::Relaxed) {
                                    let buffer_viz = std::mem::take(&mut synth_output.buffer_viz);
                                    last_waveform_consumed.store(false, Ordering::Relaxed);
                                    let consumed_clone = last_waveform_consumed.clone();
                                    window_weak
                                        .upgrade_in_event_loop(move |handle| {
                                            update_waveform(&handle, buffer_viz, consumed_clone)
                                        })
                                        .unwrap();
                                }
                            }

                            let src_len = std::cmp::min(len - di, synth_output.buffer.len());
                            let part = synth_output.buffer.drain(..src_len);
                            dest[di..di + src_len].copy_from_slice(part.as_slice());

                            di += src_len;
                        }
                    });
                },
                err_fn,
            ),
            // FIXME
            SampleFormat::I16 => device.build_output_stream(&stream_config, write_silence::<i16>, err_fn),
            SampleFormat::U16 => device.build_output_stream(&stream_config, write_silence::<u16>, err_fn),
        }
        .unwrap();

        fn write_silence<T: Sample>(data: &mut [T], _: &cpal::OutputCallbackInfo) {
            for sample in data.iter_mut() {
                *sample = Sample::from(&0.0);
            }
        }

        stream.play().unwrap();

        Context { _stream: stream }
    }));

    SoundRenderer {
        sound_send,
        context,
        #[cfg(not(target_arch = "wasm32"))]
        watcher,
        watched_path: None,
    }
}
