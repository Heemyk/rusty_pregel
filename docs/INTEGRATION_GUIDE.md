# Pregel SDK Integration Guide

How to use the SDK across languages and integrate with the Rust runtime.

---

## Quick Start (Rust)

```bash
cd pregel
cargo run -p pregel-cli -- init my-algo
# Add my-algo to workspace in Cargo.toml
cargo run -p pregel-cli -- build my-algo
cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --program target/wasm32-unknown-unknown/release/libmy_algo.wasm --algo cc
```

---

## Language SDKs

| Language       | Path               | WASM Path        |
|----------------|--------------------|------------------|
| Rust           | `crates/pregel-sdk` | ✅ `export_wasm_compute!(T)` |
| AssemblyScript | `sdk/assemblyscript` | ✅ `asc` → .wasm |
| Go             | `sdk/go`           | ✅ `tinygo -target=wasm` or `GOOS=js GOARCH=wasm` |
| TypeScript     | `sdk/typescript`   | Local testing only; use AssemblyScript for WASM |

---

## ABI Contract

All runtimes and modules speak the same wire format. See **`docs/ABI_SPEC.md`** for:

- `ComputeInput` / `ComputeResultWire` schema
- Error codes (`EINVALID`, `EDESERIALIZE`, etc.)
- Memory layout and security

---

## Rust Integration

### Native path (built-in algos)

The worker uses `native_algo::*_compute()` for `cc`, `pagerank`, `shortest_path`. No SDK involved.

### WASM path

1. Create a crate with `[lib] crate-type = ["cdylib"]`
2. Add `pregel-sdk`, `pregel-common`, `bincode`
3. Implement `VertexProgram` and `Default`
4. Add `pregel_sdk::export_wasm_compute!(YourAlgo);`
5. Build: `cargo build --target wasm32-unknown-unknown --release`
6. Run: `pregel submit --program path/to/lib*.wasm --algo cc`

### CLI Commands

| Command | Purpose |
|---------|---------|
| `pregel build [path]` | Build WASM from Cargo project |
| `pregel init <name>` | Scaffold a new algo crate |
| `pregel submit --graph X --program P --algo cc` | Single-shot run |
| `pregel session --graph X` | Start session; submit jobs with `pregel job` |

---

## AssemblyScript Integration

```bash
cd sdk/assemblyscript && npm install && npm run asbuild:release
```

Edit `assembly/index.ts` to implement your algorithm. The `compute` export is called by the runtime. Uses bincode for input/output (same as Rust).

---

## Go Integration

Implement `VertexProgram[V, M]` and build:

```bash
tinygo build -target=wasm -o algo.wasm .
```

Ensure the module exports `compute` with the ABI signature. Use `encoding/binary` for little-endian u64 serialization to match bincode.

---

## TypeScript Integration

Use `runCompute(program, input)` for local tests. For production WASM, use **AssemblyScript** (`sdk/assemblyscript`) — it compiles TypeScript-like code to WASM.

---

## Error Handling

### WASM guest errors

When the guest returns a negative `compute` result, the host surfaces:

```
Error: WASM guest error: EDESERIALIZE (ABI code -2)
```

| Code | Meaning |
|------|---------|
| -1 | Invalid arguments |
| -2 | Deserialize input failed |
| -3 | Serialize output failed |
| -4 | Output buffer too small |
| -5 | Allocation failed |
| -6 | User algorithm error |

### Host validation

The worker validates `ComputeInput` before invoking WASM. Oversized values/edges/messages are rejected.
