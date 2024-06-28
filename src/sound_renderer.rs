// Copyright © 2023 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

#[cfg(feature = "desktop")]
pub mod emulated;
#[cfg(feature = "desktop")]
use std::sync::mpsc::Sender;

#[cfg(feature = "desktop")]
pub use emulated::{new_sound_renderer, Context, SoundRenderer, Synth};

#[cfg(feature = "gba")]
pub mod gba_sound;
#[cfg(feature = "gba")]
pub use gba_sound::{new_sound_renderer, SoundRenderer, Synth};

use crate::sound_engine::SoundEngine;

pub trait SoundRendererTrait {
    fn invoke_on_sound_engine<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SoundEngine) + Send + 'static;

    fn force(&mut self);

    #[cfg(feature = "desktop")]
    fn sender(&self) -> Sender<Box<dyn FnOnce(&mut SoundEngine) + Send>>;
}
