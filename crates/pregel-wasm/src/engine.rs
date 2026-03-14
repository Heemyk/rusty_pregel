//! The WASM execution engine.
//!
//! ABI: input bytes → output bytes. Host writes input to linear memory,
//! calls compute export, reads output back. See docs/ABI_SPEC.md and docs/WASM_ABI.md.

use crate::WasmModule;
use pregel_common::{AbiErrorCode, PregelError, Result};
use wasmtime::{Engine, Linker, Module, Store};

/// Input and output share the first page. Many wasm32 modules start with 64KB.
/// Input at 0, output at 32KB so both fit in one page.
const INPUT_MAX_LEN: usize = 32 * 1024;
const OUTPUT_OFFSET: u32 = 32 * 1024;
const OUTPUT_MAX_LEN: u32 = 32 * 1024;

fn to_err(e: impl ToString) -> PregelError {
    PregelError::Serialization(e.to_string())
}

fn abi_error_name(code: i32) -> &'static str {
    AbiErrorCode::from_i32(code).map_or("UNKNOWN", |c| match c {
        AbiErrorCode::Invalid => "EINVALID",
        AbiErrorCode::Deserialize => "EDESERIALIZE",
        AbiErrorCode::Serialize => "ESERIALIZE",
        AbiErrorCode::OutputOverrun => "EOUTPUT_OVERRUN",
        AbiErrorCode::Alloc => "EALLOC",
        AbiErrorCode::User => "EUSER",
    })
}

/// Executes WASM vertex compute functions.
pub struct WasmExecutor {
    engine: Engine,
}

impl WasmExecutor {
    pub fn new() -> Self {
        Self { engine: Engine::default() }
    }

    /// Execute the WASM compute function.
    /// ABI: compute(input_ptr, input_len, output_ptr, output_max_len) -> output_len
    pub fn compute(&self, module: &WasmModule, input: &[u8]) -> Result<Vec<u8>> {
        let module = Module::new(&self.engine, &module.bytes).map_err(to_err)?;
        let mut store = Store::new(&self.engine, ());
        let linker = Linker::new(&self.engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(to_err)?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| to_err("no memory export"))?;

        let input_len = input.len() as u32;
        if input.len() > INPUT_MAX_LEN {
            return Err(to_err("input too large"));
        }

        memory.write(&mut store, 0, input).map_err(to_err)?;

        let compute_fn = match instance.get_func(&mut store, "compute") {
            Some(f) => f,
            None => return Ok(Vec::new()),
        };

        let typed_fn = compute_fn
            .typed::<(i32, i32, i32, i32), i32>(&store)
            .map_err(to_err)?;
        let result = typed_fn
            .call(&mut store, (0, input_len as i32, OUTPUT_OFFSET as i32, OUTPUT_MAX_LEN as i32))
            .map_err(to_err)?;

        if result < 0 {
            return Err(PregelError::WasmGuest(
                abi_error_name(result).to_string(),
                result,
            ));
        }

        let out_len = result as usize;
        let mut buf = vec![0u8; out_len];
        memory.read(&store, OUTPUT_OFFSET as usize, &mut buf).map_err(to_err)?;
        Ok(buf)
    }
}

impl Default for WasmExecutor {
    fn default() -> Self {
        Self::new()
    }
}
