use crate::sequencer::NoteEvent;
use crate::sequencer::Sequencer;
use crate::sixtyfps_generated_MainWindow::NoteData;
use crate::sixtyfps_generated_MainWindow::PatternData;
use crate::sixtyfps_generated_MainWindow::StepData;
use crate::synth::SetSetting;
use crate::synth::Channel::*;
use crate::synth::Synth;
use sixtyfps::Model;
use sixtyfps::VecModel;
use std::rc::Rc;

pub struct SoundStuff {
    pub sequencer: Sequencer,
    pub synth: Synth,
    selected_instrument: u32,
    visual_note_model: Rc<VecModel<NoteData>>,
}

impl SoundStuff {
    pub fn new(apu: gameboy::apu::Apu, sequencer_pattern_model: Rc<VecModel<PatternData>>, sequencer_step_model: Rc<VecModel<StepData>>, note_model: Rc<VecModel<NoteData>>) -> SoundStuff {
        let instruments: Vec<Box<dyn Fn(&mut Vec<Vec<SetSetting>>, usize, u32) -> ()>> =
            vec!(
                Box::new(move |settings_ring, i, freq| {
                    let len = settings_ring.len();
                    let f = |frame: usize| { (i + frame) % len };
                    settings_ring[f(0)].push(SetSetting::duty(0x2, Square1));
                    settings_ring[f(0)].push(SetSetting::envelope(0xf, 0x0, 0x0, Square1));
                    settings_ring[f(0)].push(SetSetting::sweep(0x0, 0x0, 0x0));
                    settings_ring[f(0)].extend(SetSetting::trigger_with_length(freq, 64, Square1));
                }),
                Box::new(move |settings_ring, i, freq| {
                    let len = settings_ring.len();
                    let f = |frame: usize| { (i + frame) % len };
                    // 2:bipp e:a:d:1 f:0:d:2 g
                    settings_ring[f(0)].push(SetSetting::duty(0x2, Square1));
                    settings_ring[f(0)].push(SetSetting::envelope(0xa, 0x0, 0x1, Square1));
                    settings_ring[f(0)].push(SetSetting::sweep(0x0, 0x1, 0x2));
                    settings_ring[f(0)].extend(SetSetting::trigger(freq, Square1));
                    // e:0:D:0 g e
                    settings_ring[f(2)].push(SetSetting::envelope(0x0, 0x0, 0x0, Square1));
                    settings_ring[f(2)].extend(SetSetting::trigger(freq, Square1));
                }),
                Box::new(move |settings_ring, i, freq| {
                    let len = settings_ring.len();
                    let f = |frame: usize| { (i + frame) % len };
                    // r:1 e:f:d:0 f:0:d:0 g
                    settings_ring[f(0)].push(SetSetting::duty(0x1, Square1));
                    settings_ring[f(0)].push(SetSetting::envelope(0xf, 0x0, 0x0, Square1));
                    settings_ring[f(0)].push(SetSetting::sweep(0x0, 0x1, 0x0));
                    settings_ring[f(0)].extend(SetSetting::trigger(freq, Square1));
                    // r:3
                    settings_ring[f(2)].push(SetSetting::duty(0x1, Square1));
                    // r:0
                    settings_ring[f(4)].push(SetSetting::duty(0x0, Square1));
                    // r:3
                    settings_ring[f(6)].push(SetSetting::duty(0x3, Square1));
                    // r:1
                    settings_ring[f(8)].push(SetSetting::duty(0x1, Square1));
                    // r:3
                    settings_ring[f(10)].push(SetSetting::duty(0x3, Square1));
                    // r:1
                    settings_ring[f(12)].push(SetSetting::duty(0x1, Square1));
                    // r:3
                    settings_ring[f(14)].push(SetSetting::duty(0x3, Square1));
                    // r:0
                    settings_ring[f(16)].push(SetSetting::duty(0x0, Square1));
                    // e:0:d:0 g
                    settings_ring[f(18)].push(SetSetting::envelope(0x0, 0x0, 0x0, Square1));
                    settings_ring[f(18)].extend(SetSetting::trigger(freq, Square1));
                }),
                Box::new(move |settings_ring, i, freq| {
                    let len = settings_ring.len();
                    let f = |frame: usize| { (i + frame) % len };
                    // 1:superdrum e:d:d:2 f:2:d:2 g e
                    settings_ring[f(0)].push(SetSetting::duty(0x2, Square1));
                    settings_ring[f(0)].push(SetSetting::envelope(0xd, 0x0, 0x2, Square1));
                    settings_ring[f(0)].push(SetSetting::sweep(0x2, 0x1, 0x2));
                    settings_ring[f(0)].extend(SetSetting::trigger(freq, Square1));
                }),
                Box::new(move |settings_ring, i, freq| {
                    let len = settings_ring.len();
                    let f = |frame: usize| { (i + frame) % len };
                    settings_ring[f(0)].push(SetSetting::sweep(0x0, 0x0, 0x0));
                    settings_ring[f(0)].push(SetSetting::duty(0x2, Square1));
                    settings_ring[f(0)].push(SetSetting::envelope(0xf, 0x1, 0x0, Square1));
                    settings_ring[f(0)].extend(SetSetting::trigger(freq, Square1));
                    settings_ring[f(1)].push(SetSetting::envelope(0xd, 0x1, 0x0, Square1));
                    settings_ring[f(1)].extend(SetSetting::trigger(freq, Square1));
                    settings_ring[f(5)].push(SetSetting::envelope(0x6, 0x1, 0x0, Square1));
                    settings_ring[f(5)].extend(SetSetting::trigger(freq, Square1));
                    settings_ring[f(7)].push(SetSetting::envelope(0x1, 0x1, 0x0, Square1));
                    settings_ring[f(7)].extend(SetSetting::trigger(freq, Square1));
                    settings_ring[f(8)].push(SetSetting::envelope(0x0, 0x1, 0x0, Square1));
                    settings_ring[f(8)].extend(SetSetting::trigger(freq, Square1));
                }),
                Box::new(move |settings_ring, i, freq| {
                    let len = settings_ring.len();
                    let f = |frame: usize| { (i + frame) % len };
                    settings_ring[f(0)].push(SetSetting::wave_power(0));
                    settings_ring[f(0)].extend(SetSetting::wave_table("2266aaeefffffeeaa6668acffeeca633"));
                    settings_ring[f(0)].push(SetSetting::wave_power(1));
                    settings_ring[f(0)].push(SetSetting::wave_volume_code(1));
                    settings_ring[f(0)].extend(SetSetting::trigger_with_length(freq, 64, Wave));
                }),
                Box::new(move |settings_ring, i, freq| {
                    let len = settings_ring.len();
                    let f = |frame: usize| { (i + frame) % len };
                    settings_ring[f(0)].push(SetSetting::envelope(0xf, 0x0, 0x1, Noise));
                    // Use the frequency as input for now just so that different
                    // keys produce different sound.
                    settings_ring[f(0)].push(SetSetting::noise_params(freq as u8 >> 4, 0x0, freq as u8 & 0x7));
                    settings_ring[f(0)].push(SetSetting::noise_trigger());
                }),
            );

        SoundStuff {
                sequencer: Sequencer::new(sequencer_pattern_model, sequencer_step_model),
                synth: Synth::new(apu, instruments),
                selected_instrument: 0,
                visual_note_model: note_model,
            }
    }

    pub fn advance_frame(&mut self) -> () {
        let note_events = self.sequencer.advance_frame();
        for (instrument, typ, note) in note_events {
            if typ == NoteEvent::Press {
                self.synth.trigger_instrument(instrument, Self::note_to_freq(note));
            }
            if instrument == self.selected_instrument {
                for row in 0..self.visual_note_model.row_count() {
                    let mut row_data = self.visual_note_model.row_data(row);
                    if row_data.note_number as u32 == note {
                        row_data.active = typ == NoteEvent::Press;
                        self.visual_note_model.set_row_data(row, row_data);
                    }
                }
            }
        }
        self.synth.advance_frame();
    }

    pub fn select_instrument(&mut self, instrument: u32) -> () {
        self.selected_instrument = instrument;
        self.sequencer.select_instrument(instrument);

        // Release all notes visually that might have been pressed for the previous instrument.
        for row in 0..self.visual_note_model.row_count() {
            let mut row_data = self.visual_note_model.row_data(row);
            row_data.active = false;
            self.visual_note_model.set_row_data(row, row_data);
        }
    }

    pub fn press_note(&mut self, note: u32) -> () {
        self.synth.trigger_instrument(self.selected_instrument, Self::note_to_freq(note));
        self.sequencer.record_trigger(self.selected_instrument, note);

        for row in 0..self.visual_note_model.row_count() {
            let mut row_data = self.visual_note_model.row_data(row);
            if row_data.note_number as u32 == note {
                row_data.active = true;
                self.visual_note_model.set_row_data(row, row_data);
            }
        }
    }

    pub fn release_notes(&mut self) -> () {
        // We have only one timer for direct interactions, and we don't handle
        // keys being held or even multiple keys at time yet, so just visually release all notes.
        for row in 0..self.visual_note_model.row_count() {
            let mut row_data = self.visual_note_model.row_data(row);
            if row_data.active {
                row_data.active = false;
                self.visual_note_model.set_row_data(row, row_data);
            }
        }
    }

    fn note_to_freq(note: u32) -> f64 {
        let a = 440.0; //frequency of A (coomon value is 440Hz)
        let key_freq = (a / 32.0) * 2.0_f64.powf((note as f64 - 9.0) / 12.0);
        println!("NOTE {:?} {:?}", note, key_freq);
        key_freq
    }
}

