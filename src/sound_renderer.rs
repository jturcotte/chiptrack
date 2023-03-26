// Copyright © 2023 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

#[cfg(feature = "std")]
pub mod emulated;
#[cfg(feature = "std")]
pub use emulated::{
    SoundRenderer, Synth, new_sound_renderer,
};

