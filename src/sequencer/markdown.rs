// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::sequencer::InstrumentStep;
use crate::sequencer::SequencerSong;
use crate::sound_engine::NUM_INSTRUMENTS;
use crate::sound_engine::NUM_PATTERNS;
use crate::sound_engine::NUM_STEPS;
use crate::utils::MidiNote;

use pulldown_cmark::Event::*;
use pulldown_cmark::OffsetIter;
use pulldown_cmark::Options;
use pulldown_cmark::Parser;
use pulldown_cmark::Tag;
use pulldown_cmark::Tag::*;
use regex::Regex;

use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;

const INSTRUMENTS_FILE_SETTING: &str = "InstrumentsFile";
const FRAMES_PER_STEP_SETTING: &str = "FramesPerStep";

#[derive(PartialEq)]
enum Section {
    Other,
    Song,
    Pattern(usize),
    Settings,
}

struct MarkdownSongParser<'a, 'b> {
    source: &'a str,
    iter: OffsetIter<'a, 'b>,

    tag_stack: Vec<Tag<'a>>,
    section: Section,
    table_column: Option<usize>,
    table_row: Option<usize>,
    table_instrument_ids: Vec<String>,
    found_known_section_heading: bool,
    out: SequencerSong,
}

impl<'a, 'b> MarkdownSongParser<'a, 'b> {
    fn new(source: &'a str, iter: OffsetIter<'a, 'b>) -> Self {
        Self {
            source,
            iter,
            tag_stack: Vec::new(),
            section: Section::Other,
            table_column: None,
            table_row: None,
            table_instrument_ids: Vec::new(),
            found_known_section_heading: false,
            out: Default::default(),
        }
    }

    fn run(mut self) -> Result<SequencerSong, Box<dyn std::error::Error>> {
        let pattern_re = Regex::new(r"Pattern (\d+)").unwrap();
        let setting_re = Regex::new(r"(\w+): (.*)").unwrap();

        for (event, tag_range) in self.iter.by_ref() {
            match event {
                Start(tag) => {
                    match tag {
                        TableRow => {
                            self.table_row = match self.table_row {
                                Some(r) => {
                                    if matches!(self.section, Section::Pattern(_)) && r >= 15 {
                                        return Err("A pattern's steps table shouldn't have more than 16 rows".into());
                                    }
                                    Some(r + 1)
                                }
                                None => Some(0),
                            }
                        }
                        TableCell => {
                            self.table_column = match self.table_column {
                                Some(c) => Some(c + 1),
                                None => Some(0),
                            };
                        }
                        _ => (),
                    }
                    self.tag_stack.push(tag);
                }
                End(tag) => {
                    let last = self.tag_stack.pop();
                    assert!(last.map_or(false, |l| l == tag));

                    match tag {
                        Heading(..) => {
                            // Keep any markdown data before the first specified known section Heading
                            // so that we can save it back.
                            // Do this in the End event to leave the Text event handler the chance to
                            // set the section, while still having access to the full Heading tag_range.
                            if !self.found_known_section_heading && self.section != Section::Other {
                                self.found_known_section_heading = true;
                                self.out.markdown_header = self.source[..tag_range.start].to_owned();
                            }
                        }
                        Table(..) => {
                            if matches!(self.section, Section::Pattern(_)) && self.table_row != Some(15) {
                                return Err("A pattern's steps table should have exactly 16 rows".into());
                            }
                            self.table_row = None;
                            self.table_instrument_ids.clear();
                        }
                        TableRow | TableHead => {
                            self.table_column = None;
                        }
                        _ => (),
                    }
                }
                Text(text) => {
                    if matches!(self.tag_stack.last(), Some(Heading(..))) {
                        self.section = if let Some(pattern_cap) = pattern_re.captures(&text) {
                            let num: usize = pattern_cap.get(1).unwrap().as_str().parse()?;
                            if num == 0 || num > NUM_PATTERNS {
                                return Err(format!("Pattern headings must be >= 1 and <= 64: [{}]", &*text).into());
                            }
                            Section::Pattern(num - 1)
                        } else {
                            match &*text {
                                "Song" => Section::Song,
                                "Settings" => Section::Settings,
                                other => {
                                    if self.found_known_section_heading {
                                        elog!("Found unknown section in song file [{}] after a known section. This will be lost if the file is saved.", other);
                                    }
                                    Section::Other
                                }
                            }
                        };
                    } else if self.section == Section::Song && self.tag_stack.contains(&Item) {
                        let caps = pattern_re
                            .captures(&text)
                            .ok_or_else(|| format!("Invalid song pattern name: [{}]", &*text))?;
                        let parsed: usize = caps.get(1).unwrap().as_str().parse()?;
                        self.out.song_patterns.push(parsed - 1);
                    } else if matches!(self.section, Section::Pattern(_)) && self.tag_stack.contains(&TableHead) {
                        let text = &self.source[tag_range].trim();
                        self.table_instrument_ids.push((*text).to_owned());
                    } else if matches!(self.section, Section::Pattern(_)) && self.tag_stack.contains(&TableRow) {
                        if let Section::Pattern(pattern_idx) = self.section {
                            let (text, ends_with_period) = text
                                .strip_suffix('.')
                                .map(|prefix| (prefix.trim(), true))
                                .unwrap_or((text.trim(), false));
                            let instrument_id = &self.table_instrument_ids[self.table_column.unwrap()];
                            let step = &mut self.out.patterns[pattern_idx].get_steps_mut(instrument_id, None)
                                [self.table_row.unwrap()];
                            if !text.is_empty() && text != "-" {
                                let MidiNote(note) = MidiNote::from_name(text)?;
                                step.note = note as u8;
                            }
                            if ends_with_period {
                                step.release = true;
                            }
                        }
                    } else if self.section == Section::Settings && self.tag_stack.contains(&Item) {
                        let caps = setting_re.captures(&text).ok_or_else(|| {
                            format!("Invalid setting format: [{}]. Should be [SettingName: Value].", &*text)
                        })?;
                        let name = caps.get(1).unwrap().as_str();
                        let value = caps.get(2).unwrap().as_str();
                        match name {
                            INSTRUMENTS_FILE_SETTING => self.out.instruments_file = value.into(),
                            FRAMES_PER_STEP_SETTING => {
                                self.out.frames_per_step = value.parse().or(Err(format!(
                                    "Setting {} contains an invalid integer ({}).",
                                    FRAMES_PER_STEP_SETTING, value
                                )))?
                            }
                            other => elog!("Unknown song setting [{}], ignoring.", other),
                        }
                    }
                }
                Code(text) => {
                    // Step param values are wrapped in backticks, so they'll appear as Code here and we just need to split by /
                    if self.tag_stack.contains(&TableRow) {
                        if let Section::Pattern(pattern_idx) = self.section {
                            let instrument_id = &self.table_instrument_ids[self.table_column.unwrap()];
                            let step = &mut self.out.patterns[pattern_idx].get_steps_mut(instrument_id, None)
                                [self.table_row.unwrap()];
                            for (i, s) in text.split('/').enumerate() {
                                let val = if s.is_empty() { None } else { Some(s.parse::<i8>()?) };
                                match i {
                                    0 => step.set_param0(val),
                                    1 => step.set_param1(val),
                                    _ => Err(format!("Too many param: {}", text))?,
                                }
                            }
                        }
                    }
                }
                _ => (),
            }
        }

        if self.out.instruments_file.is_empty() {
            Err(format!(
                "The song must contain a Settings section containing a value for the {} setting",
                INSTRUMENTS_FILE_SETTING
            )
            .into())
        } else {
            Ok(self.out)
        }
    }
}

pub fn parse_markdown_song(markdown: &str) -> Result<SequencerSong, Box<dyn std::error::Error>> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    let events = Parser::new_ext(markdown, options).into_offset_iter();

    MarkdownSongParser::new(markdown, events).run()
}

pub fn save_markdown_song(song: &SequencerSong, project_song_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    log!("Saving project song to file {:?}.", project_song_path);
    let f = File::create(project_song_path)?;
    let mut f = BufWriter::new(f);

    f.write_all(song.markdown_header.as_bytes())?;

    if !song.song_patterns.is_empty() {
        write!(f, "## Song\n\n")?;

        for spi in song.song_patterns.iter() {
            writeln!(f, "- [Pattern {}](#pattern-{})", spi + 1, spi + 1)?;
        }

        writeln!(f)?;
    }

    for (pi, p) in song.patterns.iter().enumerate() {
        let mut non_empty = Vec::with_capacity(NUM_INSTRUMENTS);
        for (ii, i) in p.instruments.iter().enumerate() {
            if i.steps.iter().any(|s| s.press() || s.release) {
                non_empty.push(ii);
            }
        }
        non_empty.sort_by(|a, b| {
            let mut ai = p.instruments[*a].synth_index.unwrap() as u32;
            let mut bi = p.instruments[*b].synth_index.unwrap() as u32;
            // Instrument are indiced by UI pages and have sequenced by row,
            // but we want to sort by column first, so change the order by moving
            // the 2 column bits from being least significant to being most significant.
            ai |= (ai & 0x3) << 8;
            bi |= (bi & 0x3) << 8;
            ai.partial_cmp(&bi).unwrap()
        });

        if !non_empty.is_empty() {
            write!(f, "## Pattern {}\n\n", pi + 1)?;

            fn params_string(s: &InstrumentStep) -> String {
                match (s.param0(), s.param1()) {
                    (Some(p0), Some(p1)) => format!(" `{}/{}`", p0, p1),
                    (Some(p), None) => format!(" `{}`", p),
                    (None, Some(p)) => format!(" `/{}`", p),
                    (None, None) => String::new(),
                }
            }
            let param_max_widths: Vec<_> = non_empty
                .iter()
                .map(|ii| {
                    let i = &p.instruments[*ii];
                    i.steps.iter().map(|s| params_string(s).len()).max().unwrap()
                })
                .collect();

            for (ii, param_width) in non_empty.iter().zip(param_max_widths.iter()) {
                let id = &p.instruments[*ii].id;
                write!(f, "|{: ^1$}", id, 4 + param_width)?;
            }
            writeln!(f, "|")?;
            for (_, param_width) in non_empty.iter().zip(param_max_widths.iter()) {
                write!(f, "|----{:-^1$}", "", param_width)?;
            }
            writeln!(f, "|")?;

            for si in 0..NUM_STEPS {
                for (ii, param_width) in non_empty.iter().zip(param_max_widths.iter()) {
                    let i = &p.instruments[*ii];
                    let s = i.steps[si];
                    if let Some(note) = s.press_note() {
                        write!(f, "|{}", MidiNote(note as i32).name())?;
                    } else {
                        write!(f, "| - ")?;
                    }
                    if s.release {
                        write!(f, ".")?;
                    } else {
                        write!(f, " ")?;
                    }
                    write!(f, "{:width$}", params_string(&s), width = param_width)?;
                }
                writeln!(f, "|")?;
            }
            writeln!(f)?;
        }
    }

    write!(f, "## Settings\n\n")?;
    writeln!(f, "- {}: {}", INSTRUMENTS_FILE_SETTING, song.instruments_file)?;
    writeln!(f, "- {}: {}", FRAMES_PER_STEP_SETTING, song.frames_per_step)?;
    writeln!(f)?;

    f.flush()?;

    Ok(())
}

#[test]
fn settings() {
    let song = parse_markdown_song(
        "
## Pattern 1

## Settings

- InstrumentsFile: some_instruments.wasm
",
    )
    .unwrap();
    assert_eq!(song.instruments_file, "some_instruments.wasm");

    assert!(parse_markdown_song("## Pattern 1").is_err());
}

#[test]
fn illegal_pattern_num() {
    assert!(parse_markdown_song(
        "
## Pattern 0

## Settings

- InstrumentsFile: blah
"
    )
    .is_err());

    assert!(parse_markdown_song(
        "
## Pattern 65

## Settings

- InstrumentsFile: blah
"
    )
    .is_err());

    // Doesn't error but ignores it.
    assert!(parse_markdown_song(
        "
## Pattern -1

## Settings

- InstrumentsFile: blah
"
    )
    .is_ok());
}

#[test]
fn pattern_steps() {
    assert!(parse_markdown_song(
        "
## Pattern 1

|0  |
|---|
|-  |

## Settings

- InstrumentsFile: blah
"
    )
    .is_err());

    assert!(parse_markdown_song(
        "
## Pattern 1

|0  |
|---|
|-  |
|-  |
|-  |
|-  |
|-  |
|-  |
|-  |
|-  |
|-  |
|-  |
|-  |
|-  |
|-  |
|-  |
|-  |
|-  |
|-  |
|-  |

## Settings

- InstrumentsFile: blah
"
    )
    .is_err());
}

#[test]
fn note_parse() {
    let song = parse_markdown_song(
        "
## Pattern 1

|0  |
|---|
| - |
| - .|
|.|
|-|
|C-5|
|C#5.|
|-  |
|   |
|   |
|   |
|   |
|   |
|   |
|   |
|   |
|   |

## Settings

- InstrumentsFile: blah
",
    )
    .unwrap();
    assert_eq!(song.patterns[0].instruments[0].steps[0].press_note(), None);
    assert_eq!(song.patterns[0].instruments[0].steps[0].release, false);

    assert_eq!(song.patterns[0].instruments[0].steps[1].press_note(), None);
    assert_eq!(song.patterns[0].instruments[0].steps[1].release, true);

    assert_eq!(song.patterns[0].instruments[0].steps[2].press_note(), None);
    assert_eq!(song.patterns[0].instruments[0].steps[2].release, true);

    assert_eq!(song.patterns[0].instruments[0].steps[3].press_note(), None);
    assert_eq!(song.patterns[0].instruments[0].steps[3].release, false);

    assert_eq!(song.patterns[0].instruments[0].steps[4].press_note(), Some(72));
    assert_eq!(song.patterns[0].instruments[0].steps[4].release, false);
    assert_eq!(song.patterns[0].instruments[0].steps[4].note, 72);

    assert_eq!(song.patterns[0].instruments[0].steps[5].press_note(), Some(73));
    assert_eq!(song.patterns[0].instruments[0].steps[5].release, true);
    assert_eq!(song.patterns[0].instruments[0].steps[5].note, 73);
}

#[test]
fn param_parse() {
    let song = parse_markdown_song(
        "
## Pattern 1

|0  |
|---|
|-  |
|   |
|   |
|   |
|C-5 `1/5`|
|- `0/5`|
| `1/4`|
|- `0`|
|-.`/3` |
|`1/`|
|   |
|   |
|   |
|   |
|   |
|   |

## Settings

- InstrumentsFile: blah
",
    )
    .unwrap();
    assert_eq!(song.patterns[0].instruments[0].steps[3].press_note(), None);
    assert_eq!(song.patterns[0].instruments[0].steps[3].release, false);
    assert_eq!(song.patterns[0].instruments[0].steps[3].param0(), None);
    assert_eq!(song.patterns[0].instruments[0].steps[3].param1(), None);

    assert_eq!(song.patterns[0].instruments[0].steps[4].press_note(), Some(72));
    assert_eq!(song.patterns[0].instruments[0].steps[4].release, false);
    assert_eq!(song.patterns[0].instruments[0].steps[4].param0(), Some(1));
    assert_eq!(song.patterns[0].instruments[0].steps[4].param1(), Some(5));

    assert_eq!(song.patterns[0].instruments[0].steps[5].press_note(), None);
    assert_eq!(song.patterns[0].instruments[0].steps[5].release, false);
    assert_eq!(song.patterns[0].instruments[0].steps[5].param0(), Some(0));
    assert_eq!(song.patterns[0].instruments[0].steps[5].param1(), Some(5));

    assert_eq!(song.patterns[0].instruments[0].steps[6].press_note(), None);
    assert_eq!(song.patterns[0].instruments[0].steps[6].release, false);
    assert_eq!(song.patterns[0].instruments[0].steps[6].param0(), Some(1));
    assert_eq!(song.patterns[0].instruments[0].steps[6].param1(), Some(4));

    assert_eq!(song.patterns[0].instruments[0].steps[7].press_note(), None);
    assert_eq!(song.patterns[0].instruments[0].steps[7].release, false);
    assert_eq!(song.patterns[0].instruments[0].steps[7].param0(), Some(0));
    assert_eq!(song.patterns[0].instruments[0].steps[7].param1(), None);

    assert_eq!(song.patterns[0].instruments[0].steps[8].press_note(), None);
    assert_eq!(song.patterns[0].instruments[0].steps[8].release, true);
    assert_eq!(song.patterns[0].instruments[0].steps[8].param0(), None);
    assert_eq!(song.patterns[0].instruments[0].steps[8].param1(), Some(3));

    assert_eq!(song.patterns[0].instruments[0].steps[9].press_note(), None);
    assert_eq!(song.patterns[0].instruments[0].steps[9].release, false);
    assert_eq!(song.patterns[0].instruments[0].steps[9].param0(), Some(1));
    assert_eq!(song.patterns[0].instruments[0].steps[9].param1(), None);
}
