use crate::sequencer::NoteEvent;
use crate::sequencer::Sequencer;
use crate::sixtyfps_generated_MainWindow::NoteData;
use crate::sixtyfps_generated_MainWindow::StepData;
use crate::sixtyfps_generated_MainWindow::MainWindow;
use crate::synth::SetSetting;
use crate::synth::Synth;
use sixtyfps::Model;
use sixtyfps::VecModel;
use sixtyfps::Weak;
use std::rc::Rc;

pub struct SoundStuff {
    pub sequencer: Sequencer,
    pub synth: Synth,
    selected_instrument: u32,
    visual_note_model: Rc<VecModel<NoteData>>,
}

impl SoundStuff {
    pub fn new(apu: gameboy::apu::Apu, window_weak: Weak<MainWindow>, sequencer_step_model: Rc<VecModel<StepData>>, note_model: Rc<VecModel<NoteData>>) -> SoundStuff {
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
                visual_note_model: note_model,
            }
    }

    pub fn advance_frame(&mut self) -> () {
        let note_events = self.sequencer.advance_frame();
        for (instrument, typ, note) in note_events {
            self.synth.trigger_instrument(instrument, Self::note_to_freq(note));
            for row in 0..self.visual_note_model.row_count() {
                let mut row_data = self.visual_note_model.row_data(row);
                if row_data.note_number as u32 == note {
                    row_data.active = typ == NoteEvent::Press;
                    self.visual_note_model.set_row_data(row, row_data);
                }
            }
        }
        self.synth.advance_frame();
    }

    pub fn select_instrument(&mut self, instrument: u32) -> () {
        self.selected_instrument = instrument;
    }

    pub fn trigger_selected_instrument(&mut self, note: u32) -> () {
        self.synth.trigger_instrument(self.selected_instrument, Self::note_to_freq(note));
        self.sequencer.record_trigger(self.selected_instrument, note);
    }

    fn note_to_freq(note: u32) -> f64 {
        let a = 440.0; //frequency of A (coomon value is 440Hz)
        let key_freq = (a / 32.0) * 2.0_f64.powf((note as f64 - 9.0) / 12.0);
        println!("NOTE {:?} {:?}", note, key_freq);
        key_freq
    }
}

