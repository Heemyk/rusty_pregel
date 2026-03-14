# Pregel AssemblyScript SDK

Write vertex algorithms in **AssemblyScript** (TypeScript-like) and compile to WASM.

## Setup

```bash
cd sdk/assemblyscript
npm install
```

## Build

```bash
npm run asbuild:release
```

Output: `build/algo.release.wasm`

## Run

```bash
cd ../../  # back to pregel/
cargo run -p pregel-cli -- submit --graph examples/data/sample.graph \
  --program sdk/assemblyscript/build/algo.release.wasm --algo cc
```

## How It Works

- AssemblyScript compiles to WebAssembly. The `compute` export matches the Pregel ABI.
- Input/output use **bincode** (same as Rust) for compatibility with the runtime.
- Edit `assembly/index.ts` to implement your algorithm. The included example is Connected Components.

## ABI

See `docs/ABI_SPEC.md` for the full contract. Signature:

```
compute(input_ptr: i32, input_len: i32, output_ptr: i32, output_max_len: i32) -> i32
```

Returns bytes written, or negative error code.
