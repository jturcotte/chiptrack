# Chiptrack

A programmable cross-platform sequencer for the Game Boy Advance sound chip.

[![image](https://github.com/jturcotte/chiptrack/assets/839935/187a2ce6-072f-43d6-9937-2c7579562908)](https://jturcotte.github.io/chiptrack)

[Try the Web Player](https://jturcotte.github.io/chiptrack)

## Install using Cargo

```bash
# On Linux you might need WAMR and CPAL local dependencies, for example on Ubuntu:
# sudo apt install build-essential cmake pkg-config libasound2-dev libxft-dev

cargo install --git https://github.com/jturcotte/chiptrack
```

## Features
- [Runs natively on the Game Boy Advance](#runs-natively-on-the-game-boy-advance)
- [Instruments are programmable](#instruments-are-programmable)
- [Songs can be distributed and played from GitHub gists](#songs-can-be-distributed-and-played-from-github-gists)
- [Basic MIDI support in the desktop version](#basic-midi-support-in-the-desktop-version)

### Runs natively on the Game Boy Advance

![image](https://github.com/jturcotte/chiptrack/assets/839935/e358fc48-d26b-46e2-9d37-58d40fa94877)

The built-in sound chip is used for sound production in this case.
The desktop and Web versions will produce the sound in software by emulating sound register commands.

Slint models are used as cross-platform abstraction, read from a [custom renderer](src/gba_platform/renderer.rs)
to take advantage of the GBA's hardware acceleration.

### Instruments are programmable

```zig
pub fn press(freq: u32, note: u8, param0: i8, param1: i8) callconv(.C) void {
    _ = note; _ = param0; _ = param1;
    gba.EnvDutyLen
        .withDuty(gba.dut_1_8)
        .withEnvStart(10)
        .writeTo(gba.square1);
    gba.CtrlFreq.init()
        .withTrigger(1)
        .withSquareFreq(freq)
        .writeTo(gba.square1);
}
```

Each song carries a little WebAssembly program that converts sequenced notes to Game Boy Advance sound
commands. **This gives almost complete control over the sound chip to each song.**

[Default instruments are provided for empty projects](instruments/default-instruments.zig) and can be customized.

### Songs can be distributed and played from GitHub gists

```md
## Pattern 16

| S1 | T2 | W2 | N1 | N2 | N3 |
|----|----|----|----|----|----|
|A-4 |A-2 |B-3 | -  |C-2.| -  |
| -  | -  | -  | -  | -  | -  |
|A-4 | -  |C#4 |C-2.| -  | -  |
| -  | -  | -  | -  | -  | -  |
|A-4 |A-2 |E-4 |C-2.| -  | -  |
...
```

Songs are saved as Markdown and are human-readable and can be discovered by searching by using GitHub's search: https://gist.github.com/search?q=%23chiptrack

### Basic MIDI support in the desktop version

An external MIDI keyboard can be used to play or record notes.

## Key / Button Mapping

Function | Desktop | Game Boy Advance
---------|---------|-----------------
Move cursor | <kbd>&#8592;</kbd>\|<kbd>&#8593;</kbd>\|<kbd>&#8594;</kbd>\|<kbd>&#8595;</kbd> | <kbd>&#8592;</kbd>\|<kbd>&#8593;</kbd>\|<kbd>&#8594;</kbd>\|<kbd>&#8595;</kbd>
Switch panel (Patterns, Steps, Instruments) | <kbd>Shift</kbd> + (<kbd>&#8592;</kbd>\|<kbd>&#8594;</kbd>) | <kbd>R</kbd> + (<kbd>&#8592;</kbd>\|<kbd>&#8594;</kbd>)
Select previous/next song pattern | <kbd>B</kbd> + (<kbd>&#8593;</kbd>/<kbd>&#8595;</kbd>) | <kbd>B</kbd> + (<kbd>&#8593;</kbd>/<kbd>&#8595;</kbd>)
Select previous/next pattern non-empty instruments | <kbd>Z</kbd> + (<kbd>&#8592;</kbd>\/<kbd>&#8594;</kbd>) | <kbd>B</kbd> + (<kbd>&#8592;</kbd>/<kbd>&#8594;</kbd>)
Cycle the selected pattern/note/param value | <kbd>X</kbd> + (<kbd>&#8592;</kbd>\|<kbd>&#8594;</kbd>) | <kbd>A</kbd> + (<kbd>&#8592;</kbd>\|<kbd>&#8594;</kbd>)
Copy | <kbd>X</kbd>  | <kbd>A</kbd>
Cut | <kbd>Z</kbd> + <kbd>X</kbd>  | <kbd>B</kbd> + <kbd>A</kbd>
Paste (on empty slot) | <kbd>X</kbd>  | <kbd>A</kbd>
Play song | <kbd>Enter</kbd> | <kbd>Start</kbd>
Play pattern | <kbd>Ctrl</kbd> + <kbd>Enter</kbd> | <kbd>Select</kbd> + <kbd>Start</kbd>
Reset sound channels | <kbd>Esc</kbd>  | <kbd>Select</kbd>
Save | <kbd>Ctrl</kbd> + <kbd>S</kbd> | <kbd>L</kbd> + <kbd>Start</kbd>
Export song to GBA save file | <kbd>Ctrl</kbd> + <kbd>G</kbd> | N/A
Toggle recording mode | <kbd>.</kbd> | N/A
Black notes | <kbd>W</kbd>\|<kbd>E</kbd>\|<kbd>T</kbd>\|<kbd>Y</kbd><kbd>U</kbd> | N/A
White notes | <kbd>A</kbd>\|<kbd>S</kbd>\|<kbd>D</kbd>\|<kbd>F</kbd>\|<kbd>G</kbd>\|<kbd>H</kbd>\|<kbd>J</kbd>\|<kbd>K</kbd> | N/A
Erase step (or hold during playback) | <kbd>Backspace</kbd> | N/A


### Song Patterns Panel

Function | Desktop | Game Boy Advance
---------|---------|-----------------
Cycle pattern | <kbd>X</kbd> + (<kbd>&#8592;</kbd>\|<kbd>&#8594;</kbd>) | <kbd>A</kbd> + (<kbd>&#8592;</kbd>\|<kbd>&#8594;</kbd>)
Duplicate pattern | <kbd>Shift</kbd> + (<kbd>Z</kbd>, <kbd>X</kbd>)  | <kbd>R</kbd> + (<kbd>B</kbd>, <kbd>A</kbd>)
Copy | <kbd>X</kbd>  | <kbd>A</kbd>
Cut (only on the last non-empty slot ) | <kbd>Z</kbd> + <kbd>X</kbd>  | <kbd>B</kbd> + <kbd>A</kbd>
Paste (only on the placeholder slot) | <kbd>X</kbd>  | <kbd>A</kbd>
Insert an empty pattern instead of pasting | <kbd>X</kbd>, <kbd>X</kbd>  | <kbd>A</kbd>, <kbd>A</kbd>

### Pattern Steps Panel

Function | Desktop | Game Boy Advance
---------|---------|-----------------
Cycle note/param | <kbd>X</kbd> + (<kbd>&#8592;</kbd>\|<kbd>&#8594;</kbd>) | <kbd>A</kbd> + (<kbd>&#8592;</kbd>\|<kbd>&#8594;</kbd>)
Cycle note/param (large amount) | <kbd>X</kbd> + (<kbd>&#8595;</kbd>\|<kbd>&#8593;</kbd>) | <kbd>A</kbd> + (<kbd>&#8595;</kbd>\|<kbd>&#8593;</kbd>)
Enter selection mode | <kbd>Shift</kbd> + <kbd>Z</kbd> | <kbd>R</kbd> + <kbd>B</kbd>
Select all rows (in selection) | <kbd>Shift</kbd> + <kbd>Z</kbd> | <kbd>R</kbd> + <kbd>B</kbd>
Copy (in selection) | <kbd>Z</kbd>  | <kbd>B</kbd>
Cut (in or not in selection) | <kbd>Z</kbd> + <kbd>X</kbd>  | <kbd>B</kbd> + <kbd>A</kbd>
Cancel selection | <kbd>Shift</kbd> | <kbd>R</kbd>
Paste selection clipboard | <kbd>Shift</kbd> + <kbd>X</kbd>  | <kbd>R</kbd> + <kbd>A</kbd>
Paste edit clipboard (on empty slot) | <kbd>X</kbd>  | <kbd>A</kbd>

Notes:
- <kbd>B</kbd> + <kbd>A</kbd> means that <kbd>B</kbd> must be held first*
- The selection clipboard is set when copy/cutting in selection mode
- The edit clipboard is set after a note/param cycle (also if unchanged)
- Cutting when not in selection mode sets both the selection and edit clipboards

## Based on the awesome work of

- [Slint - A Rust UI toolkit](https://github.com/slint-ui/slint)
- [RBoy - A Gameboy Color Emulator](https://github.com/mvdnes/rboy)
- [gba - A crate for GBA development](https://github.com/rust-console/gba)

## License

The source code is available under the terms of the MIT license
(See [LICENSE-MIT](LICENSE-MIT) for details).

However, because of the use of GPL dependencies, Chiptrack compiled binaries
are licensed under the terms of the GPLv3 (See [LICENSE-GPL](LICENSE-GPL)).

*"Game Boy Advance" is registered trademark of Nintendo*
