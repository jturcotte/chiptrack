use crate::sixtyfps_generated_MainWindow::StepData;
use sixtyfps::Model;
use std::rc::Rc;

pub const NUM_STEPS: u32 = 64;

pub struct Sequencer {
    pub current_frame: u32,
    pub current_step: u32,
    pub step_instruments_freq: [[u32; 16]; NUM_STEPS as usize],
    pub step_changed_callback: Box<dyn Fn(u32) -> ()>,
    pub visual_step_model: Rc<sixtyfps::VecModel<StepData>>,
}

impl Sequencer {
    pub fn advance_frame<F>(&mut self, mut instrument_fn: F) where F: FnMut(usize, u32) -> () {
        self.current_frame += 1;
        if self.current_frame % 6 == 0 {
            let next_step = (self.current_step + 1) % NUM_STEPS;
            self.current_step = next_step;
            (self.step_changed_callback)(self.current_step);

            if self.current_step % 16 == 0 {
                for i in 0..16 {
                    let empty = self.step_instruments_freq[next_step as usize + i].iter().sum::<u32>() == 0;
                    self.visual_step_model.set_row_data(i, StepData{empty: empty,});
                }
            }

            for (i, freq) in self.step_instruments_freq[next_step as usize].iter().enumerate() {
                if *freq != 0 {
                    println!("Instrument {:?} freq {:?}", i, freq);
                    instrument_fn(i, *freq);
                }
            }
        }
    }

    pub fn record_trigger(&mut self, instrument: usize, freq: u32) {
        self.step_instruments_freq[self.current_step as usize][instrument] = freq;
        self.visual_step_model.set_row_data((self.current_step % 16) as usize, StepData{empty: false,});
    }
}
