// Copyright © 2023 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::sound_renderer::Synth;
use crate::SoundRenderer;
use crate::GlobalSettings;
use crate::MainWindow;
use crate::sound_engine::SoundEngine;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat};
use notify::{DebouncedEvent, RecursiveMode, Watcher};
use once_cell::unsync::Lazy;
use slint::{ComponentHandle, Rgba8Pixel, SharedPixelBuffer};
use tiny_skia::*;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

thread_local! {static SOUND_ENGINE: RefCell<Option<SoundEngine<RboySynth>>> = RefCell::new(None);}
thread_local! {static SOUND_SENDER: RefCell<Option<std::sync::mpsc::Sender<Box<dyn FnOnce(&mut SoundEngine<RboySynth>) + Send>>>> = RefCell::new(None);}

use crate::synth_script::Channel;
use crate::synth_script::RegSettings;

use crate::utils::MidiNote;
use crate::ChannelActiveNote;
use crate::ChannelTraceNote;
use crate::GlobalEngine;
use crate::Settings;

use slint::Global;
use slint::Model;
use slint::VecModel;
use slint::Weak;

use std::sync::Mutex;

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

pub struct RboySynth {
    dmg: rboy::Sound,
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

impl Synth for RboySynth {
    fn advance_frame(&mut self, frame_number: usize, settings: &mut RegSettings, step_change: Option<u32>) {

        {
            let dmg = &mut self.dmg;
            settings.for_each_setting(|addr, set| {
                // Trying to read the memory wouldn't give us the value we wrote last,
                // so overwrite any state previously set in bits outside of RegSetter.mask
                // with zeros.
                dmg.wb(addr, set.value);
            });
            settings.clear_all();
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

        self.update_ui_channel_states(frame_number as i32);
    }

    fn apply_settings(&mut self, settings: &Settings) {
        let mut output_data = self.output_data.lock().unwrap();
        output_data.gain = if settings.sync_enabled { SYNC_GAIN } else { 1.0 };
        output_data.sync_pulse.enabled = settings.sync_enabled;
    }

    fn mute_instruments(&mut self) {
        // Set the envelopes to 0.
        self.dmg.wb(Channel::Square1 as u16 + 2, 0);
        self.dmg.wb(Channel::Square2 as u16 + 2, 0);
        self.dmg.wb(Channel::Wave as u16 + 2, 0);
        self.dmg.wb(Channel::Noise as u16 + 2, 0);
    }
}

impl RboySynth {
    pub fn new(main_window: Weak<MainWindow>, sample_rate: u32, settings: Settings) -> RboySynth {
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

        RboySynth {
            dmg: dmg,
            output_data: output_data,
            main_window: main_window,
        }
    }

    pub fn output_data(&self) -> Arc<Mutex<OutputData>> {
        self.output_data.clone()
    }

    fn update_ui_channel_states(&self, frame_number: i32) {
        let mut states = self.dmg.chan_states();
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
                                    Some(trace_vec_model.set_row_data(last_index, last))
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_else(|| trace_vec_model.push(trace));

                        active_vec_model.push(active);
                    }
                }
                global.set_synth_tick(frame_number);
            })
            .unwrap();
    }
}

pub fn invoke_on_sound_engine<F>(f: F)
where
    F: FnOnce(&mut SoundEngine<RboySynth>) + Send + 'static,
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

pub struct EmulatedSoundRenderer<LazyF: FnOnce() -> Context> {
    sound_send: Sender<Box<dyn FnOnce(&mut SoundEngine<RboySynth>) + Send>>,
    context: Rc<Lazy<Context, LazyF>>,
}

impl<LazyF: FnOnce() -> Context> SoundRenderer<RboySynth> for EmulatedSoundRenderer<LazyF> {
    fn invoke_on_sound_engine<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SoundEngine<RboySynth>) + Send + 'static,
    {
        Lazy::force(&*self.context);
        self.sound_send.send(Box::new(f)).unwrap();
    }

    fn sender(&self) -> Sender<Box<dyn FnOnce(&mut SoundEngine<RboySynth>) + Send>> {
        self.sound_send.clone()
    }

    fn force(&mut self) {
        Lazy::force(&*self.context);
    }
}

fn check_if_project_changed(notify_recv: &mpsc::Receiver<DebouncedEvent>, engine: &mut SoundEngine<RboySynth>) -> () {
    while let Ok(msg) = notify_recv.try_recv() {
        let reload = if let Some(instruments_path) = engine.instruments_path() {
            let instruments = instruments_path.file_name();
            match msg {
                DebouncedEvent::Write(path) if path.file_name() == instruments => true,
                DebouncedEvent::Create(path) if path.file_name() == instruments => true,
                DebouncedEvent::Remove(path) if path.file_name() == instruments => true,
                DebouncedEvent::Rename(from, to)
                    if from.file_name() == instruments || to.file_name() == instruments =>
                {
                    true
                }
                _ => false,
            }
        } else {
            false
        };
        if reload {
            engine.reload_instruments_from_file();
        }
    }
}

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
pub fn new_emulated_sound_renderer(window: &MainWindow) -> EmulatedSoundRenderer<impl FnOnce() -> Context> {
    let (sound_send, sound_recv) = mpsc::channel::<Box<dyn FnOnce(&mut SoundEngine<RboySynth>) + Send>>();
    let (notify_send, notify_recv) = mpsc::channel();

    let cloned_sound_send = sound_send.clone();
    SOUND_SENDER.with(|s| *s.borrow_mut() = Some(cloned_sound_send));

    #[cfg(not(target_arch = "wasm32"))]
    let mut watcher = notify::watcher(notify_send, Duration::from_millis(500)).unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    // FIXME: Watch the song file's folder, and update it when saving as.
    watcher.watch(".", RecursiveMode::NonRecursive).unwrap();

    let window_weak = window.as_weak();
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
                        if let None = *maybe_engine {
                            let synth = RboySynth::new(window_weak.clone(), sample_rate, initial_settings.clone());
                            *maybe_engine = Some(SoundEngine::new(
                                synth,
                                window_weak.clone(),
                            ));
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
                                    let buffer_viz = std::mem::replace(&mut synth_output.buffer_viz, Vec::new());
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

        Context {
            _stream: stream,
        }
    }));

    EmulatedSoundRenderer{sound_send, context}

}

