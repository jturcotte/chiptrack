// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT
#![macro_use]

#[cfg(target_arch = "wasm32")]
extern crate web_sys;

#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into())
    }
}
#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! elog {
    ( $( $t:tt )* ) => {
        web_sys::console::error_1(&format!( $( $t )* ).into())
    }
}

#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
#[macro_export]
macro_rules! log {
    ( $( $t:tt )* ) => { {
        let text = format!( $( $t )* );
        println!("{}", &text);
        if let Some(log_window) = crate::ui::LOG_WINDOW.lock().unwrap().as_ref() {
            log_window.upgrade_in_event_loop(move |h| h.update_log_text(&text)).unwrap();
        }
    } }
}
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
#[macro_export]
macro_rules! elog {
    ( $( $t:tt )* ) => { {
        let text = format!( $( $t )* );
        eprintln!("{}", &text);
        if let Some(log_window) = crate::ui::LOG_WINDOW.lock().unwrap().as_ref() {
            log_window.upgrade_in_event_loop(move |h| h.update_log_text(&text)).unwrap();
        }
    } }
}

#[cfg(feature = "gba")]
#[macro_export]
macro_rules! log {
    ( $( $t:tt )* ) => {{
        use core::fmt::Write;
        if let Ok(mut logger) = gba::mgba::MgbaBufferedLogger::try_new(gba::mgba::MgbaMessageLevel::Info) {
            writeln!(logger, $( $t )*).ok();
        }
    }}
}
#[cfg(feature = "gba")]
#[macro_export]
macro_rules! elog {
    ( $( $t:tt )* ) => {{
        use core::fmt::Write;
        if let Ok(mut logger) = gba::mgba::MgbaBufferedLogger::try_new(gba::mgba::MgbaMessageLevel::Error) {
            writeln!(logger, $( $t )*).ok();
        }
        $crate::gba_platform::renderer::draw_error_text(&alloc::format!($( $t )*));
    }}
}
