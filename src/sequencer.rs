use crate::sixtyfps_generated_MainWindow::SongPatternData;
use crate::MainWindow;
use serde::{Serialize, Deserialize};
use sixtyfps::Model;
use sixtyfps::VecModel;
use sixtyfps::Weak;
use std::fs::File;
use std::path::PathBuf;

pub const NUM_INSTRUMENTS: usize = 9;
pub const NUM_STEPS: usize = 16;
pub const NUM_PATTERNS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoteEvent {
    Press,
    Release,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
struct InstrumentStep {
    note: u32,
    enabled: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SequencerSong {
    selected_instrument: usize,
    song_patterns: Vec<usize>,
    step_instruments: [[[InstrumentStep; NUM_STEPS]; NUM_INSTRUMENTS]; NUM_PATTERNS],
}

pub struct Sequencer {
    current_frame: u32,
    current_step: usize,
    current_song_pattern: Option<usize>,
    selected_pattern: usize,
    playing: bool,
    recording: bool,
    song: SequencerSong,
    previous_frame_note_events: Vec<(u32, NoteEvent, u32)>,
    main_window: Weak<MainWindow>,
}

impl Sequencer {
    pub fn new(project_name: &str, main_window: Weak<MainWindow>) -> Sequencer {
        let song = Sequencer::load(project_name).unwrap_or(SequencerSong {
            selected_instrument: 0,
            song_patterns: Vec::new(),
            // Initialize all notes to C5
            step_instruments: [[[InstrumentStep{note: 60, enabled: false}; NUM_STEPS]; NUM_INSTRUMENTS]; NUM_PATTERNS],
        });
        let mut val = Sequencer {
            current_frame: 0,
            current_step: 0,
            current_song_pattern: if song.song_patterns.is_empty() { None } else { Some(0) },
            selected_pattern: 0,
            playing: true,
            recording: true,
            song: song,
            previous_frame_note_events: Vec::new(),
            main_window: main_window.clone(),
        };
        let current_song_pattern = val.current_song_pattern;
        let song_patterns = val.song.song_patterns.clone();
        main_window.clone().upgrade_in_event_loop(move |handle| {
            let model = handle.get_sequencer_song_patterns();
            let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
            for (i, number) in song_patterns.iter().enumerate() {
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

        val.select_pattern(
            *val.current_song_pattern
                .map(|i| val.song.song_patterns.get(i).unwrap())
                .unwrap_or(&0_usize) as u32
            );
        val.select_instrument(val.song.selected_instrument as u32);
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
        self.song.selected_instrument = instrument as usize;

        self.update_steps();

        self.main_window.clone().upgrade_in_event_loop(move |handle| {
            handle.set_selected_instrument(instrument as i32);
        });
    }

    fn update_steps(&mut self) -> () {
        let enabled_list: Vec<bool> = 
            (0..NUM_STEPS)
                .map(|i| self.song.step_instruments[self.selected_pattern][self.song.selected_instrument][i].enabled)
                .collect();
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
        let toggled = !self.song.step_instruments[self.selected_pattern][self.song.selected_instrument][step_num as usize].enabled;
        self.song.step_instruments[self.selected_pattern][self.song.selected_instrument][step_num as usize].enabled = toggled;

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

            for i in 0..NUM_INSTRUMENTS {
                let InstrumentStep{note, enabled} = self.song.step_instruments[self.selected_pattern][i][self.current_step];
                if enabled {
                    println!("Instrument {:?} note {:?}", i, note);
                    note_events.push((i as u32, NoteEvent::Press, note));
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
        self.song.step_instruments[pattern][instrument as usize][step].note = note;

        let already_enabled = self.song.step_instruments[pattern][instrument as usize][step].enabled;
        if !already_enabled {
            self.toggle_step(step as u32);
        }
    }

    pub fn append_song_pattern(&mut self, pattern: u32) {
        self.song.song_patterns.push(pattern as usize);

        self.main_window.clone().upgrade_in_event_loop(move |handle| {
            let model = handle.get_sequencer_song_patterns();
            let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
            vec_model.push(SongPatternData{number: pattern as i32, active: false});
        });
    }

    pub fn remove_last_song_pattern(&mut self) {
        if !self.song.song_patterns.is_empty() {
            self.song.song_patterns.pop();
            if self.current_song_pattern.unwrap() == self.song.song_patterns.len() {
                self.select_song_pattern(if self.song.song_patterns.is_empty() { None } else { Some(0) });
            }

            self.main_window.clone().upgrade_in_event_loop(move |handle| {
                let model = handle.get_sequencer_song_patterns();
                let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
                vec_model.remove(vec_model.row_count() - 1);
            });
        }
    }

    pub fn clear_song_patterns(&mut self) {
        self.song.song_patterns.clear();
        self.select_song_pattern(None);

        self.main_window.clone().upgrade_in_event_loop(move |handle| {
            let model = handle.get_sequencer_song_patterns();
            let vec_model = model.as_any().downcast_ref::<VecModel<SongPatternData>>().unwrap();
            for _ in 0..vec_model.row_count() {
                vec_model.remove(0);
            }
        });
    }

    fn project_path(project_name: &str) -> PathBuf {
        let mut path = PathBuf::new();
        path.push(project_name.to_owned() + "-song.json");
        path.canonicalize().unwrap()
    }
    fn load(project_name: &str) -> Option<SequencerSong> {
        let path = Sequencer::project_path(project_name);
        if path.exists() {
            let parsed =
                File::open(path.as_path())
                .and_then(|f| serde_json::from_reader(f).map_err(|e| e.into()));

            match parsed {
                Ok(song) => {
                    log!("Loaded project song from file {:?}", path);
                    Some(song)
                },
                Err(e) => {
                    elog!("Couldn't load project song from file {:?}, starting from scratch.\n\tError: {:?}", path, e);
                    None
                },
            }            
        } else {
            log!("Project song file {:?} doesn't exist, starting from scratch.", path);
            None
        }
    }

    pub fn save(&self, project_name: &str) {
        let path = Sequencer::project_path(project_name);
        println!("Saving project song to file {:?}.", path);
        let f = File::create(path).expect("Unable to create project file");
        serde_json::to_writer_pretty(&f, &self.song).unwrap()
    }

    fn next_step_and_pattern_and_song_pattern(&self) -> (usize, usize, Option<usize>) {
        if (self.current_step + 1) % NUM_STEPS == 0 {
            let (next_pattern, next_song_pattern) = if !self.song.song_patterns.is_empty() {
                let sp = self.current_song_pattern.map(|sp| (sp + 1) % self.song.song_patterns.len()).unwrap_or(0);
                (self.song.song_patterns[sp], Some(sp))
            } else {
                (self.selected_pattern, None)
            };
            return (0, next_pattern, next_song_pattern);
        }
        ((self.current_step + 1) % NUM_STEPS, self.selected_pattern, self.current_song_pattern)
    }
}
