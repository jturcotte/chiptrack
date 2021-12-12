// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use sixtyfps::SharedString;
use std::path::PathBuf;

pub fn midi_note_name(note: u32) -> SharedString {
    let octave = (note / 12) - 1;
    let note_names = ["C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-"];
    let mut note_name = SharedString::from(note_names[note as usize % 12]);
    note_name.push_str(&octave.to_string());
    note_name
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
