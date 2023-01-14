// Copyright © 2023 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

pub mod emulated;

use crate::synth_script::RegSettings;
use crate::Settings;
use crate::MainWindow;
use crate::sound_engine::SoundEngine;

use slint::{ComponentHandle};

use std::cell::RefCell;
use std::rc::Rc;

use std::sync::mpsc;
use std::sync::mpsc::Sender;

pub trait Synth {
    // FIXME: It's not really advancing here, more like applying
    // GameBoy games seem to use the main loop clocked to the screen's refresh rate
    // to also drive the sound chip. To keep the song timing, also use the same 59.73hz
    // frame refresh rate.
    fn advance_frame(&mut self, frame_number: usize, settings: &mut RegSettings, step_change: Option<u32>);

    fn apply_settings(&mut self, settings: &Settings);

    /// Can be used to manually mute when instruments have an infinite length and envelope.
    fn mute_instruments(&mut self);
}

pub struct PrintRegistersSynth {
}

impl Synth for PrintRegistersSynth {
    fn advance_frame(&mut self, frame_number: usize, settings: &mut RegSettings, _step_change: Option<u32>) {
        settings.for_each_setting(|addr, set| {
            log!("{} - {:#x}: {:#04x} ({:#010b})", frame_number, addr, set.value, set.value);
        });
        settings.clear_all();
    }

    fn apply_settings(&mut self, _settings: &Settings) {
    }

    fn mute_instruments(&mut self) {
    }
}

pub trait SoundRenderer<SynthType: Synth> {
    fn invoke_on_sound_engine<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SoundEngine<SynthType>) + Send + 'static;

    fn sender(&self) -> Sender<Box<dyn FnOnce(&mut SoundEngine<SynthType>) + Send>>;
    fn force(&mut self);
}

pub struct PrintRegistersSoundRenderer {
    sound_engine: Rc<RefCell<SoundEngine<PrintRegistersSynth>>>,
    _timer: slint::Timer,
}

impl PrintRegistersSoundRenderer {
    pub fn new(window: &MainWindow) -> PrintRegistersSoundRenderer {
        let synth = PrintRegistersSynth{};
        let sound_engine = Rc::new(RefCell::new(SoundEngine::new(synth, window.as_weak())));

        let timer = slint::Timer::default();
        let cloned_sound_engine = sound_engine.clone();
        timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(16), move || {
           cloned_sound_engine.borrow_mut().advance_frame();
        });

        PrintRegistersSoundRenderer{sound_engine, _timer: timer}
    }
}

impl SoundRenderer<PrintRegistersSynth> for PrintRegistersSoundRenderer {
    fn invoke_on_sound_engine<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SoundEngine<PrintRegistersSynth>) + Send + 'static,
    {
        f(&mut self.sound_engine.borrow_mut())
    }

    fn sender(&self) -> Sender<Box<dyn FnOnce(&mut SoundEngine<PrintRegistersSynth>) + Send>> {
        // FIXME
        let (sender, _receiver) = mpsc::channel();
        sender
    }

    fn force(&mut self) {
    }
}
