#!/bin/sh
# Build WASM CC module. Run from project root.
# Output: target/wasm32-unknown-unknown/release/pregel_wasm_algos.wasm
# Use --target-dir target so the CLI/workers find it.
set -e
rustup target add wasm32-unknown-unknown 2>/dev/null || true
cargo build -p pregel-wasm-algos --target wasm32-unknown-unknown --release --target-dir target
echo "Built: target/wasm32-unknown-unknown/release/pregel_wasm_algos.wasm"
echo "Run: cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo cc --program target/wasm32-unknown-unknown/release/pregel_wasm_algos.wasm"
