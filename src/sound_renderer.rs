// Copyright © 2023 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

#[cfg(feature = "desktop")]
pub mod emulated;
#[cfg(feature = "desktop")]
pub use emulated::{new_sound_renderer, SoundRenderer, Synth};

#[cfg(feature = "gba")]
pub mod gba_sound;
#[cfg(feature = "gba")]
pub use gba_sound::{new_sound_renderer, SoundRenderer, Synth};
