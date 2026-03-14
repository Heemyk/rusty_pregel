# Pregel Makefile. Run from pregel/ directory.

.PHONY: build test bench sdks build-sdks e2e-cc e2e-cc-wasm clean

# Build all crates
build:
	cargo build

# Run all tests
test:
	cargo test

# Run benchmarks
bench:
	cargo bench -p pregel-worker

# Build all SDKs (Rust WASM, AssemblyScript, TypeScript, Go)
sdks: build-sdks

build-sdks:
	./scripts/build-all-sdks.sh

# Quick E2E: CC native on sample graph
e2e-cc:
	cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo cc --workers 2

# E2E: CC with Rust WASM (build WASM first)
e2e-cc-wasm: build
	cargo build -p pregel-wasm-algos --target wasm32-unknown-unknown --release
	cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo cc --workers 2 \
		--program target/wasm32-unknown-unknown/release/pregel_wasm_algos.wasm -v

# E2E: CC with AssemblyScript WASM
e2e-cc-as:
	cd sdk/assemblyscript && npm run asbuild:release && cd ../..
	cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo cc --workers 2 \
		--program sdk/assemblyscript/build/algo.release.wasm -v

# Run SDK test script
test-sdks:
	./scripts/test-sdks.sh

# Clean build artifacts
clean:
	cargo clean
