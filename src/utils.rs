// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

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
    pub fn name(&self) -> String {
        let note_name = ('A' as u8 + (self.key_pos() as u8 + 2) % 7) as char;
        let sharp_char = if self.is_black() { '#' } else { '-' };
        format!("{}{}{}", note_name, sharp_char, self.octave())
    }

    pub fn from_freq(freq: f64) -> (MidiNote, f32) {
        let freq_a4 = 440.0;
        let f_c4_semi_tones = 12.0 * (freq / freq_a4).log2() + 9.0;
        let c4_semi_tones = f_c4_semi_tones.round() as i32;
        // Cents, but just returned as [-0.5, 0.5].
        let cent_adj = (f_c4_semi_tones - c4_semi_tones as f64) as f32;
        (MidiNote(c4_semi_tones + 60), cent_adj)
    }

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
