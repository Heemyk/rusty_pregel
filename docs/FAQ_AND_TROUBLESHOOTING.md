# FAQ & Troubleshooting

## Why does submit hang?

**Symptoms:** `Submitting job (algo=cc)...` appears but nothing happens.

**Causes & fixes:**

1. **Address already in use** – A previous session is still running. Kill it first:
   ```bash
   pkill -f pregel-worker
   pkill -f pregel-coordinator
   ```
2. **Workers not ready** – The CLI waits 1.5s for workers to register. On slow machines, increase this (edit `run_submit` in pregel-cli).
3. **CC / PageRank running 200 steps** – CC may hit the 200-superstep cap if messages are delayed. PageRank should converge (ε=1e-6). Use `-v` to see progress.

## Why "Address already in use"?

Ports 5000 (gRPC) and 5100 (HTTP) are in use by a previous run. Always stop the session (Ctrl+C) before starting a new one, or run `pkill -f pregel`.

## Why do some tests show "ignored"?

**pregel-sdk doc tests** – The `#[doc = "..."]` examples use a simplified API. They are `ignore`d because they require a full runtime. The actual unit tests in `pregel-worker` and examples run the real logic.

**pregel-worker benches** – Some `#[test]` in benches are `ignore`d by convention; benchmarks are run with `cargo bench`, not `cargo test`.

## Are tests mocked or real?

| Test type | Real or mock |
|-----------|--------------|
| `native_algo.rs` unit tests | Real compute, in-memory inputs |
| `algo_messages.rs` | Real `execute_superstep_parallel`, in-memory partition |
| `wasm_cc_smoke`, `wasm_as_cc_smoke` | Real WASM execution via wasmtime |
| Example adapter tests (e.g. CC) | Real `vertex_program_compute`, single-vertex in-memory |
| `test-sdks.sh` | Adapter + compile checks; AssemblyScript E2E is optional |
| Full `pregel submit` | Real coordinator, workers, network |

**No mocks** – All tests use real implementations. Integration tests use real processes.

## What do benchmark results mean?

```
cc_superstep_0_small      time: [17.681 µs 17.939 µs 18.148 µs]
cc_superstep_1_with_messages  time: [15.896 µs 15.991 µs 16.080 µs]
pagerank_superstep_1     time: [397.12 ns 397.92 ns 398.76 ns]
```

- **Median** (middle value): typical latency for one superstep.
- **µs** = microseconds; **ns** = nanoseconds.
- `pagerank_superstep_1` is faster because it runs fewer vertices (only those with messages).

## Why did CC hit 200 supersteps?

The sample graph (5 vertices, one component) should converge in ~3–5 steps. Hitting 200 suggests:

1. **Message timing** – 150ms drain window might be too short on slow systems.
2. **Partitioning** – With 2 workers, cross-worker message latency can delay convergence.
3. **Bug** – If you see incorrect results (e.g. not all component 0), file an issue.

The coordinator forces halt at 200 steps as a safety cap. Results are still returned.

## SDK build commands

| SDK | Build command | Output |
|-----|---------------|--------|
| Rust (wasm) | `cargo build -p pregel-wasm-algos --target wasm32-unknown-unknown --release` | `target/wasm32-unknown-unknown/release/pregel_wasm_algos.wasm` |
| AssemblyScript | `cd sdk/assemblyscript && npm run asbuild:release` | `sdk/assemblyscript/build/algo.release.wasm` |
| TypeScript | `cd sdk/typescript && npm run build` | `sdk/typescript/dist/index.js` (Node) |
| Go | `cd sdk/go && go build ./...` | Types only; no WASM from Go yet |

Use `./scripts/build-all-sdks.sh` or `make sdks` for a unified build.

## test-sdks.sh: adapter vs E2E

| Step | What it does | E2E? |
|------|--------------|------|
| Rust (connected_components) | Runs adapter test (single-vertex compute) | No – unit-level |
| AssemblyScript | Builds WASM, prints E2E command | No – build only |
| TypeScript | `runCompute` in-process with mock CC | No – adapter |
| Go | `go build` type check | No – compile only |

**To run full E2E** (WASM correctness on real graph):

```bash
# Rust WASM
cargo build -p pregel-wasm-algos --target wasm32-unknown-unknown --release
cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo cc \
  --program target/wasm32-unknown-unknown/release/pregel_wasm_algos.wasm -v

# AssemblyScript WASM
make e2e-cc-as
```

Expected result for sample.graph (one component): all vertices `value: 0`.
