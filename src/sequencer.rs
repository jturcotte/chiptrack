use crate::sixtyfps_generated_MainWindow::SongPatternData;
use crate::sixtyfps_generated_MainWindow::PatternData;
use crate::sixtyfps_generated_MainWindow::StepData;
use sixtyfps::Model;
use sixtyfps::VecModel;
use std::rc::Rc;

pub const NUM_INSTRUMENTS: usize = 9;
pub const NUM_STEPS: usize = 16;
pub const NUM_PATTERNS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoteEvent {
    Press,
    Release,
}

pub struct Sequencer {
    current_frame: u32,
    current_step: usize,
    current_song_pattern: Option<usize>,
    playing: bool,
    recording: bool,
    selected_pattern: usize,
    selected_instrument: usize,
    song_patterns: Vec<usize>,
    step_instruments_note: [[[u32; NUM_INSTRUMENTS]; NUM_STEPS]; NUM_PATTERNS],
    step_instruments_enabled: [[[bool; NUM_INSTRUMENTS]; NUM_STEPS]; NUM_PATTERNS],
    previous_frame_note_events: Vec<(u32, NoteEvent, u32)>,
    visual_song_model: Rc<VecModel<SongPatternData>>,
    visual_pattern_model: Rc<VecModel<PatternData>>,
    visual_step_model: Rc<VecModel<StepData>>,
}

impl Sequencer {
    pub fn new(sequencer_song_model: Rc<VecModel<SongPatternData>>, sequencer_pattern_model: Rc<VecModel<PatternData>>, sequencer_step_model: Rc<VecModel<StepData>>) -> Sequencer {
        let val = Sequencer {
            current_frame: 0,
            current_step: 0,
            current_song_pattern: Some(0),
            playing: true,
            recording: true,
            selected_pattern: 0,
            selected_instrument: 0,
            song_patterns: vec!(0,1,2,3),
            // Initialize all notes to C5
            step_instruments_note: [[[60; NUM_INSTRUMENTS]; NUM_STEPS]; NUM_PATTERNS],
            step_instruments_enabled: [[[false; NUM_INSTRUMENTS]; NUM_STEPS]; NUM_PATTERNS],
            previous_frame_note_events: Vec::new(),
            visual_song_model: sequencer_song_model,
            visual_pattern_model: sequencer_pattern_model,
            visual_step_model: sequencer_step_model,
        };
        for (i, number) in val.song_patterns.iter().enumerate() {
            val.visual_song_model.push(
                SongPatternData{
                    number: *number as i32,
                    active: match val.current_song_pattern {
                        Some(sp) => i == sp,
                        None => false,
                    }
                });
        }
        val
    }

    pub fn select_song_pattern(&mut self, song_pattern: Option<u32>) -> () {
        if let Some(current) = self.current_song_pattern {
            let mut pattern_row_data = self.visual_song_model.row_data(current);
            pattern_row_data.active = false;
            self.visual_song_model.set_row_data(current, pattern_row_data);
        }

        self.current_song_pattern = song_pattern.map(|sp| sp as usize);

        if let Some(current) = self.current_song_pattern {
            let mut pattern_row_data = self.visual_song_model.row_data(current);
            pattern_row_data.active = true;
            self.visual_song_model.set_row_data(current, pattern_row_data);
        }
    }

    pub fn select_pattern(&mut self, pattern: u32) -> () {
        let mut pattern_row_data = self.visual_pattern_model.row_data(self.selected_pattern);
        pattern_row_data.active = false;
        self.visual_pattern_model.set_row_data(self.selected_pattern, pattern_row_data);

        // FIXME: Queue the playback?
        self.selected_pattern = pattern as usize;

        let mut pattern_row_data = self.visual_pattern_model.row_data(self.selected_pattern);
        pattern_row_data.active = true;
        self.visual_pattern_model.set_row_data(self.selected_pattern, pattern_row_data);

        self.update_steps();
    }

    pub fn select_instrument(&mut self, instrument: u32) -> () {
        self.selected_instrument = instrument as usize;
        self.update_steps();
    }

    fn update_steps(&mut self) -> () {
        for i in 0..NUM_STEPS {
            let step_enabled = self.step_instruments_enabled[self.selected_pattern][i][self.selected_instrument];
            let mut row_data = self.visual_step_model.row_data(i);
            row_data.empty = !step_enabled;
            self.visual_step_model.set_row_data(i, row_data);
        }
    }

    pub fn toggle_step(&mut self, step_num: u32) -> () {
        let toggled = !self.step_instruments_enabled[self.selected_pattern][step_num as usize][self.selected_instrument];
        self.step_instruments_enabled[self.selected_pattern][step_num as usize][self.selected_instrument] = toggled;

        let mut pattern_row_data = self.visual_pattern_model.row_data(self.selected_pattern);
        pattern_row_data.empty = false;
        self.visual_pattern_model.set_row_data(self.selected_pattern, pattern_row_data);

        let mut step_row_data = self.visual_step_model.row_data(step_num as usize);
        step_row_data.empty = !toggled;
        self.visual_step_model.set_row_data(step_num as usize, step_row_data);
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

        // FIXME: Reset or remove overflow check
        self.current_frame += 1;
        if self.current_frame % 6 == 0 {
            let mut row_data = self.visual_step_model.row_data(self.current_step);
            row_data.active = false;
            self.visual_step_model.set_row_data(self.current_step, row_data);

            let (next_step, next_pattern, next_song_pattern) = self.next_step_and_pattern_and_song_pattern();
            self.current_step = next_step;
            if next_pattern != self.selected_pattern {
                self.select_pattern(next_pattern as u32);
            }
            if next_song_pattern != self.current_song_pattern {
                self.select_song_pattern(next_song_pattern.map(|sp| sp as u32));
            }

            let mut row_data = self.visual_step_model.row_data(self.current_step);
            row_data.active = true;
            self.visual_step_model.set_row_data(self.current_step, row_data);

            // Each note lasts only one frame, so just release everything pressed on the previous frame.
            for (instrument, typ, note) in &self.previous_frame_note_events {
                if *typ == NoteEvent::Press {
                    note_events.push((*instrument, NoteEvent::Release, *note));
                }
            }

            for (i, note) in self.step_instruments_note[self.selected_pattern][self.current_step].iter().enumerate() {
                if self.step_instruments_enabled[self.selected_pattern][self.current_step][i] {
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

        // Try to clamp the event to the nearest frame.
        // Use 4 instead of 3 just to try to compensate for the key press to visual and audible delay.
        let (step, pattern, _) =
            if self.current_frame < 4 {
                (self.current_step, self.selected_pattern, None)
            } else {
                self.next_step_and_pattern_and_song_pattern()
            };
        self.step_instruments_note[pattern][step][instrument as usize] = note;

        let already_enabled = self.step_instruments_enabled[pattern][step][instrument as usize];
        if !already_enabled {
            self.toggle_step(step as u32);
        }
    }

    pub fn append_song_pattern(&mut self, pattern: u32) {
        self.song_patterns.push(pattern as usize);
        self.visual_song_model.push(SongPatternData{number: pattern as i32, active: false});
    }

    pub fn remove_last_song_pattern(&mut self) {
        if !self.song_patterns.is_empty() {
            self.song_patterns.pop();
            if self.current_song_pattern.unwrap() == self.song_patterns.len() {
                self.select_song_pattern(if self.song_patterns.is_empty() { None } else { Some(0) });
            }
            self.visual_song_model.remove(self.visual_song_model.row_count() - 1);
        }
    }

    pub fn clear_song_patterns(&mut self) {
        self.song_patterns.clear();
        self.select_song_pattern(None);
        for _ in 0..self.visual_song_model.row_count() {
            self.visual_song_model.remove(0);
        }
    }

    fn next_step_and_pattern_and_song_pattern(&self) -> (usize, usize, Option<usize>) {
        if (self.current_step + 1) % NUM_STEPS == 0 {
            let (next_pattern, next_song_pattern) = if !self.song_patterns.is_empty() {
                let sp = self.current_song_pattern.map(|sp| (sp + 1) % self.song_patterns.len()).unwrap_or(0);
                (self.song_patterns[sp], Some(sp))
            } else {
                (self.selected_pattern, None)
            };
            return (0, next_pattern, next_song_pattern);
        }
        ((self.current_step + 1) % NUM_STEPS, self.selected_pattern, self.current_song_pattern)
    }
}
