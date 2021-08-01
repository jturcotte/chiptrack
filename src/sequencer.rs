use crate::sixtyfps_generated_MainWindow::StepData;
use sixtyfps::Model;
use sixtyfps::VecModel;
use std::rc::Rc;

pub const NUM_INSTRUMENTS: usize = 9;
pub const NUM_STEPS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoteEvent {
    Press,
    Release,
}

pub struct Sequencer {
    current_frame: u32,
    current_step: u32,
    playing: bool,
    recording: bool,
    selected_instrument: u32,
    step_instruments_note: [[u32; NUM_INSTRUMENTS]; NUM_STEPS],
    step_instruments_enabled: [[bool; NUM_INSTRUMENTS]; NUM_STEPS],
    previous_frame_note_events: Vec<(u32, NoteEvent, u32)>,
    visual_step_model: Rc<VecModel<StepData>>,
}

impl Sequencer {
    pub fn new(sequencer_step_model: Rc<VecModel<StepData>>) -> Sequencer {
        Sequencer {
            current_frame: 0,
            current_step: 0,
            playing: true,
            recording: true,
            selected_instrument: 0,
            // Initialize all notes to C5
            step_instruments_note: [[60; NUM_INSTRUMENTS]; NUM_STEPS],
            step_instruments_enabled: [[false; NUM_INSTRUMENTS]; NUM_STEPS],
            previous_frame_note_events: Vec::new(),
            visual_step_model: sequencer_step_model,
        }
    }

    pub fn select_instrument(&mut self, instrument: u32) -> () {
        self.selected_instrument = instrument;

        for i in 0..NUM_STEPS {
            let step_enabled = self.step_instruments_enabled[i][instrument as usize];
            let mut row_data = self.visual_step_model.row_data(i);
            row_data.empty = !step_enabled;
            self.visual_step_model.set_row_data(i, row_data);
        }
    }

    pub fn toggle_step(&mut self, step_num: u32) -> () {
        let toggled = !self.step_instruments_enabled[step_num as usize][self.selected_instrument as usize];

        let mut row_data = self.visual_step_model.row_data(step_num as usize);
        self.step_instruments_enabled[step_num as usize][self.selected_instrument as usize] = toggled;
        row_data.empty = !toggled;
        self.visual_step_model.set_row_data(step_num as usize, row_data);
    }
    pub fn set_playing(&mut self, val: bool) -> () {
        self.playing = val;
    }
    pub fn set_recording(&mut self, val: bool) -> () {
        self.recording = val;
    }
    pub fn advance_frame(&mut self) -> Vec<(u32, NoteEvent, u32)> {
        let mut note_events: Vec<(u32, NoteEvent, u32)> = Vec::new();

        if !self.playing {
            return note_events;
        }

        self.current_frame += 1;
        if self.current_frame % 6 == 0 {
            let mut row_data = self.visual_step_model.row_data(self.current_step as usize);
            row_data.active = false;
            self.visual_step_model.set_row_data(self.current_step as usize, row_data);

            let next_step = (self.current_step + 1) % (NUM_STEPS as u32);
            self.current_step = next_step;

            let mut row_data = self.visual_step_model.row_data(self.current_step as usize);
            row_data.active = true;
            self.visual_step_model.set_row_data(self.current_step as usize, row_data);

            // Each note lasts only one frame, so just release everything pressed on the previous frame.
            for (instrument, typ, note) in &self.previous_frame_note_events {
                if *typ == NoteEvent::Press {
                    note_events.push((*instrument, NoteEvent::Release, *note));
                }
            }
            // FIXME: This assumes that the current instrument didn't change.
            for (i, note) in self.step_instruments_note[next_step as usize].iter().enumerate() {                if self.step_instruments_enabled[self.current_step as usize][i] {
                    println!("Instrument {:?} note {:?}", i, note);
                    note_events.push((i as u32, NoteEvent::Press, *note));
                }
            }
            self.previous_frame_note_events = note_events.clone();
        }
        return note_events;
    }

    pub fn record_trigger(&mut self, instrument: u32, note: u32) {
        if !self.recording {
            return;
        }

        self.step_instruments_note[self.current_step as usize][instrument as usize] = note;
        self.step_instruments_enabled[self.current_step as usize][instrument as usize] = true;

        let mut row_data = self.visual_step_model.row_data(self.current_step as usize);
        row_data.empty = false;
        self.visual_step_model.set_row_data(self.current_step as usize, row_data);
    }
}
