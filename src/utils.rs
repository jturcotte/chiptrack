// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

#[cfg(feature = "desktop")]
use core::fmt;

use crate::ui::MainWindow;
use alloc::format;
use alloc::string::String;
use slint::EventLoopError;
use slint::SharedString;
use slint::Weak;
#[cfg(feature = "desktop")]
use url::Url;

#[rustfmt::skip]
// MIDI note number frequencies multiplied by 256 (8 bits fixed point).
// Instruments must take this into account before encoding to GBA frequencies.
pub static NOTE_FREQUENCIES: [u32; 128] = [
    // Octave -1
    2094, 2217, 2350, 2488, 2637, 2793, 2959, 3136, 3323, 3520, 3730, 3950,
    // Octave 0
    4186, 4434, 4698, 4979, 5274, 5588, 5919, 6272, 6646, 7040, 7460, 7903,
    // Octave 1
    8371, 8870, 9398, 9956, 10547, 11174, 11840, 12544, 13289, 14080, 14917, 15806,
    // Octave 2
    16745, 17741, 18796, 19912, 21097, 22351, 23680, 25088, 26580, 28160, 29834, 31608,
    // Octave 3
    33487, 35479, 37588, 39823, 42191, 44700, 47360, 50176, 53158, 56320, 59668, 63217,
    // Octave 4
    66977, 70958, 75177, 79649, 84385, 89403, 94718, 100352, 106317, 112640, 119337, 126433,
    // Octave 5
    133952, 141919, 150356, 159296, 168770, 178806, 189438, 200702, 212636, 225280, 238676, 252869,
    // Octave 6
    267904, 283835, 300713, 318594, 337538, 357609, 378875, 401403, 425272, 450560, 477353, 505736,
    // Octave 7
    535808, 567670, 601426, 637189, 675077, 715220, 757750, 802806, 850545, 901120, 954703, 1011474,
    // Octave 8
    1071618, 1135340, 1202852, 1274376, 1350154, 1430438, 1515497, 1605614, 1701089, 1802240, 1909407, 2022945,
    // Octave 9
    2143237, 2270679, 2405701, 2548751, 2700308, 2860877, 3030994, 3211226,
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
    pub fn base_note_name(&self) -> u8 {
        b'A' + (self.key_pos() as u8 + 2) % 7
    }

    pub fn char_desc(&self) -> [u8; 3] {
        let note_name = self.base_note_name();
        let sharp_char = if self.is_black() { '#' } else { '-' } as u8;
        let octave_char = b'0' + self.octave() as u8;
        [note_name, sharp_char, octave_char]
    }
    #[cfg(feature = "gba")]
    pub fn short_char_desc(&self) -> [u8; 2] {
        let note_name = self.base_note_name();
        let octave_char = b'0' + self.octave() as u8;
        [note_name, octave_char]
    }

    pub fn name(&self) -> String {
        let desc = self.char_desc();
        format!("{}{}{}", desc[0] as char, desc[1] as char, desc[2] as char)
    }
    pub fn short_name(&self) -> SharedString {
        let note_name = self.base_note_name() as char;
        let sharp_char = if self.is_black() { '#' } else { ' ' };
        format!("{}{}", note_name, sharp_char).into()
    }

    #[cfg(feature = "desktop")]
    pub fn from_freq(freq: f64) -> (MidiNote, f32) {
        let freq_a4 = 440.0;
        let f_c4_semi_tones = 12.0 * (freq / freq_a4).log2() + 9.0;
        let c4_semi_tones = f_c4_semi_tones.round() as i32;
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

    #[cfg(not(feature = "std"))]
    pub fn run_direct<R>(&self, func: impl FnOnce(MainWindow) -> R) -> R {
        func(self.inner.upgrade().unwrap())
    }
}

#[cfg(feature = "desktop")]
pub enum ParseGistUrlError {
    InvalidUrl(url::ParseError),
    InvalidHost,
}
#[cfg(feature = "desktop")]
impl fmt::Display for ParseGistUrlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseGistUrlError::InvalidUrl(e) => write!(f, "Invalid URL: {}", e),
            ParseGistUrlError::InvalidHost => write!(f, "Invalid host"),
        }
    }
}

#[cfg(feature = "desktop")]
pub fn parse_gist_url(u: &str) -> Result<String, ParseGistUrlError> {
    Url::parse(&u)
        .map_err(|e| e.into())
        .map_err(ParseGistUrlError::InvalidUrl)
        .and_then(|url| {
            if url.host_str().map_or(false, |h| h == "gist.github.com") {
                Ok(url.path().trim_start_matches('/').to_owned())
            } else {
                Err(ParseGistUrlError::InvalidHost)
            }
        })
}

#[cfg(feature = "desktop")]
pub fn fetch_gist(gist_url_path: String, handler: impl Fn(Result<serde_json::Value, String>) + Send + 'static) {
    let api_url = "https://api.github.com/gists/".to_owned() + gist_url_path.splitn(2, '/').last().unwrap();
    log!("Loading the project from gist API URL {}", api_url.to_string());
    ehttp::fetch(
        ehttp::Request::get(&api_url),
        move |result: ehttp::Result<ehttp::Response>| {
            result
                .map(|res| {
                    if res.ok {
                        let decoded: serde_json::Value =
                            serde_json::from_slice(&res.bytes).expect("JSON was not well-formatted");
                        handler(Ok(decoded));
                    } else {
                        handler(Err(format!("{} - {}", res.status, res.status_text)));
                    }
                })
                .unwrap_or_else(|err| {
                    handler(Err(format!(
                        "Error fetching the project from {}: {}.",
                        api_url.to_string(),
                        err
                    )));
                });
        },
    );
}
