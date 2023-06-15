// Copyright © 2023 Jocelyn Turcotte <turcotte.j@gmail.com>
// SPDX-License-Identifier: MIT

extern crate alloc;

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::Cell;
use core::cell::RefCell;
use core::ffi::CStr;
use core::mem;
use js_sys::{Function, Object, Reflect, WebAssembly};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};

thread_local! {
    static CURRENT_INSTANCE: RefCell<Option<WebAssembly::Instance>> = RefCell::new(None);
}

pub trait HostFunction {
    fn move_into_import(&mut self, env: &Object) -> ();
}

pub struct HostFunctionS {
    closure: Option<Closure<dyn FnMut(*const i8)>>,
    name: String,
}
impl HostFunctionS {
    pub fn new<F>(name: &str, mut closure: F) -> HostFunctionS
    where
        F: FnMut(&CStr) + 'static,
    {
        let native_closure = Closure::new(move |v1: *const i8| unsafe {
            CURRENT_INSTANCE.with(|current_instance| {
                let maybe_instance = current_instance.borrow();
                let exports = maybe_instance
                    .as_ref()
                    .expect("CURRENT_INSTANCE hasn't been initialized yet, async race condition?")
                    .exports();
                let mem = Reflect::get(exports.as_ref(), &"memory".into())
                    .unwrap()
                    .dyn_into::<WebAssembly::Memory>()
                    .unwrap();
                let typebuf = js_sys::Int8Array::new(&mem.buffer());
                let vec = typebuf.to_vec();

                closure(CStr::from_ptr(vec.as_ptr().offset(v1 as isize)));
            });
        });

        HostFunctionS {
            closure: Some(native_closure),
            name: name.to_owned(),
        }
    }
}
impl HostFunction for HostFunctionS {
    fn move_into_import(&mut self, env: &Object) -> () {
        Reflect::set(
            &env,
            &mem::take(&mut self.name).into(),
            &self.closure.take().unwrap().into_js_value(),
        )
        .unwrap();
    }
}

pub struct HostFunctionSIISSS {
    closure: Option<Closure<dyn FnMut(*const i8, i32, i32, *const i8, *const i8, *const i8)>>,
    name: String,
}
impl HostFunctionSIISSS {
    pub fn new<F>(name: &str, mut closure: F) -> HostFunctionSIISSS
    where
        F: FnMut(&WasmModuleInst, &CStr, i32, i32, &CStr, &CStr, &CStr) + 'static,
    {
        let native_closure = Closure::new(
            move |v1: *const i8, v2: i32, v3: i32, v4: *const i8, v5: *const i8, v6: *const i8| unsafe {
                log!("set_instrument_at_column {:?}", v1);

                CURRENT_INSTANCE.with(|current_instance| {
                    let maybe_instance = current_instance.borrow();
                    let exports = maybe_instance
                        .as_ref()
                        .expect("CURRENT_INSTANCE hasn't been initialized yet, async race condition?")
                        .exports();
                    let mem = Reflect::get(exports.as_ref(), &"memory".into())
                        .unwrap()
                        .dyn_into::<WebAssembly::Memory>()
                        .unwrap();
                    let typebuf = js_sys::Int8Array::new(&mem.buffer());
                    // FIXME: This copies the whole WebAssembly instance's memory on each call, just to be able to
                    //        return null-terminated CStrs. Possible alternatives:
                    //        - Asking the instruments to provide the string size would be the safest, but closures
                    //          in wasm-bindgen are currently limited to 8 parameters, so we'd bust that limit
                    //          and it would probably add an unecessary overhead for the warm ports, which maps the
                    //          instance's memory into the host's.
                    //        - If Int8Array was wrapping the native indexOf we could find the NULL ourselves to only
                    //          copy that part of the memory, but it's not exposed.
                    // In our case it's going to be one 64kb page most of the time, so for the web port to only do at
                    // startup it should be OK for now.
                    let vec = typebuf.to_vec();

                    closure(
                        &WasmModuleInst::dummy(),
                        CStr::from_ptr(vec.as_ptr().offset(v1 as isize)),
                        v2,
                        v3,
                        CStr::from_ptr(vec.as_ptr().offset(v4 as isize)),
                        CStr::from_ptr(vec.as_ptr().offset(v5 as isize)),
                        CStr::from_ptr(vec.as_ptr().offset(v6 as isize)),
                    );
                });
            },
        );

        HostFunctionSIISSS {
            closure: Some(native_closure),
            name: name.to_owned(),
        }
    }
}
impl HostFunction for HostFunctionSIISSS {
    fn move_into_import(&mut self, env: &Object) -> () {
        Reflect::set(
            &env,
            &mem::take(&mut self.name).into(),
            &self.closure.take().unwrap().into_js_value(),
        )
        .unwrap();
    }
}

pub struct HostFunctionII {
    closure: Option<Closure<dyn FnMut(i32, i32)>>,
    name: String,
}
impl HostFunctionII {
    pub fn new<F>(name: &str, mut closure: F) -> HostFunctionII
    where
        F: FnMut(i32, i32) + 'static,
    {
        let native_closure = Closure::new(move |v1: i32, v2: i32| closure(v1, v2));

        HostFunctionII {
            closure: Some(native_closure),
            name: name.to_owned(),
        }
    }
}
impl HostFunction for HostFunctionII {
    fn move_into_import(&mut self, env: &Object) -> () {
        Reflect::set(
            &env,
            &mem::take(&mut self.name).into(),
            &self.closure.take().unwrap().into_js_value(),
        )
        .unwrap();
    }
}

pub struct HostFunctionA {
    closure: Option<Closure<dyn FnMut(*const u8, i32)>>,
    name: String,
}
impl HostFunctionA {
    pub fn new<F>(name: &str, mut closure: F) -> HostFunctionA
    where
        F: FnMut(&[u8]) + 'static,
    {
        let native_closure = Closure::new(move |v1: *const u8, v1l: i32| {
            CURRENT_INSTANCE.with(|current_instance| {
                let maybe_instance = current_instance.borrow();
                let exports = maybe_instance
                    .as_ref()
                    .expect("CURRENT_INSTANCE hasn't been initialized yet, async race condition?")
                    .exports();
                let mem = Reflect::get(exports.as_ref(), &"memory".into())
                    .unwrap()
                    .dyn_into::<WebAssembly::Memory>()
                    .unwrap();
                let typebuf = js_sys::Uint8Array::new(&mem.buffer());
                let vec = typebuf.slice(v1 as u32, v1 as u32 + v1l as u32).to_vec();

                closure(&vec);
            });
        });

        HostFunctionA {
            closure: Some(native_closure),
            name: name.to_owned(),
        }
    }
}
impl HostFunction for HostFunctionA {
    fn move_into_import(&mut self, env: &Object) -> () {
        Reflect::set(
            &env,
            &mem::take(&mut self.name).into(),
            &self.closure.take().unwrap().into_js_value(),
        )
        .unwrap();
    }
}

#[derive(Debug, Clone)]
pub struct WasmFunction {
    function: Function,
}

pub struct WasmRuntime {
    imports: Object,
}

pub struct WasmModule {
    wasm_buffer: Cell<Vec<u8>>,
    // Only for ownership
    _runtime: Option<Rc<WasmRuntime>>,
}

pub struct WasmModuleInst {
    // Only for ownership
    _module: Option<Rc<WasmModule>>,
}

impl WasmRuntime {
    pub fn new(functions: Vec<Box<dyn HostFunction>>) -> Result<WasmRuntime, String> {
        let env = Object::new();
        for mut host_fn in functions {
            host_fn.move_into_import(&env)
        }
        let imports = Object::new();
        Reflect::set(&imports, &"env".into(), &env.into()).unwrap();
        Ok(WasmRuntime { imports })
    }
}

async fn run_async(wasm_buffer: Vec<u8>, imports: Object) -> Result<(), JsValue> {
    let instance_js = JsFuture::from(WebAssembly::instantiate_buffer(&wasm_buffer, &imports)).await?;
    let instance: WebAssembly::Instance = Reflect::get(&instance_js, &"instance".into())?.dyn_into()?;
    let exports_js = instance.exports();

    CURRENT_INSTANCE.with(|current_instance| {
        // FIXME: It would be cleaner to only set the thread_local instance when entering a call through WasmModuleInst
        //        so that this supports multiple module instances. But I don't need this now and it's simpler that
        //        way when it's time to construct a dummy WasmModuleInst for host function calls that provide one.
        assert!(current_instance.replace(Some(instance)).is_none());
    });

    // Call _start, which will call main and trigger the instrument setup
    Reflect::get(exports_js.as_ref(), &"_start".into())?
        .dyn_into::<Function>()?
        .call0(&JsValue::undefined())?;

    Ok(())
}

impl WasmModule {
    pub fn new(wasm_buffer: Vec<u8>, runtime: Rc<WasmRuntime>) -> Result<WasmModule, String> {
        Ok(WasmModule {
            wasm_buffer: Cell::new(wasm_buffer),
            _runtime: Some(runtime),
        })
    }
}

impl WasmModuleInst {
    pub fn new<F: Fn() + 'static>(module: Rc<WasmModule>, post_init_callback: F) -> Result<WasmModuleInst, String> {
        let wasm_buffer = module.wasm_buffer.take();
        let runtime = module._runtime.as_ref().unwrap();
        let imports = runtime.imports.clone();

        spawn_local(async move {
            run_async(wasm_buffer, imports).await.unwrap();
            post_init_callback();
        });
        Ok(WasmModuleInst { _module: Some(module) })
    }

    /// Used internally to provide to callback during _start
    fn dummy() -> WasmModuleInst {
        WasmModuleInst { _module: None }
    }

    pub fn lookup_function(&self, name: &CStr) -> Option<WasmFunction> {
        let function = CURRENT_INSTANCE.with(|current_instance| {
            let maybe_instance = current_instance.borrow();
            let exports = maybe_instance
                .as_ref()
                .expect("CURRENT_INSTANCE hasn't been initialized yet, async race condition?")
                .exports();
            let js_name = name.to_str().unwrap().into();
            if Reflect::has(exports.as_ref(), &js_name).unwrap() {
                let js_function = Reflect::get(exports.as_ref(), &name.to_str().unwrap().into()).unwrap();
                Some(
                    js_function
                        .dyn_into::<Function>()
                        .expect("Function export was found but not a Function."),
                )
            } else {
                None
            }
        });
        function.map(|f| WasmFunction { function: f })
    }

    pub fn call_ii(&self, function: &WasmFunction, a1: i32, a2: i32) -> Result<(), JsValue> {
        function.function.call2(&JsValue::undefined(), &a1.into(), &a2.into())?;
        Ok(())
    }

    pub fn call_iii(&self, function: &WasmFunction, a1: i32, a2: i32, a3: i32) -> Result<(), JsValue> {
        function
            .function
            .call3(&JsValue::undefined(), &a1.into(), &a2.into(), &a3.into())?;
        Ok(())
    }
}
