use crate::sequencer::NoteEvent;
use crate::sequencer::Sequencer;
use crate::synth::Synth;
use crate::MainWindow;
use sixtyfps::Model;
use sixtyfps::Weak;

pub struct SoundEngine {
    pub sequencer: Sequencer,
    pub synth: Synth,
    selected_instrument: u32,
    main_window: Weak<MainWindow>,
}

impl SoundEngine {
    pub fn new(sample_rate: u32, project_name: &str, main_window: Weak<MainWindow>) -> SoundEngine {
        SoundEngine {
                sequencer: Sequencer::new(project_name, main_window.clone()),
                synth: Synth::new(sample_rate),
                selected_instrument: 0,
                main_window: main_window,
            }
    }

    pub fn advance_frame(&mut self) -> () {
        let note_events = self.sequencer.advance_frame();
        for (instrument, typ, note) in note_events {
            if typ == NoteEvent::Press {
                self.synth.trigger_instrument(instrument, Self::note_to_freq(note));
            }
            if instrument == self.selected_instrument {
                self.main_window.clone().upgrade_in_event_loop(move |handle| {
                    let model = handle.get_notes();
                    for row in 0..model.row_count() {
                        let mut row_data = model.row_data(row);
                        if row_data.note_number as u32 == note {
                            row_data.active = typ == NoteEvent::Press;
                            model.set_row_data(row, row_data);
                        }
                    }
                });
            }
        }
        self.synth.advance_frame();
    }

    pub fn select_instrument(&mut self, instrument: u32) -> () {
        self.selected_instrument = instrument;
        self.sequencer.select_instrument(instrument);

        // Release all notes visually that might have been pressed for the previous instrument.
        self.main_window.clone().upgrade_in_event_loop(move |handle| {
            let model = handle.get_notes();
            for row in 0..model.row_count() {
                let mut row_data = model.row_data(row);
                row_data.active = false;
                model.set_row_data(row, row_data);
            }
        });
    }

    pub fn press_note(&mut self, note: u32) -> () {
        self.synth.trigger_instrument(self.selected_instrument, Self::note_to_freq(note));
        self.sequencer.record_trigger(self.selected_instrument, note);
    }

    pub fn save_project(&self, project_name: &str) {
        self.sequencer.save(project_name);
    }

    fn note_to_freq(note: u32) -> f64 {
        let a = 440.0; // Frequency of A
        let key_freq = (a / 32.0) * 2.0_f64.powf((note as f64 - 9.0) / 12.0);
        key_freq
    }
}

