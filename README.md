# Chiptrack

A cross-platform sequencer and synthesizer based on the emulation of the Game Boy sound chip.

[![image](https://user-images.githubusercontent.com/839935/173205865-e4ce44f0-75d3-4c26-8230-6d04aaa9dcdd.png)](https://jturcotte.github.io/chiptrack)

[Try the WebAssembly version.](https://jturcotte.github.io/chiptrack)

It uses:

- [Slint](https://github.com/slint-ui/slint) for the UI
- [RBoy](https://github.com/mvdnes/rboy) for the sound chip emulation
- [CPAL](https://github.com/RustAudio/cpal) for the audio output
- [Rhai](https://github.com/rhaiscript/rhai) for the [instruments scripting](res/default-instruments.rhai)

## Install using Cargo

```bash
# On Linux you might need CPAL local dependencies:
# sudo apt install libasound2-dev
# OR
# sudo yum install alsa-lib-devel

cargo install --git https://github.com/jturcotte/chiptrack
```

## License

The source code is available under the terms of the MIT license
(See [LICENSE-MIT](LICENSE-MIT) for details).

However, because of the use of GPL dependencies, Chiptrack compiled binaries
are licensed under the terms of the GPLv3 (See [LICENSE-GPL](LICENSE-GPL)).

*"Game Boy" is registered trademark of Nintendo*
