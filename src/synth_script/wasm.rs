#[cfg(not(feature = "desktop_web"))]
pub use crate::synth_script::wasm_host::{
    HostFunction, HostFunctionA, HostFunctionII, HostFunctionS, HostFunctionSIINNNN, WasmIndirectFunction, WasmModule,
    WasmModuleInst, WasmRuntime,
};
#[cfg(feature = "desktop_web")]
pub use crate::synth_script::wasm_web::{
    HostFunction, HostFunctionA, HostFunctionII, HostFunctionS, HostFunctionSIINNNN, WasmIndirectFunction, WasmModule,
    WasmModuleInst, WasmRuntime,
};
