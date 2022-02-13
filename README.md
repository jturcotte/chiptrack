# chiptrack

chiptrack is a cross-platform sequencer that internally uses a Game Boy emulator to synthesize the sound.

It uses:

- [Slint](https://github.com/slint-ui/slint) for the UI.
- [RBoy](https://github.com/mvdnes/rboy) for the sound synthesizer
- [CPAL](https://github.com/RustAudio/cpal) for the audio output
- [Rhai](https://github.com/rhaiscript/rhai) for the [instruments scripting](res/default-instruments.rhai)

![image](https://user-images.githubusercontent.com/839935/145720892-27b514ab-c255-40ff-933d-da44df1650d8.png)

[Try the WebAssembly version.](https://jturcotte.github.io/chiptrack)

## Build and run

```bash
# On Linux you might need CPAL local dependencies:
# sudo apt install libasound2-dev
# OR
# sudo yum install alsa-lib-devel
git submodule update --init
SLINT_NO_QT=1 cargo run --release
```

## License

The source code is available under the terms of the MIT license
(See [LICENSE-MIT](LICENSE-MIT) for details).

However, because of the use of GPL dependencies, chiptrack compiled binaries
are licensed under the terms of the GPLv3 (See [LICENSE-GPL](LICENSE-GPL)).

*"Game Boy" is registered trademark of Nintendo*
