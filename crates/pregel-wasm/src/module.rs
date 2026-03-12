//! Loading WASM modules from disk or memory.

use std::path::Path;

/// A WASM module loaded and ready for execution.
///
/// The module bytes are the raw `.wasm` file contents. `WasmExecutor` will
/// compile and instantiate them when running compute.
#[derive(Debug)]
pub struct WasmModule {
    pub bytes: Vec<u8>,
}

impl WasmModule {
    /// Load a WASM module from a file path.
    pub fn from_path(path: impl AsRef<Path>) -> pregel_common::Result<Self> {
        let bytes = std::fs::read(path)?;
        Ok(Self { bytes })
    }

    /// Create a module from in-memory bytes (e.g., downloaded or generated).
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}
