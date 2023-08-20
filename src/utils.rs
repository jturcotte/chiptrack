// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::MainWindow;
use alloc::format;
use alloc::string::String;
use slint::EventLoopError;
use slint::SharedString;
use slint::Weak;

#[rustfmt::skip]
// MIDI note number frequencies multiplied by 1024 (5 bits fixed point).
pub static NOTE_FREQUENCIES: [u32; 128] = [
    // Octave -1
    8376, 8868, 9400, 9953, 10547, 11172, 11837, 12544, 13292, 14080, 14920, 15800, 
    // Octave 0
    16742, 17736, 18790, 19917, 21094, 22354, 23675, 25088, 26583, 28160, 29839, 31611,
    // Octave 1
    33485, 35482, 37591, 39823, 42189, 44698, 47360, 50176, 53156, 56320, 59668, 63222,
    // Octave 2
    66980, 70963, 75182, 79647, 84388, 89405, 94720, 100352, 106322, 112640, 119337, 126433,
    // Octave 3
    133949, 141916, 150354, 159293, 168765, 178801, 189440, 200704, 212634, 225280, 238674, 252867,
    // Octave 4
    267909, 283832, 300708, 318597, 337541, 357612, 378870, 401408, 425267, 450560, 477348, 505733,
    // Octave 5
    535808, 567675, 601426, 637184, 675082, 715223, 757750, 802806, 850545, 901120, 954706, 1011476,
    // Octave 6
    1071616, 1135340, 1202852, 1274378, 1350154, 1430436, 1515500, 1605612, 1701089, 1802240, 1909412, 2022943,
    // Octave 7
    2143232, 2270679, 2405704, 2548756, 2700308, 2860882, 3030999, 3211223, 3402179, 3604480, 3818813, 4045896,
    // Octave 8
    4286474, 4541358, 4811407, 5097503, 5400617, 5721754, 6061988, 6422456, 6804357, 7208960, 7637627, 8091781,
    // Octave 9
    8572948, 9082716, 9622804, 10195005, 10801234, 11443507, 12123976, 12844902,
];

#[derive(Debug, Clone, Copy)]
pub struct MidiNote(pub i32);

impl MidiNote {
    pub fn octave(self) -> i32 {
        (self.0 / 12) - 1
    }
    pub fn semitone(self) -> i32 {
        self.0 % 12
    }
    pub fn key_pos(self) -> i32 {
        let semitone = self.semitone();
        if semitone < 5 {
            semitone / 2
        } else {
            (semitone + 1) / 2
        }
    }
    pub fn is_black(self) -> bool {
        let semitone = self.semitone();
        if semitone < 5 {
            semitone % 2 != 0
        } else {
            semitone % 2 == 0
        }
    }
    pub fn base_note_name(&self) -> char {
        (b'A' + (self.key_pos() as u8 + 2) % 7) as char
    }
    pub fn char_desc(&self) -> [char; 3] {
        let note_name = self.base_note_name();
        let sharp_char = if self.is_black() { '#' } else { '-' };
        let octave_char = (b'0' + self.octave() as u8) as char;
        [note_name, sharp_char, octave_char]
    }
    pub fn name(&self) -> String {
        let desc = self.char_desc();
        format!("{}{}{}", desc[0], desc[1], desc[2])
    }
    pub fn short_name(&self) -> SharedString {
        let note_name = self.base_note_name();
        let sharp_char = if self.is_black() { '#' } else { ' ' };
        format!("{}{}", note_name, sharp_char).into()
    }

    #[cfg(feature = "desktop")]
    pub fn from_freq(freq: f64) -> (MidiNote, f32) {
        let freq_a4 = 440.0;
        let f_c4_semi_tones = 12.0 * (freq / freq_a4).log2() + 9.0;
        let c4_semi_tones = f_c4_semi_tones as i32;
        // Cents, but just returned as [-0.5, 0.5].
        let cent_adj = (f_c4_semi_tones - c4_semi_tones as f64) as f32;
        (MidiNote(c4_semi_tones + 60), cent_adj)
    }

    #[cfg(feature = "desktop")]
    pub fn from_name(name: &str) -> Result<MidiNote, String> {
        let mut chars = name.chars();

        let c1 = chars.next().ok_or("MidiNote string too small")?;
        let c2 = chars.next().ok_or("MidiNote string too small")?;
        let c3 = chars.next().ok_or("MidiNote string too small")?;
        let base = match c1 {
            'A' => 9,
            'B' => 11,
            'C' => 0,
            'D' => 2,
            'E' => 4,
            'F' => 5,
            'G' => 7,
            _ => return Err("Invalid MidiNote first char, should be a note letter".into()),
        };
        let accidental_adj = match c2 {
            '#' => 1,
            '-' => 0,
            _ => return Err("Invalid MidiNote second char, should be - or #".into()),
        };
        let octave = (c3 as i32 - '0' as i32).min(9);
        Ok(MidiNote(12 + octave * 12 + base + accidental_adj))
    }
}

#[derive(Clone)]
pub struct WeakWindowWrapper {
    inner: Weak<MainWindow>,
}

impl WeakWindowWrapper {
    pub fn new(inner: Weak<MainWindow>) -> WeakWindowWrapper {
        WeakWindowWrapper { inner }
    }

    #[cfg(feature = "std")]
    pub fn upgrade_in_event_loop(&self, func: impl FnOnce(MainWindow) + Send + 'static) -> Result<(), EventLoopError> {
        self.inner.upgrade_in_event_loop(func)
    }

    #[cfg(not(feature = "std"))]
    pub fn upgrade_in_event_loop(&self, func: impl FnOnce(MainWindow)) -> Result<(), EventLoopError> {
        func(self.inner.upgrade().unwrap());
        Ok(())
    }
}
