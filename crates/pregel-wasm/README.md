# pregel-wasm

**Purpose:** Execute vertex compute functions compiled to WebAssembly (WASM). This enables multi-language support: write algorithms in Rust, Python, Go, or TypeScript; compile to WASM; run in a sandboxed, language-agnostic runtime.

**Who uses it:** `pregel-worker` loads a WASM module and uses `WasmExecutor` to run the vertex compute for each vertex.

## Why WASM?

* **Sandboxing** – WASM runs in a secure sandbox. Buggy or malicious user code can't crash the worker or access the filesystem.
* **Language agnostic** – Any language that compiles to WASM works (Rust, C, Go, etc.). Python/TypeScript can use toolchains like Pyodide.
* **Portable** – Same binary runs on x86, ARM, etc.
* **Fast** – Near-native speed, much faster than interpreting Python.

## How It Works

1. Developer writes a `VertexProgram` in their language.
2. SDK compiles it to a WASM module with a `compute` export.
3. Worker loads the module via `WasmModule::from_path()`.
4. For each vertex, worker calls `WasmExecutor::compute()` with serialized vertex + messages.
5. WASM function runs, returns serialized outgoing messages.

## What's Inside

| Module | Purpose |
|--------|---------|
| `engine` | `WasmExecutor` – wasmtime engine, runs the compute export |
| `module` | `WasmModule` – loaded WASM bytes (from file or memory) |

## Rust Note: wasmtime

We use [wasmtime](https://wasmtime.dev/), a standalone WASM runtime (not tied to browsers). It's the same tech used by Fermyon, Fastly, and others for server-side WASM.
