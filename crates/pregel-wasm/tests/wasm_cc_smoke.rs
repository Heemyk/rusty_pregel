//! Smoke test: run WASM CC with known input, verify non-empty output.
//! Requires: cargo build -p pregel-wasm-algos --target wasm32-unknown-unknown --release

use pregel_common::{ComputeInput, ComputeResultWire};
use pregel_wasm::{WasmExecutor, WasmModule};
use std::path::Path;

#[test]
fn wasm_cc_produces_outgoing_in_superstep0() {
    let wasm_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/wasm32-unknown-unknown/release/pregel_wasm_algos.wasm");
    if !wasm_path.exists() {
        eprintln!("Skipping: build WASM first: cargo build -p pregel-wasm-algos --target wasm32-unknown-unknown --release");
        return;
    }

    let module = WasmModule::from_path(&wasm_path).unwrap();
    let executor = WasmExecutor::new();

    let input = ComputeInput {
        vertex_id: 0,
        value: bincode::serialize(&0u64).unwrap(),
        edges: vec![1, 2],
        messages: vec![],
    };
    let serialized = bincode::serialize(&input).unwrap();
    let output = executor.compute(&module, &serialized).unwrap();

    assert!(!output.is_empty(), "WASM should return non-empty output for superstep 0");
    let wire: ComputeResultWire = bincode::deserialize(&output).expect("output must deserialize");
    assert!(!wire.outgoing.is_empty(), "CC superstep 0 must send to neighbors");
    assert_eq!(wire.outgoing.len(), 2, "vertex 0 has 2 edges");
}
