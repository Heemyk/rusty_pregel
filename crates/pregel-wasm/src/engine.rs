//! The WASM execution engine.
//!
//! Loads and runs WASM modules. The expected interface is a `compute` function
//! that takes (vertex, messages, context) and returns outgoing messages.

use crate::WasmModule;
use pregel_common::Result;
use wasmtime::{Engine, Linker, Module, Store};

/// Executes WASM vertex compute functions.
///
/// Uses [wasmtime](https://wasmtime.dev/) under the hood. The WASM module should
/// export a `compute` function. The exact ABI (how vertex/messages/context are
/// passed) is defined by the SDK that compiles user code.
///
/// # Current State
///
/// This is a scaffold. The actual implementation will:
/// 1. Instantiate the module
/// 2. Set up host imports (if any)
/// 3. Call the compute export with serialized inputs
/// 4. Return serialized outputs
pub struct WasmExecutor {
    engine: Engine,
}

impl WasmExecutor {
    pub fn new() -> Self {
        Self {
            engine: Engine::default(),
        }
    }

    /// Execute the WASM compute function.
    ///
    /// Expected WASM interface: `compute(vertex_ptr, message_ptr, context_ptr) -> i32`
    /// (or similar – the SDK defines the ABI). For now, returns input unchanged as placeholder.
    pub fn compute(&self, module: &WasmModule, input: &[u8]) -> Result<Vec<u8>> {
        let module = Module::new(&self.engine, &module.bytes)
            .map_err(|e| pregel_common::PregelError::Serialization(e.to_string()))?;

        let mut store = Store::new(&self.engine, ());
        let linker = Linker::new(&self.engine);
        let _instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| pregel_common::PregelError::Serialization(e.to_string()))?;

        // Placeholder: actual impl would call the compute export
        Ok(input.to_vec())
    }
}

impl Default for WasmExecutor {
    fn default() -> Self {
        Self::new()
    }
}
