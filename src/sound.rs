use crate::sequencer::Sequencer;
use crate::sixtyfps_generated_MainWindow::StepData;
use crate::sixtyfps_generated_MainWindow::MainWindow;
use crate::synth::SetSetting;
use crate::synth::Synth;
use sixtyfps::VecModel;
use sixtyfps::Weak;
use std::rc::Rc;

pub struct SoundStuff {
    pub sequencer: Sequencer,
    pub synth: Synth,
    selected_instrument: usize,
}

impl SoundStuff {
    pub fn new(apu: gameboy::apu::Apu, window_weak: Weak<MainWindow>, sequencer_step_model: Rc<VecModel<StepData>>) -> SoundStuff {
        let instruments: Vec<Box<dyn Fn(&mut Vec<Vec<SetSetting>>, usize, u32) -> ()>> =
            vec!(
                Box::new(move |settings_ring, i, freq| {
                    let len = settings_ring.len();
                    let f = |frame: usize| { (i + frame) % len };
                    settings_ring[f(0)].push(SetSetting::duty(0x2));
                    settings_ring[f(0)].push(SetSetting::envelope(0xf, 0x0, 0x0));
                    settings_ring[f(0)].push(SetSetting::sweep(0x0, 0x0, 0x0));
                    settings_ring[f(0)].extend(SetSetting::trigger_with_length(freq, 64));
                }),
                Box::new(move |settings_ring, i, freq| {
                    let len = settings_ring.len();
                    let f = |frame: usize| { (i + frame) % len };
                    // 2:bipp e:a:d:1 f:0:d:2 g
                    settings_ring[f(0)].push(SetSetting::duty(0x0));
                    settings_ring[f(0)].push(SetSetting::envelope(0xa, 0x0, 0x1));
                    settings_ring[f(0)].push(SetSetting::sweep(0x0, 0x1, 0x2));
                    settings_ring[f(0)].extend(SetSetting::trigger(freq));
                    // e:0:D:0 g e
                    settings_ring[f(2)].push(SetSetting::envelope(0x0, 0x0, 0x0));
                    settings_ring[f(2)].extend(SetSetting::trigger(freq));
                }),
                Box::new(move |settings_ring, i, freq| {
                    let len = settings_ring.len();
                    let f = |frame: usize| { (i + frame) % len };
                    // r:1 e:f:d:0 f:0:d:0 g
                    settings_ring[f(0)].push(SetSetting::duty(0x1));
                    settings_ring[f(0)].push(SetSetting::envelope(0xf, 0x0, 0x0));
                    settings_ring[f(0)].push(SetSetting::sweep(0x0, 0x1, 0x0));
                    settings_ring[f(0)].extend(SetSetting::trigger(freq));
                    // r:3
                    settings_ring[f(2)].push(SetSetting::duty(0x1));
                    // r:0
                    settings_ring[f(4)].push(SetSetting::duty(0x0));
                    // r:3
                    settings_ring[f(6)].push(SetSetting::duty(0x3));
                    // r:1
                    settings_ring[f(8)].push(SetSetting::duty(0x1));
                    // r:3
                    settings_ring[f(10)].push(SetSetting::duty(0x3));
                    // r:1
                    settings_ring[f(12)].push(SetSetting::duty(0x1));
                    // r:3
                    settings_ring[f(14)].push(SetSetting::duty(0x3));
                    // r:0
                    settings_ring[f(16)].push(SetSetting::duty(0x0));
                    // e:0:d:0 g
                    settings_ring[f(18)].push(SetSetting::envelope(0x0, 0x0, 0x0));
                    settings_ring[f(18)].extend(SetSetting::trigger(freq));
                }),
                Box::new(move |settings_ring, i, freq| {
                    let len = settings_ring.len();
                    let f = |frame: usize| { (i + frame) % len };
                    // 1:superdrum e:d:d:2 f:2:d:2 g e
                    settings_ring[f(0)].push(SetSetting::duty(0x0));
                    settings_ring[f(0)].push(SetSetting::envelope(0xd, 0x0, 0x2));
                    settings_ring[f(0)].push(SetSetting::sweep(0x2, 0x1, 0x2));
                    settings_ring[f(0)].extend(SetSetting::trigger(freq));
                }),
            );

        let synth = Synth {
                apu: apu,
                settings_ring: vec![vec![]; 512],
                settings_ring_index: 0,
                instruments: instruments,
            };
        SoundStuff {
                sequencer: Sequencer::new(window_weak, sequencer_step_model),
                synth: synth,
                selected_instrument: 0,
            }
    }

    pub fn advance_frame(&mut self) -> () {
        self.sequencer.advance_frame(&mut self.synth);
        self.synth.advance_frame();
    }

    pub fn select_instrument(&mut self, instrument: usize) -> () {
        self.selected_instrument = instrument;
    }

    pub fn trigger_selected_instrument(&mut self, freq: u32) -> () {
        self.synth.trigger_instrument(self.selected_instrument, freq);
        self.sequencer.record_trigger(self.selected_instrument, freq);
    }
}

