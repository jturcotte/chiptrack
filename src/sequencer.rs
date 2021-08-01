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
    step_instruments_note: [[u32; NUM_INSTRUMENTS]; NUM_STEPS],
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
            step_instruments_note: [[0; NUM_INSTRUMENTS]; NUM_STEPS],
            previous_frame_note_events: Vec::new(),
            visual_step_model: sequencer_step_model,
        }
    }

    pub fn set_current_step(&mut self, step_num: u32) -> () {
        let mut row_data = self.visual_step_model.row_data(self.current_step as usize);
        row_data.active = false;
        self.visual_step_model.set_row_data(self.current_step as usize, row_data);

        self.current_step = step_num;

        let mut row_data = self.visual_step_model.row_data(self.current_step as usize);
        row_data.active = true;
        self.visual_step_model.set_row_data(self.current_step as usize, row_data);
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
            let next_step = (self.current_step + 1) % (NUM_STEPS as u32);
            self.set_current_step(next_step);

            // Each note lasts only one frame, so just release everything pressed on the previous frame.
            for (instrument, typ, note) in &self.previous_frame_note_events {
                if *typ == NoteEvent::Press {
                    note_events.push((*instrument, NoteEvent::Release, *note));
                }
            }
            // FIXME: This assumes that the current instrument didn't change.
            for (i, note) in self.step_instruments_note[next_step as usize].iter().enumerate() {
                if *note != 0 {
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
        let mut row_data = self.visual_step_model.row_data(self.current_step as usize);
        row_data.empty = false;
        self.visual_step_model.set_row_data(self.current_step as usize, row_data);
    }
}
