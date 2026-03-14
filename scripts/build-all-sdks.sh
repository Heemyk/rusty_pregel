#!/bin/bash
# Build all Pregel SDKs and their WASM outputs. Run from pregel/ directory.
set -e
cd "$(dirname "$0")/.."

echo "=== Building Pregel SDKs ==="

echo ""
echo "--- 1. Rust WASM (pregel-wasm-algos) ---"
cargo build -p pregel-wasm-algos --target wasm32-unknown-unknown --release 2>&1
RUST_WASM="target/wasm32-unknown-unknown/release/pregel_wasm_algos.wasm"
if [ -f "$RUST_WASM" ]; then
  echo "  -> $RUST_WASM"
else
  echo "  Failed"
  exit 1
fi

echo ""
echo "--- 2. AssemblyScript ---"
cd sdk/assemblyscript
if [ ! -d node_modules ]; then npm install 2>/dev/null || true; fi
npm run asbuild:release 2>&1
cd ../..
if [ -f sdk/assemblyscript/build/algo.release.wasm ]; then
  echo "  -> sdk/assemblyscript/build/algo.release.wasm"
else
  echo "  Failed"
  exit 1
fi

echo ""
echo "--- 3. TypeScript (Node build) ---"
cd sdk/typescript
if [ ! -d node_modules ]; then npm install 2>/dev/null || true; fi
npm run build 2>&1
cd ../..
if [ -f sdk/typescript/dist/index.js ]; then
  echo "  -> sdk/typescript/dist/index.js"
else
  echo "  Failed"
  exit 1
fi

echo ""
echo "--- 4. Go (types check) ---"
cd sdk/go
go build ./... 2>&1
cd ../..
echo "  -> Go types OK"

echo ""
echo "=== SDK builds complete ==="
echo ""
echo "WASM outputs:"
echo "  Rust:   $RUST_WASM"
echo "  AS:     sdk/assemblyscript/build/algo.release.wasm"
echo ""
echo "E2E test: cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo cc --program $RUST_WASM"
