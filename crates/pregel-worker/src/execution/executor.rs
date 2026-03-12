//! Invoking the vertex compute function (WASM or native).

use pregel_common::Result;
use pregel_wasm::{WasmExecutor, WasmModule};

/// Executes the vertex compute for a single vertex.
///
/// When using WASM: deserializes vertex + messages, calls the WASM module's
/// compute export, returns serialized outgoing messages.
///
/// **Current state:** Placeholder. Returns empty vec. Full impl will wire
/// up WasmExecutor::compute() with proper serialization.
/// Placeholder for WASM execution. Currently native algorithms run directly in vertex_loop.
pub struct VertexExecutor {
    #[allow(dead_code)]
    pub(crate) wasm_executor: WasmExecutor,
}

impl VertexExecutor {
    pub fn new() -> Self {
        Self {
            wasm_executor: WasmExecutor::new(),
        }
    }

    /// Run compute for one vertex. Returns serialized outgoing messages.
    pub fn execute(
        &self,
        _vertex: &pregel_storage::VertexData,
        _messages: &[u8],
        _module: Option<&WasmModule>,
    ) -> Result<Vec<u8>> {
        // Placeholder: would call wasm_executor.compute()
        Ok(Vec::new())
    }
}

impl Default for VertexExecutor {
    fn default() -> Self {
        Self::new()
    }
}
