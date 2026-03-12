//! # pregel-wasm
//!
//! WebAssembly execution for vertex compute functions.
//!
//! Enables multi-language vertex programs: compile to WASM from Rust, Python, Go,
//! or TypeScript, then run in a sandboxed environment. Uses [wasmtime](https://wasmtime.dev/).

pub mod engine;
pub mod module;

pub use engine::WasmExecutor;
pub use module::WasmModule;
