// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use core::cell::RefCell;

use crate::sound_renderer::SoundRendererTrait;

use alloc::rc::Rc;
#[cfg(feature = "desktop")]
use slint::Model;

// By putting this here, every generated Slint type is imported into crate::ui.
slint::include_modules!();

pub fn set_window_handlers<SR: SoundRendererTrait + 'static>(window: &MainWindow, _sound_renderer: Rc<RefCell<SR>>) {
    #[cfg(feature = "gba")]
    {
        let cloned_sound_renderer = _sound_renderer.clone();
        window.on_save_to_sram(move || {
            cloned_sound_renderer
                .borrow_mut()
                .invoke_on_sound_engine(move |se| se.save_song_to_gba_sram());
        });

        window.on_clear_status_text(move || {
            crate::gba_platform::renderer::clear_status_text();
        });
    }
    let _window_weak = window.as_weak();
    #[cfg(feature = "desktop")]
    window.on_octave_increased(move |octave_delta| {
        let window = _window_weak.clone().upgrade().unwrap();
        let first_note = window.get_first_note();
        if first_note <= 24 && octave_delta < 0 || first_note >= 96 && octave_delta > 0 {
            return;
        }
        window.set_first_note(first_note + octave_delta * 12);
        let model = window.get_notes();
        for row in 0..model.row_count() {
            let mut row_data = model.row_data(row).unwrap();
            // The note_number changed and thus the sequencer release events
            // won't see that note anymore, so release it already while we're here here.
            row_data.active = false;
            model.set_row_data(row, row_data);
        }
    });
}

pub fn set_global_engine_handlers<SR: SoundRendererTrait + 'static>(
    window: &MainWindow,
    sound_renderer: Rc<RefCell<SR>>,
) {
    let global_engine = GlobalEngine::get(window);

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_display_instrument(move |instrument| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.display_instrument(instrument as u8));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_instrument(move |col_delta, row_delta| {
        // FIXME: This might need to go through the SoundEngine as with display_instrument.
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().cycle_instrument(col_delta, row_delta));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_pattern_instrument(move |forward| {
        // FIXME: This might need to go through the SoundEngine as with display_instrument.
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().cycle_pattern_instrument(forward));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_step_note_start(move |step| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_step_note_start(step as usize));
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_step_note_end(move |step| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_step_note_end(step as usize));
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_step_note(move |step, forward, large_inc| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_step_note(step as usize, forward, large_inc));
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_step_range_note(move |step_range_first, step_range_last, forward, large_inc| {
        debug_assert!(step_range_first >= 0);
        cloned_sound_renderer.borrow_mut().invoke_on_sound_engine(move |se| {
            se.cycle_step_range_note(step_range_first as usize, step_range_last as usize, forward, large_inc)
        });
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_instrument_param_start(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_instrument_param_start());
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_instrument_param_end(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_instrument_param_end());
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_instrument_param(move |param_num, forward| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_instrument_param(param_num as u8, forward));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_step_param_start(move |step, param_num| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_step_param_start(step as usize, param_num as u8));
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_step_param_end(move |step, param_num| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_step_param_end(step as usize, param_num as u8));
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_step_param(move |step, param_num, forward, large_inc| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.cycle_step_param(step as usize, param_num as u8, forward, large_inc));
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_step_range_param(
        move |step_range_first, step_range_last, param_num, forward, large_inc| {
            debug_assert!(step_range_first >= 0);
            cloned_sound_renderer.borrow_mut().invoke_on_sound_engine(move |se| {
                se.cycle_step_range_param(
                    step_range_first as usize,
                    step_range_last as usize,
                    param_num as u8,
                    forward,
                    large_inc,
                )
            });
        },
    );

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_song_pattern_start_with_new(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().cycle_song_pattern_start_with_new());
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_song_pattern_start(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().cycle_song_pattern_start());
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cycle_song_pattern(move |forward| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().cycle_song_pattern(forward));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_toggle_mute_instrument(move |instrument| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().toggle_mute_instrument(instrument as u8));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_note_pressed(move |note| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.press_note(note as u8));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_note_released(move |note| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.release_note(note as u8));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_set_erasing(move |v| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().set_erasing(v));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_toggle_step(move |step| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().toggle_step(step as usize));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_toggle_step_release(move |step| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().toggle_step_release(step as usize));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_activate_step(move |step| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().activate_step(step as usize));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_copy_step_range_note(move |step_range_first, step_range_last| {
        debug_assert!(step_range_first >= 0);
        cloned_sound_renderer.borrow_mut().invoke_on_sound_engine(move |se| {
            se.sequencer
                .borrow_mut()
                .copy_step_range_note(step_range_first as usize, step_range_last as usize)
        });
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cut_step_range_note(move |step_range_first, step_range_last| {
        debug_assert!(step_range_first >= 0);
        cloned_sound_renderer.borrow_mut().invoke_on_sound_engine(move |se| {
            se.sequencer
                .borrow_mut()
                .cut_step_range_note(step_range_first as usize, step_range_last as usize)
        });
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cut_step_single_note(move |step| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().cut_step_single_note(step as usize));
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_paste_step_range_note(move |at_step| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().paste_step_range_note(at_step as usize));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_copy_step_range_param(move |step_range_first, step_range_last, param_num| {
        debug_assert!(step_range_first >= 0);
        cloned_sound_renderer.borrow_mut().invoke_on_sound_engine(move |se| {
            se.sequencer.borrow_mut().copy_step_range_param(
                step_range_first as usize,
                step_range_last as usize,
                param_num as u8,
            )
        });
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cut_step_range_param(move |step_range_first, step_range_last, param_num| {
        debug_assert!(step_range_first >= 0);
        cloned_sound_renderer.borrow_mut().invoke_on_sound_engine(move |se| {
            se.sequencer.borrow_mut().cut_step_range_param(
                step_range_first as usize,
                step_range_last as usize,
                param_num as u8,
            )
        });
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_cut_step_single_param(move |step, param_num| {
        cloned_sound_renderer.borrow_mut().invoke_on_sound_engine(move |se| {
            se.sequencer
                .borrow_mut()
                .cut_step_single_param(step as usize, param_num as u8)
        });
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_paste_step_range_param(move |at_step, param_num| {
        cloned_sound_renderer.borrow_mut().invoke_on_sound_engine(move |se| {
            se.sequencer
                .borrow_mut()
                .paste_step_range_param(at_step as usize, param_num as u8)
        });
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_toggle_step_release(move |step| {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().toggle_step_release(step as usize));
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_set_playing(move |playing, song_mode| {
        // FIXME: Stop the sound device
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.set_playing(playing, song_mode));
    });

    #[cfg(feature = "desktop")]
    {
        let cloned_sound_renderer = sound_renderer.clone();
        let window_weak = window.as_weak();
        global_engine.on_record_clicked(move |toggled| {
            cloned_sound_renderer
                .borrow_mut()
                .invoke_on_sound_engine(move |se| se.sequencer.borrow_mut().set_recording(toggled));
            window_weak.unwrap().global::<GlobalUI>().set_recording(toggled);
        });
    }

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_display_song_pattern(move |song_pattern_idx| {
        cloned_sound_renderer.borrow_mut().invoke_on_sound_engine(move |se| {
            se.sequencer
                .borrow_mut()
                .display_song_pattern(song_pattern_idx as usize)
        });
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_display_song_pattern_with_nearest_instrument(move |song_pattern_idx| {
        cloned_sound_renderer.borrow_mut().invoke_on_sound_engine(move |se| {
            se.sequencer
                .borrow_mut()
                .display_song_pattern_with_nearest_instrument(song_pattern_idx as usize)
        });
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_remove_last_song_pattern(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(|se| se.sequencer.borrow_mut().remove_last_song_pattern());
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_clone_displayed_song_pattern(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(|se| se.sequencer.borrow_mut().clone_displayed_song_pattern());
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_activate_song_pattern(move |song_pattern_idx| {
        cloned_sound_renderer.borrow_mut().invoke_on_sound_engine(move |se| {
            se.sequencer
                .borrow_mut()
                .activate_song_pattern(song_pattern_idx as usize)
        });
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_save_project(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(|se| se.save_project())
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_export_project_as_gba_sav(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(|se| se.export_project_as_gba_sav())
    });

    let window_weak = window.as_weak();
    let mut maybe_previous_phasing: Option<f32> = None;
    #[cfg(feature = "desktop")]
    global_engine.on_phase_visualization_tick(move |animation_ms| {
        // 4194304 Hz / 70224 Hz per frame = ~59.7 frames per second
        let animation_synth_tick = animation_ms * (4194304.0 / 70224.0) / 1000.0;
        let window = window_weak.clone().upgrade().unwrap();
        let last_synth_tick = GlobalEngine::get(&window).get_last_synth_tick();
        let cur_phasing = last_synth_tick as f32 - animation_synth_tick;

        // The phasing will fluctuate depending on the timing of the sound and rendering loops,
        // which would cause non-smooth scrolling if we'd rely on it for timing, so fix the phasing.
        // But the refresh rates are slightly different, and thus the two drift apart slowly by one
        // frame every few minutes, so we also need to slowly bring them back together from time to
        // time to keep the timelines in sync.
        let phasing = match maybe_previous_phasing {
            // If the rendering and sound synthesis diverged too much, snap them back
            Some(previous_phasing) if (previous_phasing - cur_phasing).abs() > 60.0 => {
                maybe_previous_phasing = Some(cur_phasing);
                cur_phasing
            }
            // If they diverged a little, bring them back little by little
            Some(previous_phasing) if previous_phasing - cur_phasing > 2.0 => {
                let new_phasing = previous_phasing - 0.5;
                maybe_previous_phasing = Some(new_phasing);
                new_phasing
            }
            Some(previous_phasing) if cur_phasing - previous_phasing > 2.0 => {
                let new_phasing = previous_phasing + 0.5;
                maybe_previous_phasing = Some(new_phasing);
                new_phasing
            }
            // If it only diverged within the fluctuation range, keep the current phasing.
            Some(previous_phasing) => previous_phasing,
            // Still uninitialized
            None => {
                // Don't set in stone until the sound engine is ready.
                if last_synth_tick != -1 {
                    maybe_previous_phasing = Some(cur_phasing);
                }
                cur_phasing
            }
        };

        animation_synth_tick + phasing
    });

    let cloned_sound_renderer = sound_renderer.clone();
    global_engine.on_mute_instruments(move || {
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(|se| se.mute_instruments());
    });
}

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

pub fn set_global_settings_handlers<SR: SoundRendererTrait + 'static>(
    window: &MainWindow,
    sound_renderer: Rc<RefCell<SR>>,
) {
    let global_settings = GlobalSettings::get(window);

    let cloned_sound_renderer = sound_renderer.clone();
    global_settings.on_settings_changed(move |settings| {
        log!("SET {:?}", settings);
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.apply_settings(&settings));
    });
    let cloned_sound_renderer = sound_renderer.clone();
    global_settings.on_song_settings_changed(move |settings| {
        log!("SET {:?}", settings);
        cloned_sound_renderer
            .borrow_mut()
            .invoke_on_sound_engine(move |se| se.apply_song_settings(&settings));
    });
}

#[cfg(feature = "desktop")]
pub fn set_global_utils_handlers(window: &MainWindow) {
    let global = &window.global::<GlobalUtils>();

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
