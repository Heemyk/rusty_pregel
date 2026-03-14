# SDK Architecture: From Code to WASM and Cross-Language

How the Pregel SDK works, how it compiles to WASM, and how the ABI enables Python, Go, TypeScript, etc.

---

## Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Algorithm Author                                                           │
├─────────────────────────────────────────────────────────────────────────────┤
│  Rust SDK: impl VertexProgram for MyAlgo { compute(...) }                   │
│  Python:   class MyAlgo(VertexProgram): def compute(...)                     │
│  Go:       type MyAlgo struct{}; func (m *MyAlgo) Compute(...)               │
│  TypeScript: class MyAlgo extends VertexProgram { compute(...) }             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  ABI (Wire Format) — Language-agnostic contract                              │
├─────────────────────────────────────────────────────────────────────────────┤
│  Input:  ComputeInput  = { vertex_id, value: Vec<u8>, edges, messages }     │
│  Output: ComputeResultWire = { new_value: Option<Vec<u8>>, outgoing }       │
│  Serialization: bincode (or JSON/msgpack for non-Rust)                      │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Runtime (pregel-worker)                                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│  • Native:  Calls Rust fn directly (ComputeInput → ComputeResultWire)        │
│  • WASM:    Loads .wasm, calls compute(ptr,len,ptr,len) → output_len        │
│  • Host writes input to linear memory, calls export, reads output            │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 1. The SDK Layer (Per Language)

### Rust SDK (`pregel-sdk`)

**Trait:** `VertexProgram`

```rust
pub trait VertexProgram: Send + Sync {
    type VertexValue: Send + Sync;
    type Message: Send + Sync + Clone;

    fn compute(
        &mut self,
        vertex: &mut Vertex<Self::VertexValue>,
        messages: &[Self::Message],
        ctx: &mut Context<Self::Message>,
    );
}
```

- **Vertex<V>**: id, value, edges
- **Context<M>**: `send(target, msg)`, `superstep()`
- Typed, ergonomic API. Algorithm author never touches raw bytes.

**Bridge to ABI:** An adapter (`vertex_program_compute`) converts:

1. `ComputeInput` (bytes) → deserialize value/messages → `Vertex<V>` + `Vec<M>`
2. Call `program.compute(&mut vertex, &messages, &mut ctx)`
3. `ctx.outgoing` + `vertex.value` → `ComputeResultWire` → serialize → bytes

This adapter is used for:
- **Native path**: Worker calls it instead of handwritten `pagerank_compute`, etc.
- **WASM path**: A thin wrapper compiles to WASM; it imports nothing, just exports `compute`.

### AssemblyScript / Go

- **AssemblyScript** (`sdk/assemblyscript`): TypeScript-like syntax, compiles to WASM with `asc`. Implements the ABI directly (bincode decode/encode).
- **Go** (`sdk/go`): Compiles to WASM with `tinygo -target=wasm` or `GOOS=js GOARCH=wasm`. Implement the `compute` export, bincode-compatible I/O.

### Python

Python does not practically produce small WASM modules (Pyodide ships the whole interpreter). For Pregel, implement algorithms in Rust, Go, or AssemblyScript.

---

## 2. From SDK Code to WASM

### Rust: Two Paths

| Path | Use Case | How |
|------|----------|-----|
| **Native** | Fast dev, debugging | `vertex_loop` calls `vertex_program_compute(program, input)` directly |
| **WASM** | Sandboxed, multi-tenant, cross-language | Build with `--target wasm32-unknown-unknown`, export `compute` |

**Rust → WASM flow:**

1. Author implements `VertexProgram` (e.g. `struct CcAlgo; impl VertexProgram for CcAlgo { ... }`).
2. SDK provides a macro or helper:

   ```rust
   // In crate that will be compiled to WASM
   pregel_sdk::export_compute!(CcAlgo);
   ```

   This expands to a `#[no_mangle] pub extern "C" fn compute(...)` that:
   - Deserializes `ComputeInput` from the input pointer
   - Constructs `Vertex`, `Context`, messages
   - Calls `CcAlgo.compute(...)`
   - Serializes result to `ComputeResultWire`, writes to output pointer

3. Build: `cargo build -p pregel-wasm-algos --target wasm32-unknown-unknown --release`
4. Run: `pregel submit --graph X --program target/wasm32-unknown-unknown/release/libpregel_wasm_algos.wasm`
   (cdylib produces `lib<crate_name>.wasm` with hyphens → underscores)

### AssemblyScript / Go / TypeScript

- **AssemblyScript**: Full support. `sdk/assemblyscript` compiles to `.wasm`. Edit `assembly/index.ts`, run `npm run asbuild:release`, use with `pregel submit --program build/algo.release.wasm`.
- **Go**: `tinygo build -target=wasm` or `GOOS=js GOARCH=wasm`. Implement `compute` export with bincode I/O.
- **TypeScript**: Use `sdk/typescript` for local testing with `runCompute()`. For WASM, use AssemblyScript.

---

## 3. The ABI (WASM Contract)

Defined in `docs/ABI_SPEC.md` and `docs/WASM_ABI.md`. Summary:

**Export:** `compute(input_ptr: i32, input_len: i32, output_ptr: i32, output_max_len: i32) -> i32`

**Input (bincode):** `ComputeInput`

```rust
struct ComputeInput {
    vertex_id: u64,
    value: Vec<u8>,           // Serialized VertexValue
    edges: Vec<u64>,
    messages: Vec<(u64, Vec<u8>)>,  // (source, serialized Message)
}
```

**Output (bincode):** `ComputeResultWire`

```rust
struct ComputeResultWire {
    new_value: Option<Vec<u8>>,   // Updated vertex value, or None to halt
    outgoing: Vec<(u64, Vec<u8>)>, // (target, serialized Message)
}
```

**Memory:** Host writes input at offset 0, reads output at offset 32KB. Module exports `memory`.

---

## 4. SDK Crate Structure

```
pregel-sdk/
├── src/
│   ├── lib.rs           # Re-exports
│   ├── program.rs       # VertexProgram trait
│   ├── vertex.rs        # Vertex<V>
│   ├── context.rs       # Context<M>
│   ├── message.rs       # Message marker trait
│   ├── aggregator.rs    # Aggregator<V,R> (future)
│   └── wire.rs          # Adapter: VertexProgram + ComputeInput/ResultWire
├── Cargo.toml
└── README.md
```

**wire.rs** (adapter):

- `vertex_program_compute<P>(program: &mut P, input: &ComputeInput, superstep: u64) -> ComputeResultWire`
- Requires `P: VertexProgram` where `VertexValue: Serialize + Deserialize`, `Message: Serialize + Deserialize`
- Used by native path and by the WASM export macro

---

## 5. Native vs WASM Dispatch

In `vertex_loop.rs`:

```rust
let result = if let (Some(exec), Some(modu)) = (wasm_executor, wasm_module) {
    // WASM path: host calls module
    let bytes = bincode::serialize(&input).unwrap();
    let output = exec.compute(modu, &bytes).unwrap_or_default();
    bincode::deserialize(&output).unwrap_or_default()
} else {
    // Native path: call Rust directly
    // Option A: handwritten native_algo (current)
    // Option B: vertex_program_compute(&mut sdk_program, &input)
    native_algo::connected_components_compute(&input)
};
```

Eventually: native path can use SDK-backed algorithms (e.g. from `examples/connected_components`) or keep handwritten for performance-critical built-ins.

---

## 6. Cross-Language Summary

| Language       | SDK                 | → WASM                          | ABI  |
|----------------|---------------------|----------------------------------|------|
| Rust           | `crates/pregel-sdk` | `export_wasm_compute!`          | ✅   |
| AssemblyScript | `sdk/assemblyscript`| `asc` → .wasm                   | ✅   |
| Go             | `sdk/go`            | tinygo / GOOS=js GOARCH=wasm    | ✅   |
| TypeScript     | `sdk/typescript`    | Local testing only; use AS for WASM | — |

**One runtime, many languages.** The ABI is the universal contract.
