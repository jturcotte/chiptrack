use crate::sixtyfps_generated_MainWindow::StepData;
use crate::sixtyfps_generated_MainWindow::MainWindow;
use sixtyfps::Model;
use sixtyfps::VecModel;
use sixtyfps::Weak;
use std::rc::Rc;

pub const NUM_STEPS: u32 = 64;

pub struct Sequencer {
    current_frame: u32,
    current_step: u32,
    locked_bar: Option<u32>,
    playing: bool,
    recording: bool,
    step_instruments_freq: [[u32; 16]; NUM_STEPS as usize],
    step_changed_callback: Box<dyn Fn(u32) -> ()>,
    visual_step_model: Rc<VecModel<StepData>>,
}

impl Sequencer {
    pub fn new(window_weak: Weak<MainWindow>, sequencer_step_model: Rc<VecModel<StepData>>) -> Sequencer {
        Sequencer {
            current_frame: 0,
            current_step: 0,
            locked_bar: None,
            playing: true,
            recording: true,
            step_instruments_freq: [[0; 16]; NUM_STEPS as usize],
            step_changed_callback: Box::new(move |s| {
                let window = window_weak.unwrap();
                window.set_current_sequencer_bar(s as i32 / 16);
                window.set_current_sequencer_step(s as i32 % 16);
            }),
            visual_step_model: sequencer_step_model,
        }
    }

    pub fn set_locked_bar(&mut self, bar_num: Option<u32>) -> Option<u32> {
        if self.locked_bar == bar_num {
            self.locked_bar = None;
        } else {
            self.locked_bar = bar_num;
        }
        self.locked_bar
    }
    pub fn set_playing(&mut self, val: bool) -> () {
        self.playing = val;
    }
    pub fn set_recording(&mut self, val: bool) -> () {
        self.recording = val;
    }
    pub fn advance_frame<F>(&mut self, mut instrument_fn: F) where F: FnMut(usize, u32) -> () {
        if !self.playing {
            return;
        }

        self.current_frame += 1;
        if self.current_frame % 6 == 0 {
            let mut next_step = (self.current_step + 1) % NUM_STEPS;

            if next_step % 16 == 0 {
                if let Some(locked_bar) = self.locked_bar {
                    next_step = locked_bar * 16;
                }
                for i in 0..16 {
                    let empty = self.step_instruments_freq[next_step as usize + i].iter().sum::<u32>() == 0;
                    self.visual_step_model.set_row_data(i, StepData{empty: empty,});
                }
            }
            self.current_step = next_step;
            (self.step_changed_callback)(self.current_step);

            for (i, freq) in self.step_instruments_freq[next_step as usize].iter().enumerate() {
                if *freq != 0 {
                    println!("Instrument {:?} freq {:?}", i, freq);
                    instrument_fn(i, *freq);
                }
            }
        }
    }

    pub fn record_trigger(&mut self, instrument: usize, freq: u32) {
        if !self.recording {
            return;
        }

        self.step_instruments_freq[self.current_step as usize][instrument] = freq;
        self.visual_step_model.set_row_data((self.current_step % 16) as usize, StepData{empty: false,});
    }
}
