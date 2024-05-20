// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::GlobalEngine;
use crate::GlobalUI;
use crate::MainWindow;

use slint::ComponentHandle;

pub fn set_global_ui_handlers(window: &MainWindow) {
    let global_ui = &window.global::<GlobalUI>();

    let weak = window.as_weak();
    global_ui.on_cycle_selected_column(move |forward| {
        let handle = weak.upgrade().unwrap();
        let this = handle.global::<GlobalUI>();
        let engine = handle.global::<GlobalEngine>();

        let selected_column = this.get_selected_column();
        if forward {
            match selected_column {
                0 if engine.get_instrument_param_1().defined => this.invoke_select_column(1),
                0..=1 => this.invoke_select_column(2),
                // Don't enter the release column while in selection mode.
                2 if !this.invoke_in_selection_mode() => this.invoke_select_column(3),
                _ => (),
            };
        } else {
            match selected_column {
                3 => this.invoke_select_column(2),
                2 if engine.get_instrument_param_1().defined => this.invoke_select_column(1),
                _ if engine.get_instrument_param_0().defined => this.invoke_select_column(0),
                _ => (),
            };
        }
    });
}

#[cfg(feature = "desktop")]
pub fn set_global_utils_handlers(window: &MainWindow) {
    let global = &window.global::<crate::GlobalUtils>();

    global.on_get_midi_note_name(|note| crate::utils::MidiNote(note).name().into());
    global.on_get_midi_note_short_name(|note| crate::utils::MidiNote(note).short_name());
    global.on_to_signed_hex(|i| {
        (if i < 0 {
            format!("-{:02X}", i.abs() as i8)
        } else {
            format!("{:02X}", i as i8)
        })
        .into()
    });
}
#[cfg(not(feature = "desktop"))]
pub fn set_global_utils_handlers(_window: &MainWindow) {}
