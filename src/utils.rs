// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use sixtyfps::SharedString;
use std::path::PathBuf;

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
    pub fn name(&self) -> SharedString {
        let note_name = ('A' as u8 + (self.key_pos() as u8 + 2) % 7) as char;
        let sharp_char = if self.is_black() { '#' } else { '-' };
        format!("{}{}{}", note_name, sharp_char, self.octave()).into()
    }

    pub fn from_freq(freq: f64) -> (MidiNote, u32) {
        let freq_a4 = 440.0;
        let f_c4_semi_tones = 12.0 * (freq / freq_a4).log2() + 9.0;
        let c4_semi_tones = f_c4_semi_tones.round() as i32;
        let cents = ((f_c4_semi_tones - c4_semi_tones as f64) * 100.0).round() as u32;
        (MidiNote(c4_semi_tones + 60), cents)
    }
}

pub fn encode_file(path: PathBuf) -> String {
    let instruments_data = std::fs::read(path).unwrap();
    let compressed = miniz_oxide::deflate::compress_to_vec(&instruments_data, 9);
    base64_url::encode(&compressed)
}

#[cfg(target_arch = "wasm32")]
pub fn decode_string(base64: &str) -> Result<String, Box<dyn std::error::Error>> {
    let decoded = base64_url::decode(&base64)?;
    let decompressed = miniz_oxide::inflate::decompress_to_vec(decoded.as_slice())
        .map_err(|s| format!("{:?}", s))?;
    let utf8 = String::from_utf8(decompressed)?;
    Ok(utf8)
}
