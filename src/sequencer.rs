use crate::sixtyfps_generated_MainWindow::SongPatternData;
use crate::MainWindow;
use sixtyfps::Model;
use sixtyfps::VecModel;
use sixtyfps::Weak;

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
    main_window: Weak<MainWindow>,
}

impl Sequencer {
    pub fn new(main_window: Weak<MainWindow>) -> Sequencer {
        let test_patterns = vec!(0,1,2,3);
        let val = Sequencer {
            current_frame: 0,
            current_step: 0,
            current_song_pattern: Some(0),
            playing: true,
            recording: true,
            selected_pattern: 0,
            selected_instrument: 0,
            song_patterns: test_patterns.clone(),
            // Initialize all notes to C5
            step_instruments_note: [[[60; NUM_INSTRUMENTS]; NUM_STEPS]; NUM_PATTERNS],
            step_instruments_enabled: [[[false; NUM_INSTRUMENTS]; NUM_STEPS]; NUM_PATTERNS],
            previous_frame_note_events: Vec::new(),
            main_window: main_window.clone(),
        };
        let current_song_pattern = val.current_song_pattern;
        main_window.clone().upgrade_in_event_loop(move |handle| {
            let model = handle.get_sequencer_song_patterns();
            let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
            for (i, number) in test_patterns.iter().enumerate() {
                vec_model.push(
                    SongPatternData{
                        number: *number as i32,
                        active: match current_song_pattern {
                            Some(sp) => i == sp,
                            None => false,
                        }
                    });
            }
        });
        val
    }

    pub fn select_song_pattern(&mut self, song_pattern: Option<u32>) -> () {
        let old = self.current_song_pattern;
        self.current_song_pattern = song_pattern.map(|sp| sp as usize);
        let new = self.current_song_pattern;

        self.main_window.clone().upgrade_in_event_loop(move |handle| {
            let model = handle.get_sequencer_song_patterns();
            if let Some(current) = old {
                let mut pattern_row_data = model.row_data(current);
                pattern_row_data.active = false;
                model.set_row_data(current, pattern_row_data);
            }
            if let Some(current) = new {
                let mut pattern_row_data = model.row_data(current);
                pattern_row_data.active = true;
                model.set_row_data(current, pattern_row_data);
            }
        });
    }

    pub fn select_pattern(&mut self, pattern: u32) -> () {
        let old = self.selected_pattern;
        // FIXME: Queue the playback?
        self.selected_pattern = pattern as usize;
        let new = self.selected_pattern;

        self.update_steps();

        self.main_window.clone().upgrade_in_event_loop(move |handle| {
            let model = handle.get_sequencer_patterns();
            let mut pattern_row_data = model.row_data(old);
            pattern_row_data.active = false;
            model.set_row_data(old, pattern_row_data);

            let mut pattern_row_data = model.row_data(new);
            pattern_row_data.active = true;
            model.set_row_data(new, pattern_row_data);
        });
    }

    pub fn select_instrument(&mut self, instrument: u32) -> () {
        self.selected_instrument = instrument as usize;
        self.update_steps();
    }

    fn update_steps(&mut self) -> () {
        let enabled_list: Vec<bool> = (0..NUM_STEPS).map(|i| self.step_instruments_enabled[self.selected_pattern][i][self.selected_instrument]).collect();
        self.main_window.clone().upgrade_in_event_loop(move |handle| {
            let model = handle.get_sequencer_steps();
            for (i, step_enabled) in enabled_list.iter().enumerate() {
                let mut row_data = model.row_data(i);
                row_data.empty = !step_enabled;
                model.set_row_data(i, row_data);
            }
        });
    }

    pub fn toggle_step(&mut self, step_num: u32) -> () {
        let toggled = !self.step_instruments_enabled[self.selected_pattern][step_num as usize][self.selected_instrument];
        self.step_instruments_enabled[self.selected_pattern][step_num as usize][self.selected_instrument] = toggled;

        let selected_pattern = self.selected_pattern;
        self.main_window.clone().upgrade_in_event_loop(move |handle| {

            let patterns = handle.get_sequencer_patterns();
            let mut pattern_row_data = patterns.row_data(selected_pattern);
            pattern_row_data.empty = false;
            patterns.set_row_data(selected_pattern, pattern_row_data);

            let steps = handle.get_sequencer_steps();
            let mut step_row_data = steps.row_data(step_num as usize);
            step_row_data.empty = !toggled;
            steps.set_row_data(step_num as usize, step_row_data);
        });
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
            let (next_step, next_pattern, next_song_pattern) = self.next_step_and_pattern_and_song_pattern();
            let old_step = self.current_step;
            self.current_step = next_step;

            if next_pattern != self.selected_pattern {
                self.select_pattern(next_pattern as u32);
            }
            if next_song_pattern != self.current_song_pattern {
                self.select_song_pattern(next_song_pattern.map(|sp| sp as u32));
            }

            self.main_window.clone().upgrade_in_event_loop(move |handle| {
                let model = handle.get_sequencer_steps();
                let mut row_data = model.row_data(old_step);
                row_data.active = false;
                model.set_row_data(old_step, row_data);

                let mut row_data = model.row_data(next_step);
                row_data.active = true;
                model.set_row_data(next_step, row_data);
            });

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

        self.main_window.clone().upgrade_in_event_loop(move |handle| {
            let model = handle.get_sequencer_song_patterns();
            let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
            vec_model.push(SongPatternData{number: pattern as i32, active: false});
        });
    }

    pub fn remove_last_song_pattern(&mut self) {
        if !self.song_patterns.is_empty() {
            self.song_patterns.pop();
            if self.current_song_pattern.unwrap() == self.song_patterns.len() {
                self.select_song_pattern(if self.song_patterns.is_empty() { None } else { Some(0) });
            }

            self.main_window.clone().upgrade_in_event_loop(move |handle| {
                let model = handle.get_sequencer_song_patterns();
                let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
                vec_model.remove(vec_model.row_count() - 1);
            });
        }
    }

    pub fn clear_song_patterns(&mut self) {
        self.song_patterns.clear();
        self.select_song_pattern(None);

        self.main_window.clone().upgrade_in_event_loop(move |handle| {
            let model = handle.get_sequencer_song_patterns();
            let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
            for _ in 0..vec_model.row_count() {
                vec_model.remove(0);
            }
        });
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
