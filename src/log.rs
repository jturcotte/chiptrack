// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

#![macro_use]

#[cfg(target_arch = "wasm32")]
extern crate web_sys;

#[cfg(target_arch = "wasm32")]
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into())
    }
}
#[cfg(target_arch = "wasm32")]
macro_rules! elog {
    ( $( $t:tt )* ) => {
        web_sys::console::error_1(&format!( $( $t )* ).into())
    }
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
macro_rules! log {
    ( $( $t:tt )* ) => {
        println!( $( $t )* )
    }
}
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
macro_rules! elog {
    ( $( $t:tt )* ) => {
        eprintln!( $( $t )* )
    }
}

#[cfg(not(feature = "std"))]
macro_rules! log {
    ( $( $t:tt )* ) => {
    }
}
#[cfg(not(feature = "std"))]
macro_rules! elog {
    ( $( $t:tt )* ) => {
        ()
    }
}
