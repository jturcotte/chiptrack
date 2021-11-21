use sixtyfps::SharedString;

pub fn midi_note_name(note: u32) -> SharedString {
    let octave = (note / 12) - 1;
    let note_names = ["C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-"];
    let mut note_name = SharedString::from(note_names[note as usize % 12]);
    note_name.push_str(&octave.to_string());
    note_name
}