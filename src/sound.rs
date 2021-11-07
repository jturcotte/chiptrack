use crate::sequencer::NoteEvent;
use crate::sequencer::Sequencer;
use crate::sixtyfps_generated_MainWindow::NoteData;
use crate::sixtyfps_generated_MainWindow::SongPatternData;
use crate::sixtyfps_generated_MainWindow::PatternData;
use crate::sixtyfps_generated_MainWindow::StepData;
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
    pub fn new(apu: gameboy::apu::Apu, sequencer_song_model: Rc<VecModel<SongPatternData>>, sequencer_pattern_model: Rc<VecModel<PatternData>>, sequencer_step_model: Rc<VecModel<StepData>>, note_model: Rc<VecModel<NoteData>>) -> SoundStuff {
        SoundStuff {
                sequencer: Sequencer::new(sequencer_song_model, sequencer_pattern_model, sequencer_step_model),
                synth: Synth::new(apu),
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

