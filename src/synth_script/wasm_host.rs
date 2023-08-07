// Copyright © 2022 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::alloc::alloc;
use alloc::alloc::dealloc;
use alloc::alloc::realloc;
use alloc::alloc::Layout;
use alloc::boxed::Box;
use alloc::ffi::CString;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::ffi::CStr;
use core::mem;
use core::ptr;
use wamr_sys::*;

pub trait HostFunction {
    fn to_native_symbol(&mut self) -> NativeSymbol;
}

pub struct HostFunctionS<F> {
    closure: F,
    name: CString,
}
impl<F> HostFunctionS<F> {
    pub fn new(name: &str, closure: F) -> HostFunctionS<F> {
        HostFunctionS {
            closure: closure,
            name: CString::new(name).unwrap(),
        }
    }
}
const S_SIG: &str = "($)\0";
unsafe extern "C" fn trampoline_s_<F: FnMut(&CStr)>(exec_env: wasm_exec_env_t, v1: *const i8) {
    let f = &mut *(wasm_runtime_get_function_attachment(exec_env) as *mut F);
    f(CStr::from_ptr(v1));
}
impl<F: FnMut(&CStr)> HostFunction for HostFunctionS<F> {
    fn to_native_symbol(&mut self) -> NativeSymbol {
        NativeSymbol {
            symbol: self.name.as_ptr(),
            func_ptr: trampoline_s_::<F> as *mut c_void,
            signature: S_SIG.as_ptr() as *const i8,
            attachment: &mut self.closure as *mut _ as *mut c_void,
        }
    }
}

pub struct HostFunctionSIINNNN<F> {
    closure: F,
    name: CString,
}
impl<F> HostFunctionSIINNNN<F> {
    pub fn new(name: &str, closure: F) -> HostFunctionSIINNNN<F> {
        HostFunctionSIINNNN {
            closure: closure,
            name: CString::new(name).unwrap(),
        }
    }
}
const SIINNNN_SIG: &str = "($iiiiii)\0";
unsafe extern "C" fn trampoline_siisss_<
    F: FnMut(
        &CStr,
        i32,
        i32,
        Option<WasmIndirectFunction>,
        Option<WasmIndirectFunction>,
        Option<WasmIndirectFunction>,
        Option<WasmIndirectFunction>,
    ),
>(
    exec_env: wasm_exec_env_t,
    v1: *const i8,
    v2: i32,
    v3: i32,
    v4: u32,
    v5: u32,
    v6: u32,
    v7: u32,
) {
    let f = &mut *(wasm_runtime_get_function_attachment(exec_env) as *mut F);

    f(
        CStr::from_ptr(v1),
        v2,
        v3,
        WasmModuleInst::lookup_indirect_function(v4),
        WasmModuleInst::lookup_indirect_function(v5),
        WasmModuleInst::lookup_indirect_function(v6),
        WasmModuleInst::lookup_indirect_function(v7),
    );
}
impl<
        F: FnMut(
            &CStr,
            i32,
            i32,
            Option<WasmIndirectFunction>,
            Option<WasmIndirectFunction>,
            Option<WasmIndirectFunction>,
            Option<WasmIndirectFunction>,
        ),
    > HostFunction for HostFunctionSIINNNN<F>
{
    fn to_native_symbol(&mut self) -> NativeSymbol {
        NativeSymbol {
            symbol: self.name.as_ptr(),
            func_ptr: trampoline_siisss_::<F> as *mut c_void,
            signature: SIINNNN_SIG.as_ptr() as *const i8,
            attachment: &mut self.closure as *mut _ as *mut c_void,
        }
    }
}

pub struct HostFunctionII<F> {
    closure: F,
    name: CString,
}
impl<F> HostFunctionII<F> {
    pub fn new(name: &str, closure: F) -> HostFunctionII<F> {
        HostFunctionII {
            closure: closure,
            name: CString::new(name).unwrap(),
        }
    }
}
const II_SIG: &str = "(ii)\0";
unsafe extern "C" fn trampoline_ii_<F: FnMut(i32, i32)>(exec_env: wasm_exec_env_t, v1: i32, v2: i32) {
    let f = &mut *(wasm_runtime_get_function_attachment(exec_env) as *mut F);
    f(v1, v2)
}
impl<F: FnMut(i32, i32)> HostFunction for HostFunctionII<F> {
    fn to_native_symbol(&mut self) -> NativeSymbol {
        NativeSymbol {
            symbol: self.name.as_ptr(),
            func_ptr: trampoline_ii_::<F> as *mut c_void,
            signature: II_SIG.as_ptr() as *const i8,
            attachment: &mut self.closure as *mut _ as *mut c_void,
        }
    }
}

pub struct HostFunctionA<F> {
    closure: F,
    name: CString,
}
impl<F> HostFunctionA<F> {
    pub fn new(name: &str, closure: F) -> HostFunctionA<F> {
        HostFunctionA {
            closure: closure,
            name: CString::new(name).unwrap(),
        }
    }
}
const A_SIG: &str = "(*~)\0";
unsafe extern "C" fn trampoline_a_<F: FnMut(&[u8])>(exec_env: wasm_exec_env_t, v1: *const u8, v1l: i32) {
    let f = &mut *(wasm_runtime_get_function_attachment(exec_env) as *mut F);
    f(core::slice::from_raw_parts(v1, v1l as usize))
}
impl<F: FnMut(&[u8])> HostFunction for HostFunctionA<F> {
    fn to_native_symbol(&mut self) -> NativeSymbol {
        NativeSymbol {
            symbol: self.name.as_ptr(),
            func_ptr: trampoline_a_::<F> as *mut c_void,
            signature: A_SIG.as_ptr() as *const i8,
            attachment: &mut self.closure as *mut _ as *mut c_void,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct WasmIndirectFunction {
    table_index: u32,
}

pub struct WasmRuntime {
    _module_name: CString,
    _functions: Vec<Box<dyn HostFunction>>,
    _native_symbols: Vec<NativeSymbol>,
}

pub struct WasmModule {
    module: wasm_module_t,
    _wasm_buffer: Vec<u8>,
    // Only for ownership
    _runtime: Option<Rc<WasmRuntime>>,
}

pub struct WasmModuleInst {
    module_inst: wasm_module_inst_t,
    // Only for ownership
    _module: Option<Rc<WasmModule>>,
}

const INSTANCE_STACK_SIZE: u32 = 8092;
// Not sure what this is about. This seems to be used for the wamr application framework, which I don't use.
// WASM code is creating memory objects for their own memory and are allocated separately from this setting.
const INSTANCE_HEAP_SIZE: u32 = 0;

const PTR_SIZE: usize = mem::size_of::<usize>();
unsafe fn malloc_func(size: usize) -> *mut u8 {
    let layout = Layout::from_size_align(size + PTR_SIZE, PTR_SIZE).unwrap();
    let ptr = alloc(layout);
    // free only provides the allocated pointer, so we need to expand the allocation,
    // store the size at the beginning and return the address just after to the application.
    core::slice::from_raw_parts_mut(ptr as *mut usize, 1)[0] = size;
    ptr.offset(PTR_SIZE as isize)
}

unsafe fn free_func(ptr: *mut u8) {
    let alloc_ptr = ptr.offset(-(PTR_SIZE as isize));
    let size = core::slice::from_raw_parts_mut(alloc_ptr as *mut usize, 1)[0];
    let layout = Layout::from_size_align(size, PTR_SIZE).unwrap();
    dealloc(alloc_ptr, layout);
}

unsafe fn realloc_func(ptr: *mut u8, new_size: usize) -> *mut u8 {
    let alloc_ptr = ptr.offset(-(PTR_SIZE as isize));
    let size = core::slice::from_raw_parts_mut(alloc_ptr as *mut usize, 1)[0];
    let layout = Layout::from_size_align(size, PTR_SIZE).unwrap();
    let new_ptr = realloc(alloc_ptr, layout, new_size);
    core::slice::from_raw_parts_mut(alloc_ptr as *mut usize, 1)[0] = new_size;
    new_ptr.offset(PTR_SIZE as isize)
}

impl WasmRuntime {
    pub fn new(mut functions: Vec<Box<dyn HostFunction>>) -> Result<WasmRuntime, String> {
        unsafe {
            let mut init_args: RuntimeInitArgs = mem::zeroed();

            // Configure memory allocation.
            // Use a manual allocator that uses the Rust global allocator
            // to support no_std builds.
            init_args.mem_alloc_type = mem_alloc_type_t_Alloc_With_Allocator;
            init_args.mem_alloc_option.allocator.malloc_func = malloc_func as *mut c_void;
            init_args.mem_alloc_option.allocator.free_func = free_func as *mut c_void;
            init_args.mem_alloc_option.allocator.realloc_func = realloc_func as *mut c_void;

            // initialize the runtime before registering the native functions
            if !wasm_runtime_full_init(&mut init_args as *mut _) {
                panic!("CANT INIT RUNTIME");
            }

            let mut native_symbols: Vec<NativeSymbol> = functions.iter_mut().map(|f| f.to_native_symbol()).collect();

            let module_name = CString::new("env").unwrap();
            if !wasm_runtime_register_natives(
                module_name.as_ptr(),
                native_symbols.as_mut_ptr(),
                native_symbols.len() as u32,
            ) {
                panic!("wasm_runtime_register_natives failed");
            }
            Ok(WasmRuntime {
                _module_name: module_name,
                _functions: functions,
                _native_symbols: native_symbols,
            })
        }
    }
}

impl Drop for WasmRuntime {
    fn drop(&mut self) {
        unsafe { wasm_runtime_destroy() }
    }
}

impl WasmModule {
    pub fn new(mut wasm_buffer: Vec<u8>, runtime: Rc<WasmRuntime>) -> Result<WasmModule, String> {
        unsafe {
            let mut error_buf = [0; 128];
            // parse the WASM file from buffer and create a WASM module
            let module = wasm_runtime_load(
                wasm_buffer.as_mut_ptr(),
                wasm_buffer.len() as u32,
                error_buf.as_mut_ptr(),
                error_buf.len() as u32,
            );
            if module == ptr::null_mut() {
                panic!("wasm_runtime_load failed: {:?}", CStr::from_ptr(error_buf.as_ptr()));
            }
            Ok(WasmModule {
                module,
                _wasm_buffer: wasm_buffer,
                _runtime: Some(runtime),
            })
        }
    }
}

impl Drop for WasmModule {
    fn drop(&mut self) {
        unsafe { wasm_runtime_unload(self.module) }
    }
}

impl WasmModuleInst {
    pub fn new<F: Fn() + 'static>(module: Rc<WasmModule>, post_init_callback: F) -> Result<WasmModuleInst, String> {
        unsafe {
            let mut error_buf = [0; 128];

            // create an instance of the WASM module (WASM linear memory is ready)
            let module_inst = wasm_runtime_instantiate(
                module.module,
                INSTANCE_STACK_SIZE,
                INSTANCE_HEAP_SIZE,
                error_buf.as_mut_ptr(),
                error_buf.len() as u32,
            );
            if module_inst == ptr::null_mut() {
                panic!(
                    "wasm_runtime_instantiate failed: {:?}",
                    CStr::from_ptr(error_buf.as_ptr())
                );
            }

            let module_inst = WasmModuleInst {
                module_inst,
                _module: Some(module),
            };

            let maybe_start = module_inst.lookup_function(CStr::from_ptr("_start\0".as_ptr() as *const i8));
            if let Some(start) = maybe_start {
                let argv: [u32; 0] = [];
                module_inst.call_argv(start, argv)?;
            }

            post_init_callback();

            Ok(module_inst)
        }
    }

    fn lookup_indirect_function(table_index: u32) -> Option<WasmIndirectFunction> {
        // Zig and others seems to leave the offset 0 of the table empty in generated WASM,
        // which errors out on calls and is convenient to represent null.
        if table_index != 0 {
            Some(WasmIndirectFunction { table_index })
        } else {
            None
        }
    }

    fn lookup_function(&self, name: &CStr) -> Option<wasm_function_inst_t> {
        unsafe {
            let f = wasm_runtime_lookup_function(self.module_inst, name.as_ptr(), ptr::null());
            if f != ptr::null_mut() {
                Some(f)
            } else {
                None
            }
        }
    }

    pub fn call_indirect_ii(&self, function: &WasmIndirectFunction, a1: i32, a2: i32) -> Result<(), String> {
        let argv: [u32; 2] = [a1 as u32, a2 as u32];
        self.call_indirect_argv(function, argv)
    }

    pub fn call_indirect_iii(&self, function: &WasmIndirectFunction, a1: i32, a2: i32, a3: i32) -> Result<(), String> {
        let argv: [u32; 3] = [a1 as u32, a2 as u32, a3 as u32];
        self.call_indirect_argv(function, argv)
    }

    pub fn call_indirect_iiii(
        &self,
        function: &WasmIndirectFunction,
        a1: i32,
        a2: i32,
        a3: i32,
        a4: i32,
    ) -> Result<(), String> {
        let argv: [u32; 4] = [a1 as u32, a2 as u32, a3 as u32, a4 as u32];
        self.call_indirect_argv(function, argv)
    }

    fn call_argv<const ARGC: usize>(
        &self,
        function: wasm_function_inst_t,
        mut argv: [u32; ARGC],
    ) -> Result<(), String> {
        unsafe {
            // get the singleton execution environment of this instance to execute the WASM functions
            let exec_env = wasm_runtime_get_exec_env_singleton(self.module_inst);
            if exec_env == ptr::null_mut() {
                return Err("wasm_runtime_get_exec_env_singleton failed.".to_string());
            }

            // call the WASM function
            if wasm_runtime_call_wasm(exec_env, function, ARGC as u32, argv.as_mut_ptr()) {
                // the return value is stored in argv[0], ignore it for now.
                Ok(())
            } else {
                // exception is thrown if call fails
                let cstr = CStr::from_ptr(wasm_runtime_get_exception(wasm_runtime_get_module_inst(exec_env)));
                panic!("wasm_runtime_call_wasm failed: {:?}", cstr);
            }
        }
    }

    fn call_indirect_argv<const ARGC: usize>(
        &self,
        function: &WasmIndirectFunction,
        mut argv: [u32; ARGC],
    ) -> Result<(), String> {
        unsafe {
            // get the singleton execution environment of this instance to execute the WASM functions
            let exec_env = wasm_runtime_get_exec_env_singleton(self.module_inst);
            if exec_env == ptr::null_mut() {
                return Err("wasm_runtime_get_exec_env_singleton failed.".to_string());
            }
            // call the WASM function
            if wasm_runtime_call_indirect(exec_env, function.table_index, ARGC as u32, argv.as_mut_ptr()) {
                // the return value is stored in argv[0], ignore it for now.
                Ok(())
            } else {
                // exception is thrown if call fails
                let cstr = CStr::from_ptr(wasm_runtime_get_exception(wasm_runtime_get_module_inst(exec_env)));
                panic!(
                    "wasm_runtime_call_indirect of table index {:?} failed: {:?}",
                    function.table_index, cstr
                );
            }
        }
    }
}

impl Drop for WasmModuleInst {
    fn drop(&mut self) {
        unsafe { wasm_runtime_deinstantiate(self.module_inst) }
    }
}
