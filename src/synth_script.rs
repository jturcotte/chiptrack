// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::sound_engine::NUM_INSTRUMENT_COLS;
use crate::sound_engine::NUM_INSTRUMENTS;
use crate::synth_script::wasm::WasmExecEnv;
use crate::synth_script::wasm::WasmFunction;
use crate::synth_script::wasm::WasmModule;
use crate::synth_script::wasm::WasmModuleInst;
use crate::synth_script::wasm::WasmRuntime;
use crate::utils::NOTE_FREQUENCIES;

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use alloc::rc::Rc;
use alloc::string::String;
use core::cell::Ref;
use core::cell::RefCell;
use core::ffi::CStr;
#[cfg(feature = "desktop")]
use std::fs::File;
#[cfg(feature = "desktop")]
use std::io::Write;

pub mod wasm;

fn instrument_print(s: &CStr) {
  log!("print: {}", s.to_str().expect("Invalid UTF-8"));
}

#[derive(Clone, Copy)]
struct PressedNote {
    note: u8,
    pressed_frame: usize,
    extended_frames: Option<usize>,
}

#[derive(Clone)]
struct InstrumentState {
    press_function: Option<WasmFunction>,
    release_function: Option<WasmFunction>,
    frame_function: Option<WasmFunction>,
    frames_after_release: i32,
    pressed_note: Option<PressedNote>,
}

impl Default for InstrumentState {
    fn default() -> Self {
        InstrumentState {
            press_function: None,
            release_function: None,
            frame_function: None,
            frames_after_release: 0,
            pressed_note: None,
        }
    }
}

trait InstrumentColArrayExt {
    fn get_instrument(&mut self, index: u8) -> Option<&mut InstrumentState>;
}
impl InstrumentColArrayExt for [Vec<InstrumentState>; NUM_INSTRUMENT_COLS] {
    fn get_instrument(&mut self, index: u8) -> Option<&mut InstrumentState> {
        // Column index is in the two lsb
        let col = &mut self[(index & 0x3) as usize];
        // Row index in the remaining bits
        col.get_mut((index >> 2) as usize)
    }
}

pub struct SynthScript {
    wasm_runtime: Rc<WasmRuntime>,
    wasm_exec_env: Option<WasmExecEnv>,
    instrument_ids: Rc<RefCell<Vec<String>>>,
    instrument_states: Rc<RefCell<[Vec<InstrumentState>; NUM_INSTRUMENT_COLS]>>,
}

impl SynthScript {
    const DEFAULT_INSTRUMENTS: &'static [u8] = include_bytes!("../res/default-instruments.wasm");

    pub fn new<F, G>(synth_set_sound_reg: F, synth_set_wave_table: G) -> SynthScript
        where
            F: Fn(i32, i32) + 'static,
            G: Fn(&[u8]) + 'static,
    {
        let instrument_ids: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![Default::default(); NUM_INSTRUMENTS]));
        let instrument_states: Rc<RefCell<[Vec<InstrumentState>; NUM_INSTRUMENT_COLS]>> = Default::default();
        let instrument_ids_clone = instrument_ids.clone();
        let instrument_states_clone = instrument_states.clone();

        let set_instrument_at_column = move |module: &WasmModuleInst, cid: &CStr, col: i32, frames_after_release: i32, press: &CStr, release: &CStr, frame: &CStr| -> () {
            let id = cid.to_str().unwrap();
            assert!(!id.is_empty(), "set_instrument_at_column: id must not be empty, got {:?}", id);
            assert!(!instrument_ids_clone.borrow().iter().any(|i| i == id), "set_instrument_at_column: id {} must be unique, but was already set", id);
            assert!(col >= 0 && col <= NUM_INSTRUMENT_COLS as i32,
                "set_instrument_at_column: column must be 0 <= col <= {}, got {}",
                NUM_INSTRUMENT_COLS, col);
            let mut state_cols = instrument_states_clone.borrow_mut();
            let (state, index) = {
                let state_col = &mut state_cols[col as usize];
                if state_col.len() >= 16 {
                    elog!("set_instrument_at_column: column {} already contains 16 instruments", col);
                }
                state_col.push(Default::default());
                // Column index is in the two lsb
                // 0, 1, 2, 3,
                // 4, 5, 6, 7,
                // ...
                let index = ((state_col.len() - 1) << 2) + col as usize;
                (&mut state_col.last_mut().unwrap(), index)
            };

            instrument_ids_clone.borrow_mut()[index] = id.to_owned();

            state.frames_after_release = frames_after_release;
            state.press_function = module.lookup_function(press);
            state.release_function = module.lookup_function(release);
            state.frame_function = module.lookup_function(frame);
        };

        let functions: Vec<Box<dyn wasm::HostFunction>> = vec![
            Box::new(wasm::HostFunctionS::new("print", instrument_print)),
            Box::new(wasm::HostFunctionSIISSS::new("set_instrument_at_column", set_instrument_at_column)),
            Box::new(wasm::HostFunctionII::new("gba_set_sound_reg", synth_set_sound_reg)),
            Box::new(wasm::HostFunctionA::new("gba_set_wave_table", synth_set_wave_table)),
        ];
      
        let runtime = Rc::new(WasmRuntime::new(functions).unwrap());

        SynthScript {
            wasm_runtime: runtime,
            wasm_exec_env: None,
            instrument_ids: instrument_ids,
            instrument_states: instrument_states,
        }
    }

    pub fn instrument_ids<'a>(&'a self) -> Ref<'a, Vec<String>> {
        self.instrument_ids.borrow()
    }

    #[cfg(feature = "desktop")]
    fn reset_instruments(&mut self) {
        for state_col in &mut *self.instrument_states.borrow_mut() {
            state_col.clear();
        }
        for id in &mut *self.instrument_ids.borrow_mut() {
            *id = Default::default();
        }
        self.wasm_exec_env = None;
    }

    pub fn load_default(&mut self, _frame_number: usize) {
        // FIXME: Take a slice
        let module = Rc::new(WasmModule::new(SynthScript::DEFAULT_INSTRUMENTS.to_vec(), self.wasm_runtime.clone()).unwrap());
        let module_inst = Rc::new(WasmModuleInst::new(module).unwrap());
        self.wasm_exec_env = Some(WasmExecEnv::new(module_inst).unwrap());

        // self.script_engine
        //     .compile(SynthScript::DEFAULT_INSTRUMENTS)
        //     .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        //     .and_then(|ast| {
        //         self.set_instruments_ast(ast, frame_number)
        //             .map_err(|e| e as Box<dyn std::error::Error>)
        //     })
        //     .expect("Error loading default instruments.");
    }

    #[cfg(feature = "desktop")]
    pub fn load_str(&mut self, _encoded: &str, _frame_number: usize) -> Result<(), Box<dyn std::error::Error>> {
        self.reset_instruments();

        // self.interpreter.run_code(encoded, None)?;
        // let ast = self.script_engine.compile(encoded)?;
        // self.set_instruments_ast(ast, frame_number)?;
        Ok(())
    }

    #[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
    pub fn load_file(&mut self, instruments_path: &std::path::Path, _frame_number: usize) -> Result<(), Box<dyn std::error::Error>> {
        self.reset_instruments();

        if instruments_path.exists() {
            let buffer = std::fs::read(instruments_path)?;
            let module = Rc::new(WasmModule::new(buffer, self.wasm_runtime.clone()).unwrap());
            let module_inst = Rc::new(WasmModuleInst::new(module).unwrap());
            self.wasm_exec_env = Some(WasmExecEnv::new(module_inst).unwrap());

            // let ast = self.script_engine.compile_file(instruments_path.to_path_buf())?;
            // self.interpreter.run_file(instruments_path).unwrap();
            // self.set_instruments_ast(ast, frame_number)?;
            Ok(())
        } else {
            return Err(format!("Project instruments file {:?} doesn't exist.", instruments_path).into());
        }
    }

    #[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
    pub fn save_as(&mut self, instruments_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut f = File::create(instruments_path)?;
        f.write_all(SynthScript::DEFAULT_INSTRUMENTS)?;
        f.flush()?;
        Ok(())
    }

    pub fn press_instrument_note(&mut self, frame_number: usize, instrument: u8, note: u8) -> () {
        let mut states = self.instrument_states.borrow_mut();
        if let Some(state) = states.get_instrument(instrument) {
            if let Some(f) = &state.press_function {
                state.pressed_note = Some(PressedNote {
                    note: note,
                    pressed_frame: frame_number,
                    extended_frames: None,
                });
                self.wasm_exec_env.as_ref().unwrap().call_ii(*f, Self::note_to_freq(note), note as i32).unwrap();
            }
        }
    }

    pub fn release_instrument(&mut self, frame_number: usize, instrument: u8) -> () {
        let mut states = self.instrument_states.borrow_mut();
        if let Some(state) = states.get_instrument(instrument) {
            if let Some(PressedNote {
                note,
                pressed_frame,
                extended_frames,
            }) = &mut state.pressed_note
            {
                if let Some(f) = &state.release_function {
                    self.wasm_exec_env.as_ref().unwrap().call_iii(
                        *f,
                        Self::note_to_freq(*note),
                        *note as i32,
                        (frame_number - *pressed_frame) as i32).unwrap();
                }
                // Since the release function might trigger an envelope that lasts a few
                // frames, the frame function would need to continue running during that time.
                // The "frames" function will be run as long as pressed_note is some,
                // so if the instrument has set frames_after_release, transfer that info
                // into a countdown that the frame function runner will decrease, and then
                // finally empty `pressed_note`.
                if state.frames_after_release > 0 {
                    *extended_frames = Some(state.frames_after_release as usize)
                } else {
                    state.pressed_note = None;
                }
            }
        }
    }

    pub fn advance_frame(&mut self, frame_number: usize) {
        for state_col in &mut *self.instrument_states.borrow_mut() {
            for state in state_col {
                // Only run the frame function on instruments currently pressed.
                if let (
                    Some(f),
                    Some(PressedNote {
                        note,
                        pressed_frame,
                        extended_frames,
                    }),
                ) = (&state.frame_function, &mut state.pressed_note)
                {
                    self.wasm_exec_env.as_ref().unwrap().call_iii(
                        *f,
                        Self::note_to_freq(*note),
                        *note as i32,
                        (frame_number - *pressed_frame) as i32).unwrap();
                    if let Some(remaining) = extended_frames {
                        *remaining -= 1;
                        if *remaining == 0 {
                            // Finally empty `pressed_note` to prevent further
                            // runs of the frames function.
                            state.pressed_note = None;
                        }
                    }
                }
            }
        }
    }

    // fn set_instruments_ast(
    //     &mut self,
    //     ast: AST,
    //     frame_number: usize,
    // ) -> Result<(), std::boxed::Box<rhai::EvalAltResult>> {
    //     self.script_ast = ast;

    //     // The script might also contain sound settings directly in the its root.
    //     {
    //         self.script_context.set_frame_number(frame_number);
    //         self.script_context.mark_pending_settings_as_resettable();
    //         // FIXME: Also reset the gb states somewhere like gbsplay does
    //     }

    //     let mut scope = Scope::new();
    //     scope.push("gb", self.script_context.clone());

    //     self.script_engine.run_ast_with_scope(&mut scope, &self.script_ast)
    // }

    fn note_to_freq(note: u8) -> i32 {
        // let a = 440.0; // Frequency of A
        // let key_freq = (a / 32.0) * 2.0_f64.powf((note as f64 - 9.0) / 12.0);
        // key_freq
        NOTE_FREQUENCIES[note as usize] as i32
    }
}
