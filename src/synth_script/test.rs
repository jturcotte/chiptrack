// extern crate wamr_sys;

extern crate alloc;

use wamr_sys::mem_alloc_type_t_Alloc_With_Pool;
use wamr_sys::NativeSymbol;
use wamr_sys::RuntimeInitArgs;
use wamr_sys::wasm_exec_env_t;
use wamr_sys::wasm_function_inst_t;
use wamr_sys::wasm_module_inst_t;
use wamr_sys::wasm_module_t;
use wamr_sys::wasm_runtime_call_wasm;
use wamr_sys::wasm_runtime_create_exec_env;
use wamr_sys::wasm_runtime_full_init;
use wamr_sys::wasm_runtime_get_exception;
use wamr_sys::wasm_runtime_get_function_attachment;
use wamr_sys::wasm_runtime_get_module_inst;
use wamr_sys::wasm_runtime_instantiate;
use wamr_sys::wasm_runtime_load;
use wamr_sys::wasm_runtime_lookup_function;
use wamr_sys::wasm_runtime_register_natives;

use alloc::vec::Vec;
use alloc::vec;

use core::mem;
use core::ptr;
use std::ffi::c_void;
use std::ffi::CStr;
use std::ffi::CString;
use std::rc::Rc;



pub trait HostFunction {
    fn to_native_symbol(&mut self) -> NativeSymbol;
}

pub struct HostFunctionSISSS<F> 
{
    closure: F,
    name: CString,
}
impl<F> HostFunctionSISSS<F> 
{
    pub fn new(name: &str, closure: F) -> HostFunctionSISSS<F> {
        HostFunctionSISSS{
            closure: closure,
            name: CString::new(name).unwrap(),
        }
    }
}
const SISSS_SIG: &str = "($i$$$)\0";
unsafe extern "C" fn trampoline_siss_<F: FnMut(&WasmModuleInst, &CStr, i32, &CStr, &CStr, &CStr)>(exec_env: wasm_exec_env_t, v1: *const i8, v2: i32, v3: *const i8, v4: *const i8, v5: *const i8) {
    let f = &mut *(wasm_runtime_get_function_attachment(exec_env) as *mut F);
    let m = WasmModuleInst{
        module_inst: wasm_runtime_get_module_inst(exec_env),
        _module: None};
    f(&m, CStr::from_ptr(v1), v2, CStr::from_ptr(v3), CStr::from_ptr(v4), CStr::from_ptr(v5));
}
impl<F: FnMut(&WasmModuleInst, &CStr, i32, &CStr, &CStr, &CStr)> HostFunction for HostFunctionSISSS<F>
{
    fn to_native_symbol(&mut self) -> NativeSymbol {
        NativeSymbol { 
        symbol: self.name.as_ptr(),
        func_ptr: trampoline_siss_::<F> as *mut c_void,
        signature: SISSS_SIG.as_ptr() as *const i8,
        attachment: &mut self.closure as *mut _ as *mut c_void
        }
    }
}

pub struct HostFunctionI<F> 
{
    closure: F,
    name: CString,
}
impl<F> HostFunctionI<F> 
{
    pub fn new(name: &str, closure: F) -> HostFunctionI<F> {
        HostFunctionI{
            closure: closure,
            name: CString::new(name).unwrap(),
        }
    }
}
const I_SIG: &str = "(i)\0";
unsafe extern "C" fn trampoline_i_<F: FnMut(i32)>(exec_env: wasm_exec_env_t, v: i32) {
    let f = &mut *(wasm_runtime_get_function_attachment(exec_env) as *mut F);
    f(v)
}
impl<F: FnMut(i32)> HostFunction for HostFunctionI<F>
{
    fn to_native_symbol(&mut self) -> NativeSymbol {
        NativeSymbol { 
        symbol: self.name.as_ptr(),
        func_ptr: trampoline_i_::<F> as *mut c_void,
        signature: I_SIG.as_ptr() as *const i8,
        attachment: &mut self.closure as *mut _ as *mut c_void
        }
    }
}

pub struct HostFunctionII<F> 
{
    closure: F,
    name: CString,
}
impl<F> HostFunctionII<F> 
{
    pub fn new(name: &str, closure: F) -> HostFunctionII<F> {
        HostFunctionII{
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
impl<F: FnMut(i32, i32)> HostFunction for HostFunctionII<F>
{
    fn to_native_symbol(&mut self) -> NativeSymbol {
        NativeSymbol { 
        symbol: self.name.as_ptr(),
        func_ptr: trampoline_ii_::<F> as *mut c_void,
        signature: II_SIG.as_ptr() as *const i8,
        attachment: &mut self.closure as *mut _ as *mut c_void
        }
    }
}

pub struct HostFunctionIA<F> 
{
    closure: F,
    name: CString,
}
impl<F> HostFunctionIA<F> 
{
    pub fn new(name: &str, closure: F) -> HostFunctionIA<F> {
        HostFunctionIA{
            closure: closure,
            name: CString::new(name).unwrap(),
        }
    }
}
const IA_SIG: &str = "(i*~)\0";
unsafe extern "C" fn trampoline_ia_<F: FnMut(i32, &[i32])>(exec_env: wasm_exec_env_t, v1: i32, v2: *const i32, v2l: i32) {
    let f = &mut *(wasm_runtime_get_function_attachment(exec_env) as *mut F);
    f(v1, std::slice::from_raw_parts(v2, v2l as usize))
}
impl<F: FnMut(i32, &[i32])> HostFunction for HostFunctionIA<F>
{
    fn to_native_symbol(&mut self) -> NativeSymbol {
        NativeSymbol { 
        symbol: self.name.as_ptr(),
        func_ptr: trampoline_ia_::<F> as *mut c_void,
        signature: IA_SIG.as_ptr() as *const i8,
        attachment: &mut self.closure as *mut _ as *mut c_void
        }
    }
}

pub struct HostFunctionIAA<F> 
{
    closure: F,
    name: CString,
}
impl<F> HostFunctionIAA<F> 
{
    pub fn new(name: &str, closure: F) -> HostFunctionIAA<F> {
        HostFunctionIAA{
            closure: closure,
            name: CString::new(name).unwrap(),
        }
    }
}
const IAA_SIG: &str = "(i*~*~)\0";
unsafe extern "C" fn trampoline_iaa_<F: FnMut(i32, &[i32], &[i32])>(exec_env: wasm_exec_env_t, v1: i32, v2: *const i32, v2l: i32, v3: *const i32, v3l: i32) {
    let f = &mut *(wasm_runtime_get_function_attachment(exec_env) as *mut F);
    f(v1, std::slice::from_raw_parts(v2, v2l as usize), std::slice::from_raw_parts(v3, v3l as usize))
}
impl<F: FnMut(i32, &[i32], &[i32])> HostFunction for HostFunctionIAA<F>
{
    fn to_native_symbol(&mut self) -> NativeSymbol {
        NativeSymbol { 
        symbol: self.name.as_ptr(),
        func_ptr: trampoline_iaa_::<F> as *mut c_void,
        signature: IAA_SIG.as_ptr() as *const i8,
        attachment: &mut self.closure as *mut _ as *mut c_void
        }
    }
}

pub struct HostFunctionIi<F> 
{
    closure: F,
    name: CString,
}
impl<F> HostFunctionIi<F> 
{
    pub fn new(name: &str, closure: F) -> HostFunctionIi<F> {
        HostFunctionIi{
            closure: closure,
            name: CString::new(name).unwrap(),
        }
    }
}
const I_I_SIG: &str = "(i)i\0";
unsafe extern "C" fn trampoline_i_i<F: FnMut(i32) -> i32>(exec_env: wasm_exec_env_t, v: i32) -> i32 {
    let f = &mut *(wasm_runtime_get_function_attachment(exec_env) as *mut F);
    f(v)
}
impl<F: FnMut(i32) -> i32> HostFunction for HostFunctionIi<F>
{
    fn to_native_symbol(&mut self) -> NativeSymbol {
        NativeSymbol { 
        symbol: self.name.as_ptr(),
        func_ptr: trampoline_i_i::<F> as *mut c_void,
        signature: I_I_SIG.as_ptr() as *const i8,
        attachment: &mut self.closure as *mut _ as *mut c_void
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct WasmFunction {
    f: wasm_function_inst_t,
}

// FIXME: Drop
pub struct WasmRuntime {
    _module_name: CString,
    _functions: Vec<Box<dyn HostFunction>>,
    _native_symbols: Vec<NativeSymbol>,
    _heap_pool: Vec<u8>,
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

pub struct WasmExecEnv {
    exec_env: wasm_exec_env_t,
    // Only for ownership
    _module_inst: Option<Rc<WasmModuleInst>>,
}

const INSTANCE_STACK_SIZE: u32 = 8092;
// Not sure what this is about. This seems to be used for the wamr application framework, which I don't use.
// WASM code is creating memory objects for their own memory and are allocated separately from this setting.
const INSTANCE_HEAP_SIZE: u32 = 0;

impl WasmRuntime {
    pub fn new(mut functions: Vec<Box<dyn HostFunction>>) -> Result<WasmRuntime, String> { unsafe {
        // let init_args = RuntimeInitArgs{};
        let mut init_args: RuntimeInitArgs = mem::zeroed();

        // configure memory allocation 
        let mut heap_pool: Vec<u8> = vec![0; 128*1024];
        init_args.mem_alloc_type = mem_alloc_type_t_Alloc_With_Pool;
        init_args.mem_alloc_option.pool.heap_buf = heap_pool.as_mut_ptr() as *mut c_void;
        init_args.mem_alloc_option.pool.heap_size = heap_pool.len() as u32;

        // initialize the runtime before registering the native functions
        if !wasm_runtime_full_init(&mut init_args as *mut _) {
            return Err("CANT INIT RUNTIME".to_string());
        }

        let mut native_symbols: Vec<NativeSymbol> = 
            functions.iter_mut().map(|f| f.to_native_symbol()).collect();

        let module_name = CString::new("gb").unwrap();
        if !wasm_runtime_register_natives(module_name.as_ptr(),
                                         native_symbols.as_mut_ptr(), 
                                         native_symbols.len() as u32) {
            return Err("wasm_runtime_register_natives failed".to_string());
        }
        Ok(WasmRuntime{
            _module_name: module_name,
            _functions: functions,
            _native_symbols: native_symbols,
            _heap_pool: heap_pool
            })
    } }
}

impl WasmModule {

    pub fn new(mut wasm_buffer: Vec<u8>, runtime: Rc<WasmRuntime>) -> Result<WasmModule, String> { unsafe {
        let mut error_buf = [0; 128];
        // parse the WASM file from buffer and create a WASM module 
        let module = wasm_runtime_load(wasm_buffer.as_mut_ptr(), wasm_buffer.len() as u32, error_buf.as_mut_ptr(), error_buf.len() as u32);
        if module == ptr::null_mut() {
            return Err(format!("wasm_runtime_load failed: {:?}", CStr::from_ptr(error_buf.as_ptr())));
        }
        Ok(WasmModule{module, _wasm_buffer: wasm_buffer, _runtime: Some(runtime)})
    } }
}

impl WasmModuleInst {

    pub fn new(module: Rc<WasmModule>) -> Result<WasmModuleInst, String> { unsafe {
        let mut error_buf = [0; 128];

        // create an instance of the WASM module (WASM linear memory is ready) 
        let module_inst = wasm_runtime_instantiate(module.module, INSTANCE_STACK_SIZE, INSTANCE_HEAP_SIZE,
                                             error_buf.as_mut_ptr(), error_buf.len() as u32);
        if module_inst == ptr::null_mut() {
            return Err(format!("wasm_runtime_instantiate failed: {:?}", CStr::from_ptr(error_buf.as_ptr())));
        }
        Ok(WasmModuleInst{module_inst, _module: Some(module)})
    } }

    pub fn lookup_function(&self, name: &CStr) -> Option<WasmFunction> { unsafe {
        let f = wasm_runtime_lookup_function(self.module_inst, name.as_ptr(), ptr::null());
        if f != ptr::null_mut() {
            Some(WasmFunction{f})
        } else {
            None
        }
    } }

}


impl WasmExecEnv {

    pub fn new(module_inst: Rc<WasmModuleInst>) -> Result<WasmExecEnv, String> { unsafe {
        
        // creat an execution environment to execute the WASM functions 
        let exec_env = wasm_runtime_create_exec_env(module_inst.module_inst, INSTANCE_STACK_SIZE);
        if exec_env == ptr::null_mut() {
            return Err("wasm_runtime_create_exec_env failed.".to_string());
        }
        
        Ok(WasmExecEnv{exec_env, _module_inst: Some(module_inst)})
    } }

    pub fn call_ii(&self, function: WasmFunction, a1: i32, a2: i32) -> Result<(), String> {
        let argv: [u32; 2] = [a1 as u32, a2 as u32];
        self.call_argv(function, argv)
    }

    pub fn call_iii(&self, function: WasmFunction, a1: i32, a2: i32, a3: i32) -> Result<(), String> {
        let argv: [u32; 3] = [a1 as u32, a2 as u32, a3 as u32];
        self.call_argv(function, argv)
    }

    fn call_argv<const ARGC: usize>(&self, function: WasmFunction, mut argv: [u32; ARGC]) -> Result<(), String> { unsafe {
        // call the WASM function 
        if wasm_runtime_call_wasm(self.exec_env, function.f, ARGC as u32, argv.as_mut_ptr()) {
            // the return value is stored in argv[0], ignore it for now.
            Ok(())
        }
        else {
            // exception is thrown if call fails 
            let cstr = CStr::from_ptr(wasm_runtime_get_exception(wasm_runtime_get_module_inst(self.exec_env)));
            return Err(format!("wasm_runtime_call_wasm failed: {:?}", cstr));
        }
    } }
}