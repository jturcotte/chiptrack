# Chiptrack

A cross-platform sequencer and synthesizer based on the emulation of the Game Boy sound chip.

[![image](https://user-images.githubusercontent.com/839935/173205865-e4ce44f0-75d3-4c26-8230-6d04aaa9dcdd.png)](https://jturcotte.github.io/chiptrack)

[Try the WebAssembly version.](https://jturcotte.github.io/chiptrack)

It uses:

- [Slint](https://github.com/slint-ui/slint) for the UI
- [RBoy](https://github.com/mvdnes/rboy) for the sound chip emulation
- [CPAL](https://github.com/RustAudio/cpal) for the audio output
- [Instruments are implemented in Zig](res/default-instruments.zig), compiled to WebAssembly and executed using [WAMR](https://github.com/bytecodealliance/wasm-micro-runtime)

## Install using Cargo

```bash
# On Linux you might need WAMR and CPAL local dependencies, for example on Ubuntu:
# sudo apt install build-essential cmake pkg-config libasound2-dev libxft-dev

cargo install --git https://github.com/jturcotte/chiptrack
```

## License

The source code is available under the terms of the MIT license
(See [LICENSE-MIT](LICENSE-MIT) for details).

However, because of the use of GPL dependencies, Chiptrack compiled binaries
are licensed under the terms of the GPLv3 (See [LICENSE-GPL](LICENSE-GPL)).

*"Game Boy" is registered trademark of Nintendo*
