// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::sequencer::SequencerSong;
use crate::sound_engine::NUM_INSTRUMENTS;
use crate::sound_engine::NUM_PATTERNS;
use crate::sound_engine::NUM_STEPS;
use crate::utils::MidiNote;

use bit_set::BitSet;
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

        while let Some((event, tag_range)) = self.iter.next() {
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
                            if self.tag_stack.contains(&TableHead) {
                                if matches!(self.section, Section::Pattern(_)) {
                                    let text = &self.source[tag_range].trim();
                                    self.table_instrument_ids.push((*text).to_owned());
                                }
                            } else if self.tag_stack.contains(&TableRow) {
                                // I should read table cells through the Text event to be more resilient to stuff like
                                // inline HTML in the cells and still be able to extract the note.
                                // But the tokenizer seems to return underscore in separate Text events, and the underscore
                                // help avoiding empty table rows to be smaller on GitHub, so read the full text within
                                // the cell in the Start tag event instead for now.
                                if let Section::Pattern(pattern_idx) = self.section {
                                    let text = &self.source[tag_range].trim();
                                    let instrument_id = &self.table_instrument_ids[self.table_column.unwrap()];
                                    let mut step = &mut self.out.patterns[pattern_idx]
                                        .get_steps_mut(instrument_id, None)[self.table_row.unwrap()];
                                    if text.len() >= 3 && &text[0..3] != "___" {
                                        let MidiNote(note) = MidiNote::from_name(&text)?;
                                        step.press = true;
                                        step.note = note as u32;
                                    }
                                    if text.ends_with('.') {
                                        step.release = true;
                                    }
                                }
                            }
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
                        self.section = if let Some(pattern_cap) = pattern_re.captures(&*text) {
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
                            .captures(&*text)
                            .ok_or_else(|| format!("Invalid song pattern name: [{}]", &*text))?;
                        let parsed: usize = caps.get(1).unwrap().as_str().parse()?;
                        self.out.song_patterns.push(parsed - 1);
                    } else if self.section == Section::Settings && self.tag_stack.contains(&Item) {
                        let caps = setting_re.captures(&*text).ok_or_else(|| {
                            format!("Invalid setting format: [{}]. Should be [SettingName: Value].", &*text)
                        })?;
                        let name = caps.get(1).unwrap().as_str();
                        let value = caps.get(2).unwrap().as_str();
                        match name {
                            INSTRUMENTS_FILE_SETTING => self.out.instruments_file = value.into(),
                            other => elog!("Unknown song setting [{}], ignoring.", other),
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
            write!(f, "- [Pattern {}](#pattern-{})\n", spi + 1, spi + 1)?;
        }

        write!(f, "\n")?;
    }

    for (pi, p) in song.patterns.iter().enumerate() {
        let mut non_empty = BitSet::with_capacity(NUM_INSTRUMENTS);
        for (ii, i) in p.instruments.iter().enumerate() {
            if i.steps.iter().any(|s| s.press || s.release) {
                non_empty.insert(ii);
            }
        }

        if !non_empty.is_empty() {
            write!(f, "## Pattern {}\n\n", pi + 1)?;

            for ii in non_empty.iter() {
                let id = &p.instruments[ii].id;
                write!(f, "|{:^4}", id)?;
            }
            write!(f, "|\n")?;
            for _ in 0..non_empty.len() {
                write!(f, "|----")?;
            }
            write!(f, "|\n")?;

            for si in 0..NUM_STEPS {
                for ii in non_empty.iter() {
                    let i = &p.instruments[ii];
                    let s = i.steps[si];
                    if s.press {
                        write!(f, "|{}", MidiNote(s.note as i32).name())?;
                    } else {
                        write!(f, "|___")?;
                    }
                    if s.release {
                        write!(f, ".")?;
                    } else {
                        write!(f, " ")?;
                    }
                }
                write!(f, "|\n")?;
            }
            write!(f, "\n")?;
        }
    }

    write!(f, "## Settings\n\n")?;
    write!(f, "- {}: {}\n", INSTRUMENTS_FILE_SETTING, song.instruments_file)?;
    write!(f, "\n")?;

    f.flush()?;

    Ok(())
}

#[test]
fn settings() {
    let song = parse_markdown_song(
        "
## Pattern 1

## Settings

- InstrumentsFile: some_instruments.rhai
",
    )
    .unwrap();
    assert_eq!(song.instruments_file, "some_instruments.rhai");

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
|___|

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
|___|
|___|
|___|
|___|
|___|
|___|
|___|
|___|
|___|
|___|
|___|
|___|
|___|
|___|
|___|
|___|
|___|
|___|

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
|___|
|___.|
|.|
|   |
|C-5|
|C#5.|
|   |
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
    assert_eq!(song.patterns[0].instruments[0].steps[0].press, false);
    assert_eq!(song.patterns[0].instruments[0].steps[0].release, false);

    assert_eq!(song.patterns[0].instruments[0].steps[1].press, false);
    assert_eq!(song.patterns[0].instruments[0].steps[1].release, true);

    assert_eq!(song.patterns[0].instruments[0].steps[2].press, false);
    assert_eq!(song.patterns[0].instruments[0].steps[2].release, true);

    assert_eq!(song.patterns[0].instruments[0].steps[3].press, false);
    assert_eq!(song.patterns[0].instruments[0].steps[3].release, false);

    assert_eq!(song.patterns[0].instruments[0].steps[4].press, true);
    assert_eq!(song.patterns[0].instruments[0].steps[4].release, false);
    assert_eq!(song.patterns[0].instruments[0].steps[4].note, 72);

    assert_eq!(song.patterns[0].instruments[0].steps[5].press, true);
    assert_eq!(song.patterns[0].instruments[0].steps[5].release, true);
    assert_eq!(song.patterns[0].instruments[0].steps[5].note, 73);
}
