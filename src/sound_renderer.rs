// Copyright © 2023 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

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

thread_local! {static SOUND_ENGINE: RefCell<Option<SoundEngine>> = RefCell::new(None);}
thread_local! {static SOUND_SENDER: RefCell<Option<std::sync::mpsc::Sender<Box<dyn FnOnce(&mut SoundEngine) + Send>>>> = RefCell::new(None);}

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

pub struct CpalSoundRenderer<LazyF: FnOnce() -> Context> {
    sound_send: Sender<Box<dyn FnOnce(&mut SoundEngine) + Send>>,
    context: Rc<Lazy<Context, LazyF>>,
}

impl<LazyF: FnOnce() -> Context> CpalSoundRenderer<LazyF> {
    pub fn invoke_on_sound_engine<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SoundEngine) + Send + 'static,
    {
        Lazy::force(&*self.context);
        self.sound_send.send(Box::new(f)).unwrap();
    }

    pub fn sender(&self) -> Sender<Box<dyn FnOnce(&mut SoundEngine) + Send>> {
        self.sound_send.clone()
    }

    pub fn force(&mut self) {
        Lazy::force(&*self.context);
    }
}

fn check_if_project_changed(notify_recv: &mpsc::Receiver<DebouncedEvent>, engine: &mut SoundEngine) -> () {
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
pub fn new_cpal_sound_renderer(window: &MainWindow) -> CpalSoundRenderer<impl FnOnce() -> Context> {
    let (sound_send, sound_recv) = mpsc::channel::<Box<dyn FnOnce(&mut SoundEngine) + Send>>();
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
                            *maybe_engine = Some(SoundEngine::new(
                                sample_rate,
                                window_weak.clone(),
                                initial_settings.clone(),
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

    CpalSoundRenderer{sound_send, context}

}

