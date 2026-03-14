# Testing Guide

How to run automated tests, benchmarks, and manual end-to-end validation.

**Prerequisite:** Run all commands from the `pregel/` directory (workspace root).

**Troubleshooting:** If submit hangs or you see "Address already in use", kill old processes first:
```bash
pkill -f pregel-worker; pkill -f pregel-coordinator
```
See `docs/FAQ_AND_TROUBLESHOOTING.md` for more.

---

## Verbose Output

Use `-v`, `--verbose`, or `--verbose=2` for increasing detail:

- **Level 0** (default): Quiet
- **Level 1** (`-v`): Summary (superstep progress, coordinator advances)
- **Level 2** (`--verbose=2`): Full dumps (inbox, vertex states, outgoing messages, DBG phase markers)

Verbose applies to both coordinator and workers. Works with or without `--metrics-port`.

```bash
cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo cc -v
cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo cc --verbose=2
```

---

## 1. Automated Tests

### Run All Tests (workspace)

```bash
cd pregel
cargo test
```

### Run Tests by Crate

| Crate | Command | What It Tests |
|-------|---------|---------------|
| pregel-worker | `cargo test -p pregel-worker` | Native algo compute logic (PageRank, CC, SSSP), message shapes |
| pregel-wasm | `cargo test -p pregel-wasm` | WASM CC smoke (requires WASM built first) |
| pregel-core | `cargo test -p pregel-core` | Partition logic |
| pregel-storage | `cargo test -p pregel-storage` | Graph loading |
| pregel-checkpoint | `cargo test -p pregel-checkpoint` | Checkpoint save/load |

### Unit Tests (in source)

- **`pregel-worker/src/native_algo.rs`** – `#[test]` blocks for CC and PageRank compute (real logic, no mocks)
- **`pregel-worker/tests/algo_messages.rs`** – Message format and `execute_superstep_parallel` behavior

### Ignored / doc tests

- **`pregel-sdk` doc tests** – Examples in `///` docs use `ignore` because they need full runtime setup. The examples in `examples/` are the canonical tests.

### WASM Tests

WASM smoke tests require building the WASM module first:

```bash
cargo build -p pregel-wasm-algos --target wasm32-unknown-unknown --release
cargo test -p pregel-wasm
```

---

## 2. Benchmarks

### Run All Benchmarks

```bash
cargo bench -p pregel-worker
```

### Individual Benchmarks

- **`cc_superstep_0_small`** – CC superstep 0 (no messages), ~18 µs
- **`cc_superstep_1_with_messages`** – CC with inbox, ~16 µs
- **`pagerank_superstep_1`** – PageRank superstep 1, ~400 ns (fewer vertices with messages)

Results show median time per superstep. See `docs/FAQ_AND_TROUBLESHOOTING.md` for interpretation.

---

## 3. Manual Testing — Quick Reference

### One-Command Flow (single-shot)

```bash
# CC on sample graph (fast, converges quickly)
cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo cc --workers 2

# PageRank (runs until halt; may need --max-supersteps or similar in future)
cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo pagerank --workers 2

# Shortest path
cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo shortest_path --workers 2

# With metrics ( scrape while running )
cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo cc --workers 2 --metrics-port 9090
# In another terminal: curl http://127.0.0.1:9090/metrics
```

### Session + Job Flow (two terminals)

**Terminal 1 — start session:**
```bash
cargo run -p pregel-cli -- session --graph examples/data/sample.graph --workers 2 --metrics-port 9090
```

**Terminal 2 — submit jobs:**
```bash
cargo run -p pregel-cli -- job --session http://127.0.0.1:5100 --algo cc
cargo run -p pregel-cli -- job --session http://127.0.0.1:5100 --algo pagerank
```

Stop session: `Ctrl+C` in Terminal 1.

### Standalone Binaries (for debugging)

```bash
# Coordinator (expects workers to connect)
./target/debug/pregel-coordinator 127.0.0.1:5000 2 --http-port 5100

# Worker (manual, one per terminal)
./target/debug/pregel-worker 0 http://127.0.0.1:5000 examples/data/sample.graph 2 5001 --session
./target/debug/pregel-worker 1 http://127.0.0.1:5000 examples/data/sample.graph 2 5002 --session
```

Then submit from CLI or `curl`:
```bash
curl -X POST http://127.0.0.1:5100/jobs -H "Content-Type: application/json" -d '{"algo":"cc","program":""}'
```

---

## 4. Manual Testing — Full Checklist

### Preflight

```bash
cd pregel
cargo build
```

### A. CLI — Single-shot (submit)

1. **CC**
   ```bash
   cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo cc --workers 2
   ```
   - Expect: Job runs, prints **Result** with vertex IDs and values (CC labels).

2. **PageRank**
   ```bash
   cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo pagerank --workers 2
   ```
   - Expect: Result with vertex ranks (f64). Halts on convergence (ε=1e-6) or max 200 supersteps.

3. **Shortest path**
   ```bash
   cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo shortest_path --workers 2
   ```

4. **With metrics**
   ```bash
   cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo cc --metrics-port 9090
   ```
   While running, in another terminal:
   ```bash
   curl http://127.0.0.1:9090/metrics
   curl http://127.0.0.1:9091/metrics
   ```

5. **With checkpoint dir**
   ```bash
   mkdir -p /tmp/pregel-ckpt
   cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo cc --checkpoint-dir /tmp/pregel-ckpt
   ```
   After run: `ls /tmp/pregel-ckpt` should show `.ckpt` files.

6. **Transport: QUIC**
   ```bash
   cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo cc --transport quic --workers 2
   ```

### B. CLI — Session + Job

1. **Terminal 1**
   ```bash
   cargo run -p pregel-cli -- session --graph examples/data/sample.graph --workers 2 --metrics-port 9090
   ```

2. **Terminal 2** (after "Session ready")
   ```bash
   cargo run -p pregel-cli -- job --session http://127.0.0.1:5100 --algo cc
   cargo run -p pregel-cli -- job --session http://127.0.0.1:5100 --algo pagerank
   ```

3. **Terminal 1:** `Ctrl+C` to stop.

### C. Graph Data

- `examples/data/sample.graph` — small graph
- `examples/data/multi_cc.graph` — 3 connected components (0–3, 4–6, 7)

```bash
cargo run -p pregel-cli -- submit --graph examples/data/multi_cc.graph --algo cc --workers 2
```

### D. Help and Flags

```bash
cargo run -p pregel-cli -- --help
cargo run -p pregel-cli -- session --help
cargo run -p pregel-cli -- job --help
cargo run -p pregel-cli -- submit --help
cargo run -p pregel-cli -- build --help
cargo run -p pregel-cli -- cluster --help
```

### E. SDK / Examples

**Rust examples (run after build):**
```bash
cargo build -p connected-components-example
cargo run -p connected-components-example   # Runs adapter test, prints "OK"

cargo build -p pagerank-example
cargo run -p pagerank-example

cargo build -p shortest-path-example
cargo run -p shortest-path-example
```

These examples implement `VertexProgram` and run a quick adapter test. To run the actual algorithm on a graph, use the CLI: `pregel submit --algo cc|pagerank|shortest_path`.

**Build WASM for use with `--program`:**
```bash
# Built-in WASM algos (CC, etc.)
cargo build -p pregel-wasm-algos --target wasm32-unknown-unknown --release
cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --algo cc --program target/wasm32-unknown-unknown/release/pregel_wasm_algos.wasm

# Custom SDK project (if it has lib.rs for WASM)
cargo run -p pregel-cli -- build <path-to-crate>
```

**TypeScript SDK (local testing):**
```bash
cd sdk/typescript
npm install
npm run build
# Use runCompute(program, input) for local algo testing
```

**Go SDK:**
```bash
cd sdk/go
go build
# Build for WASM: GOOS=js GOARCH=wasm go build -o algo.wasm .
```

**Full SDK test script:**
```bash
./scripts/test-sdks.sh   # Rust, AssemblyScript, TypeScript, Go
```

### F. Coordinator and Worker (standalone)

1. **Coordinator**
   ```bash
   ./target/debug/pregel-coordinator 127.0.0.1:5000 2 --http-port 5100 --worker-timeout 60
   ```

2. **Worker 0** (new terminal)
   ```bash
   ./target/debug/pregel-worker 0 http://127.0.0.1:5000 examples/data/sample.graph 2 5001 --session
   ```

3. **Worker 1** (new terminal)
   ```bash
   ./target/debug/pregel-worker 1 http://127.0.0.1:5000 examples/data/sample.graph 2 5002 --session
   ```

4. **Submit job**
   ```bash
   curl -X POST http://127.0.0.1:5100/jobs -H "Content-Type: application/json" -d '{"algo":"cc","program":""}'
   ```

---

## 5. Observability in Tests

To capture `ObservableEvent` in unit tests:

```rust
let backend = TestObserver::new();
pregel_observability::set_observer_for_test(Observer::test(backend.clone()));
// ... run worker loop ...
let events = backend.events();
assert!(events.iter().any(|e| matches!(e, ObservableEvent::SuperstepStarted { .. })));
```

---

## 6. pregel-sdk Crate vs sdk/ Folder

| Location | Purpose |
|----------|---------|
| `crates/pregel-sdk` | Rust crate. Used by examples (connected_components, pagerank, shortest_path) and `pregel-wasm-algos`. Defines `VertexProgram`, `Context`, `vertex_program_compute`. |
| `sdk/typescript` | Standalone npm package. TypeScript types and `runCompute()` for local algo testing. Not connected to the Rust crate by code. |
| `sdk/go` | Go SDK. Implement `VertexProgram` in Go; build to WASM for runtime. |
| `sdk/assemblyscript` | AssemblyScript SDK. Compiles to WASM for use with `--program`. |

All implement the same ABI/contract (see `docs/ABI_SPEC.md`).

---

## 7. gRPC, QUIC, and TCP — Low-Level Overview

### TCP (Transmission Control Protocol)
- **Layer:** Transport (Layer 4). Reliable, ordered byte stream between two endpoints.
- **How it works:** Three-way handshake (SYN, SYN-ACK, ACK), then data flows; lost packets are retransmitted; connection-oriented.
- **In Pregel:** Workers use TCP for **inter-worker message transport** when `--transport tcp`. Raw byte streams between workers (e.g. port 5001 ↔ 5002).

### QUIC (Quick UDP Internet Connections)
- **Layer:** Transport over UDP. Modern alternative to TCP+TLS.
- **How it works:** Uses UDP as the carrier (no kernel connection state); multiplexing and encryption built-in; faster connection setup; handles packet loss and reordering.
- **In Pregel:** Workers use QUIC for **inter-worker message transport** when `--transport quic`. Same logical role as TCP but different wire protocol.

### gRPC
- **Layer:** Application (Layer 7). An RPC framework, not a transport.
- **How it works:** Uses HTTP/2 for transport (which typically uses TCP or QUIC under the hood). Defines request/response and streaming semantics. Encodes messages (e.g. Protobuf).
- **In Pregel:** gRPC is used for **coordinator ↔ worker** communication only. Workers connect to the coordinator over HTTP/2 (backed by TCP). Commands: `RegisterWorker`, `ReportSuperstepDone`, `WaitForAdvance`, `WaitForJobStart`, `ReportJobResults`, etc.

### Summary

| Concern | Technology |
|---------|------------|
| Coordinator ↔ Worker | gRPC (HTTP/2 over TCP) |
| Worker ↔ Worker (messages) | TCP or QUIC, chosen by `--transport` |

---

## 8. Troubleshooting

| Issue | Check |
|-------|-------|
| `pregel-worker not found` | Run `cargo build -p pregel-worker -p pregel-coordinator` |
| `Graph file not found` | Use path from `pregel/` or absolute path |
| `Connection refused` | Ensure coordinator is running before workers |
| `Address already in use` | Kill stale processes: `pkill -f pregel-worker; pkill -f pregel-coordinator` |
| Metrics empty | Scrape while job is running |
| Verbose not showing | Use `-v` or `--verbose=2`; output appears on stderr |
| Submit hangs | Ensure WASM built if using `--program`; check worker/coordinator logs |
| Session "Timeout expired" | Fixed: gRPC timeout increased to 24h for `wait_for_job_start` |
| Go SDK build error | Fixed: `TotalVertices` field renamed to `TotalVerticesN` |
