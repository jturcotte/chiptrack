use crate::sequencer::Sequencer;
use crate::synth::Synth;

pub struct SoundStuff {
    pub sequencer: Sequencer,
    pub synth: Synth,
    pub selected_instrument: usize,
}

impl SoundStuff {
    pub fn advance_frame(&mut self) -> () {
        self.sequencer.advance_frame(&mut self.synth);
        self.synth.advance_frame();
    }

    pub fn trigger_selected_instrument(&mut self, freq: u32) -> () {
        self.synth.trigger_instrument(self.selected_instrument, freq);
    }
}

