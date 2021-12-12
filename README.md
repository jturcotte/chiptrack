# chiptrack

chiptrack is a cross-platform sequencer that internally uses a Game Boy emulator to synthesize the sound.

It uses:

- [SixtyFPS](https://github.com/sixtyfpsui/sixtyfps) for the UI.
- [RBoy](https://github.com/mvdnes/rboy) for the sound synthesizer
- [CPAL](https://github.com/RustAudio/cpal) for the audio output
- [Rhai](https://github.com/rhaiscript/rhai) for the [instruments scripting](res/instruments.rhai)

"Game Boy" is registered trade mark of Nintendo

## License

The source code is available under the terms of the MIT license
(See [LICENSE-MIT](LICENSE-MIT) for details).

However, because of the use of GPL dependencies, chiptrack compiled binaries
are licensed under the terms of the GPLv3 (See [LICENSE-GPL](LICENSE-GPL)).
