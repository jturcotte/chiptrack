// Copyright © 2021 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

use crate::log;
use crate::sequencer::InstrumentParamDef;
use crate::sound_engine::NUM_INSTRUMENTS;
use crate::sound_engine::NUM_INSTRUMENT_COLS;
use crate::sound_engine::NUM_INSTRUMENT_PARAMS;
use crate::synth_script::wasm::WasmIndirectFunction;
use crate::synth_script::wasm::WasmModule;
use crate::synth_script::wasm::WasmModuleInst;
use crate::synth_script::wasm::WasmRuntime;
use crate::utils::NOTE_FREQUENCIES;

use slint::SharedString;

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::ffi::CStr;
#[cfg(feature = "desktop")]
use std::fs::File;
#[cfg(feature = "desktop")]
use std::io::Write;

pub mod wasm;
#[cfg(not(feature = "desktop_web"))]
pub mod wasm_host;
#[cfg(feature = "desktop_web")]
pub mod wasm_web;

fn instrument_print(s: &CStr) {
    log!("print: {}", s.to_str().expect("Invalid UTF-8"));
}

#[derive(Clone, Copy)]
struct PressedNote {
    note: u8,
    pressed_frame: usize,
    extended_frames: Option<usize>,
}

#[derive(Clone, Default)]
struct InstrumentState {
    press_function: WasmIndirectFunction,
    release_function: WasmIndirectFunction,
    frame_function: WasmIndirectFunction,
    set_param_functions: [WasmIndirectFunction; NUM_INSTRUMENT_PARAMS],
    frames_after_release: i32,
    pressed_note: Option<PressedNote>,
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

/// Accumulates instrument definitions during loading, then apply them to the sequencer.
#[derive(Default, Clone)]
pub struct SequencerInstrumentDef {
    pub ids: Vec<SharedString>,
    pub params: Vec<[Option<InstrumentParamDef>; NUM_INSTRUMENT_PARAMS]>,
}

pub struct SynthScript {
    wasm_runtime: Rc<WasmRuntime>,
    wasm_module_inst: Option<WasmModuleInst>,
    sequencer_instrument_def: Rc<RefCell<SequencerInstrumentDef>>,
    instrument_states: Rc<RefCell<[Vec<InstrumentState>; NUM_INSTRUMENT_COLS]>>,
    apply_instrument_def_callback: Rc<dyn Fn(SequencerInstrumentDef)>,
}

impl SynthScript {
    #[cfg(feature = "desktop")]
    pub const DEFAULT_INSTRUMENTS_TEXT: &'static [u8] = include_bytes!("../instruments/default-instruments.wat");
    #[cfg(feature = "desktop")]
    pub const DEFAULT_INSTRUMENTS_ZIG: &'static [u8] = include_bytes!("../instruments/default-instruments.zig");
    #[cfg(feature = "desktop")]
    pub const DEFAULT_INSTRUMENTS_BUILD: &'static [u8] = include_bytes!("../instruments/build.zig");
    // Built by build.rs
    #[cfg(feature = "gba")]
    pub const DEFAULT_INSTRUMENTS: &'static [u8] =
        include_bytes!(concat!(env!("OUT_DIR"), "/default-instruments.wasm"));

    pub fn new<F, G, H>(synth_set_sound_reg: F, synth_set_wave_table: G, apply_instrument_def: H) -> SynthScript
    where
        F: Fn(i32, i32) + 'static,
        G: Fn(&[u8]) + 'static,
        H: Fn(SequencerInstrumentDef) + 'static,
    {
        let sequencer_instrument_def: Rc<RefCell<SequencerInstrumentDef>> =
            Rc::new(RefCell::new(SequencerInstrumentDef::default()));
        let instrument_states: Rc<RefCell<[Vec<InstrumentState>; NUM_INSTRUMENT_COLS]>> = Default::default();

        let sequencer_instrument_def_clone = sequencer_instrument_def.clone();
        let instrument_states_clone = instrument_states.clone();
        let set_instrument_at_column = move |cid: &CStr,
                                             col: i32,
                                             frames_after_release: i32,
                                             press: WasmIndirectFunction,
                                             release: WasmIndirectFunction,
                                             frame: WasmIndirectFunction|
              -> i32 {
            let id = cid.to_str().unwrap();
            log!(
                "Setting instrument [{}] with press: {}, release: {}, frame: {}, f_a_r: {}",
                id,
                press.is_defined(),
                release.is_defined(),
                frame.is_defined(),
                frames_after_release
            );

            if id.is_empty() {
                elog!(
                    "set_instrument_at_column: id must not be empty, got {:?}. Ignoring instrument.",
                    id
                );
                return 255;
            }
            if sequencer_instrument_def_clone.borrow().ids.is_empty() {
                elog!("set_instrument_at_column: can only be called during start/main. Ignoring instrument.");
                return 255;
            }
            if sequencer_instrument_def_clone.borrow().ids.iter().any(|i| i == id) {
                elog!(
                    "set_instrument_at_column: id {} must be unique, but was already set. Ignoring instrument.",
                    id
                );
                return 255;
            }
            if !(col >= 0 && col < NUM_INSTRUMENT_COLS as i32) {
                elog!(
                    "set_instrument_at_column: column must be 0 <= col < {}, got {}. Ignoring instrument.",
                    NUM_INSTRUMENT_COLS,
                    col
                );
                return 255;
            }

            let mut state_cols = instrument_states_clone.borrow_mut();
            let (state, index) = {
                let state_col = &mut state_cols[col as usize];
                if state_col.len() >= 16 {
                    elog!(
                        "set_instrument_at_column: column {} already contains 16 instruments. Ignoring instrument.",
                        col
                    );
                    return 255;
                }
                state_col.push(Default::default());
                // Column index is in the two lsb
                // 0, 1, 2, 3,
                // 4, 5, 6, 7,
                // ...
                let index = ((state_col.len() - 1) << 2) + col as usize;
                (&mut state_col.last_mut().unwrap(), index)
            };

            sequencer_instrument_def_clone.borrow_mut().ids[index] = id.into();

            state.frames_after_release = frames_after_release;
            state.press_function = press;
            state.release_function = release;
            state.frame_function = frame;

            // Return the index to the script as the instrument handle to attach parameter definitions.
            index as i32
        };

        let sequencer_instrument_def_clone = sequencer_instrument_def.clone();
        let instrument_states_clone = instrument_states.clone();
        let define_param = move |instrument: i32,
                                 param_num: i32,
                                 cname: &CStr,
                                 default: i32,
                                 min: i32,
                                 max: i32,
                                 set_param: WasmIndirectFunction| {
            let name = cname.to_str().unwrap();
            log!(
                "Defining param {} for instrument [{}]: {} [{}, {}, {}] set_param: {}",
                param_num,
                instrument,
                name,
                default,
                min,
                max,
                set_param.is_defined()
            );
            if name.is_empty() {
                elog!(
                    "define_param: name must not be empty, got {:?}. Ignoring parameter.",
                    name
                );
                return;
            }
            if instrument < 0 || instrument >= NUM_INSTRUMENTS as i32 {
                elog!(
                    "define_param: instrument must be 0 <= instrument < {}, got {}. Ignoring parameter.",
                    NUM_INSTRUMENTS,
                    instrument
                );
                return;
            }
            if param_num < 0 || param_num >= NUM_INSTRUMENT_PARAMS as i32 {
                elog!(
                    "define_param: param_num must be 0 <= param_num < {}, got {}. Ignoring parameter.",
                    NUM_INSTRUMENT_PARAMS,
                    param_num
                );
                return;
            }

            // Part of the arguments will be used by the sequencer, accumulate those first and they'll be moved after main returned.
            let mut instruments = sequencer_instrument_def_clone.borrow_mut();
            let param = &mut instruments.params[instrument as usize][param_num as usize];
            param.replace(InstrumentParamDef {
                name: name[0..2].into(),
                default: default as i8,
                min: min as i8,
                max: max as i8,
            });

            // set_param stays in the synth_script
            let mut states = instrument_states_clone.borrow_mut();
            if let Some(state) = states.get_instrument(instrument as u8) {
                state.set_param_functions[param_num as usize] = set_param;
            } else {
                elog!("define_param: instrument {} not found. Ignoring parameter.", instrument);
            }
        };

        let functions: Vec<Box<dyn wasm::HostFunction>> = vec![
            Box::new(wasm::HostFunctionS::new("print", instrument_print)),
            Box::new(wasm::HostFunctionSIINNN::new(
                "set_instrument_at_column",
                set_instrument_at_column,
            )),
            Box::new(wasm::HostFunctionIISIIIN::new("define_param", define_param)),
            Box::new(wasm::HostFunctionII::new("gba_set_sound_reg", synth_set_sound_reg)),
            Box::new(wasm::HostFunctionA::new("gba_set_wave_table", synth_set_wave_table)),
        ];

        let runtime = Rc::new(WasmRuntime::new(functions).unwrap());

        SynthScript {
            wasm_runtime: runtime,
            wasm_module_inst: None,
            sequencer_instrument_def,
            instrument_states,
            apply_instrument_def_callback: Rc::new(apply_instrument_def),
        }
    }

    fn reset_instruments(&mut self) {
        for state_col in &mut *self.instrument_states.borrow_mut() {
            state_col.clear();
        }
        self.wasm_module_inst = None;
    }

    pub fn load_default(&mut self) -> Result<(), String> {
        #[cfg(feature = "gba")]
        return self.load_bytes(SynthScript::DEFAULT_INSTRUMENTS.to_vec());
        #[cfg(not(feature = "gba"))]
        return self.load_wasm_or_wat_bytes(SynthScript::DEFAULT_INSTRUMENTS_TEXT);
    }

    pub fn load_bytes(&mut self, encoded: Vec<u8>) -> Result<(), String> {
        self.reset_instruments();
        // instrument_ids is only valid during loading.
        *self.sequencer_instrument_def.borrow_mut() = SequencerInstrumentDef {
            ids: vec![Default::default(); NUM_INSTRUMENTS],
            params: vec![Default::default(); NUM_INSTRUMENTS],
        };
        let callback = self.apply_instrument_def_callback.clone();
        let instrument_def = self.sequencer_instrument_def.clone();

        let module = Rc::new(WasmModule::new(encoded, self.wasm_runtime.clone())?);
        self.wasm_module_inst = Some(WasmModuleInst::new(module, move || callback(instrument_def.take()))?);

        Ok(())
    }

    /// Loading binary from a gist doesn't work well with the binary downloaded as a UTF-8 string.
    /// Uploading a binary also only seems to work using the git interface and not using the gist
    /// web page and GH CLI.
    /// So support converting WAT to binary WASM directly using the wat crate if the song file
    /// references either format.
    /// For a gist it's most likely that only WAT will work.
    #[cfg(feature = "desktop")]
    pub fn load_wasm_or_wat_bytes(&mut self, wasm_or_wat: &[u8]) -> Result<(), String> {
        let cow = wat::parse_bytes(&wasm_or_wat).map_err(|e| e.to_string())?;
        self.load_bytes(cow.to_vec())
    }

    #[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
    pub fn load_file(&mut self, instruments_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        self.reset_instruments();

        if instruments_path.exists() {
            let buffer = std::fs::read(instruments_path)?;
            Ok(self.load_wasm_or_wat_bytes(&buffer)?)
        } else {
            Err(format!("Project instruments file {:?} doesn't exist.", instruments_path).into())
        }
    }

    #[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
    pub fn save_default_instruments_as(
        &mut self,
        instruments_dir: &std::path::Path,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let instruments_path = instruments_dir.join("instruments.wat");
        // Also save the zig files for the user to edit and rebuild, but just print a warning if they already exist.
        let instruments_zig_path = instruments_dir.join("instruments.zig");
        let build_zig_path = instruments_dir.join("build.zig");

        let files = [
            (instruments_path.clone(), SynthScript::DEFAULT_INSTRUMENTS_TEXT),
            (instruments_zig_path.clone(), SynthScript::DEFAULT_INSTRUMENTS_ZIG),
            (build_zig_path.clone(), SynthScript::DEFAULT_INSTRUMENTS_BUILD),
        ];

        for (path, content) in files.iter() {
            let f = File::options().write(true).create_new(true).open(path);
            match f {
                Ok(mut file) => {
                    file.write_all(content)?;
                    file.flush()?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    elog!("File already exists: {:?}, not overwriting. If those are not default instruments anymore the instruments might not match.", path);
                }
                Err(e) => return Err(e.into()),
            }
        }

        Ok(instruments_path)
    }

    pub fn press_instrument_note(&mut self, frame_number: usize, instrument: u8, note: u8, param0: i8, param1: i8) {
        let mut states = self.instrument_states.borrow_mut();
        if let Some(state) = states.get_instrument(instrument) {
            state.pressed_note = Some(PressedNote {
                note,
                pressed_frame: frame_number,
                extended_frames: None,
            });
            if state.press_function.is_defined() {
                if let Some(wasm_module_inst) = &self.wasm_module_inst {
                    if let Err(e) = wasm_module_inst.call_indirect_iiii(
                        &state.press_function,
                        Self::note_to_freq(note),
                        note as i32,
                        param0 as i32,
                        param1 as i32,
                    ) {
                        elog!("press: {:?}", e);
                    }
                }
            }
        }
    }

    pub fn release_instrument(&mut self, frame_number: usize, instrument: u8) {
        let mut states = self.instrument_states.borrow_mut();
        if let Some(state) = states.get_instrument(instrument) {
            if let Some(PressedNote {
                note,
                pressed_frame,
                extended_frames,
            }) = &mut state.pressed_note
            {
                if state.release_function.is_defined() {
                    if let Some(wasm_module_inst) = &self.wasm_module_inst {
                        if let Err(e) = wasm_module_inst.call_indirect_iii(
                            &state.release_function,
                            Self::note_to_freq(*note),
                            *note as i32,
                            (frame_number - *pressed_frame) as i32,
                        ) {
                            elog!("release: {:?}", e);
                        }
                    }
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

    pub fn set_instrument_param(&mut self, instrument: u8, param_num: u8, val: i8) {
        let mut states = self.instrument_states.borrow_mut();
        if let Some(state) = states.get_instrument(instrument) {
            let function = &state.set_param_functions[param_num as usize];
            if function.is_defined() {
                if let Err(e) = self
                    .wasm_module_inst
                    .as_ref()
                    .unwrap()
                    .call_indirect_i(&function, val as i32)
                {
                    elog!("set_param: {:?}", e);
                }
            }
        }
    }

    pub fn instrument_has_set_param_fn(&mut self, instrument: u8, param_num: u8) -> bool {
        let mut states = self.instrument_states.borrow_mut();
        states
            .get_instrument(instrument)
            .map_or(false, |s| s.set_param_functions[param_num as usize].is_defined())
    }

    pub fn release_instruments(&mut self) {
        for state_col in &mut *self.instrument_states.borrow_mut() {
            for state in state_col {
                state.pressed_note = None;
            }
        }
    }

    pub fn advance_frame(&mut self, frame_number: usize) {
        for state_col in &mut *self.instrument_states.borrow_mut() {
            for state in state_col {
                // Only run the frame function on instruments currently pressed.
                if let Some(PressedNote {
                    note,
                    pressed_frame,
                    extended_frames,
                }) = &mut state.pressed_note
                {
                    if state.frame_function.is_defined() {
                        if let Some(wasm_module_inst) = &self.wasm_module_inst {
                            if let Err(e) = wasm_module_inst.call_indirect_iii(
                                &state.frame_function,
                                Self::note_to_freq(*note),
                                *note as i32,
                                (frame_number - *pressed_frame) as i32,
                            ) {
                                elog!("frame: {:?}", e);
                            }
                        }
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
    }

    fn note_to_freq(note: u8) -> i32 {
        NOTE_FREQUENCIES[note as usize] as i32
    }
}
