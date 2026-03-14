# Pregel Roadmap

Prioritized plan for next features and improvements.

---

## Phase 1: Algos & Testing (Current Focus)

### 1.1 Shortest Path (Single-Source) — Native ✅
- [x] Add `ShortestPath` to `Algorithm` enum
- [x] Implement `shortest_path_compute` in `native_algo.rs`
  - Value: distance from source (u64; `u64::MAX` = infinity)
  - Message: sender's distance; receiver computes `min(current, min(messages) + 1)`
  - Source vertex: id passed via initial value or fixed (e.g. vertex 0)
- [x] Wire through `vertex_loop.rs` and CLI `--algo shortest_path|sssp`
- [x] Update example in `examples/shortest_path`

### 1.2 Bigger Test Graph (Multiple CCs) ✅
- [x] Create `examples/data/multi_cc.graph` — 3 components (0–3, 4–6, 7)
- [ ] Use for CC regression and stress tests

### 1.3 Benchmarks (Reusable Timing) ✅
- [x] Add `cargo bench` in pregel-worker (cc_superstep, pagerank_superstep)
- [x] `pregel_observability::measure(f)` returns (result, Duration) for reusable timing

### 1.4 PageRank: Convergence / Halt
- [ ] PageRank currently runs indefinitely (never votes to halt)
- [ ] Add halt condition: e.g. max iterations (`--max-supersteps`) or convergence (delta < ε)
- [ ] Vote-to-halt when rank changes below threshold, or cap at N supersteps

---

## Phase 2: WASM Path End-to-End

### 2.1 WASM CC or PageRank Module ✅
- [x] Build a WASM module (`pregel-wasm-algos`) that implements ComputeInput → ComputeResultWire ABI
- [ ] Use pregel-sdk or wasm-bindgen to produce compatible `.wasm`
- [ ] Document ABI in `docs/WASM_ABI.md` (if not done)

### 2.2 Wire WASM Through Engine ✅
- [x] Worker loads WASM from path when `--program <path>` or `--wasm` provided
- [x] Pass `WasmExecutor` + `WasmModule` into `execute_superstep_parallel`
- [x] WASM returns `ComputeResultWire` (new_value + outgoing), bincode-serialized

### 2.3 CLI `--wasm` / `--program` Path ✅
- [x] `pregel submit --graph X --program <path>` or `--wasm <path>` (alias)
- [x] Worker receives `--program`, loads module at startup; uses WASM instead of native when set

---

## Phase 3: Observability (Prometheus & Logging)

### 3.1 Prometheus Hooks (Flesh Out)
- [x] Ensure key events emit `ObservableEvent`:
  - SuperstepStarted / SuperstepCompleted (with duration_ms)
  - MessagesSent (count, bytes)
  - VerticesComputed
  - BatchesReceived (verbose=2)
- [ ] Add `WorkerRegistered`, `WorkerReported` if useful for coordinator metrics
- [ ] Worker health (heartbeat / last-seen) — coordinator metrics

### 3.2 Prometheus Exporter ✅
- [x] Add `PrometheusObserver` in pregel-observability
- [x] Metrics:
  - `pregel_superstep_duration_seconds` (histogram, labels: worker_id)
  - `pregel_messages_sent_total` (counter, labels: worker_id)
  - `pregel_vertices_computed_total` (counter, labels: worker_id)
  - `pregel_checkpoints_saved_total` (counter, labels: worker_id)
- [x] HTTP endpoint `/metrics` for Prometheus scrape (axum)
- [x] Worker `--metrics-port <port>`; CLI `--metrics-port <base>` (workers get base, base+1, ...)
- [x] SO_REUSEADDR on metrics listener for quick restarts
- [ ] Note: scrape while job runs — metrics server exits when workers exit

### 3.3 Structured Logging
- [ ] Add `tracing` (or `log` + structured fields)
- [ ] Replace ad-hoc `eprintln!` with structured spans/events
- [ ] JSON output option for production (`RUST_LOG=...` or `--log-format json`)

---

## Phase 4: Fault Tolerance

### 4.1 Failure Detection ✅
- [x] Coordinator: detect worker timeout (no report within N seconds); `--worker-timeout` (default 60s)
- [x] Worker: gRPC request timeout 60s; coordinator unreachable → exit with error
- [x] Failure semantics: on worker timeout, coordinator aborts job (advance to terminate); workers exit

### 4.2 Recovery from Checkpoint
- [ ] On worker failure: coordinator instructs remaining workers to load from last checkpoint
- [ ] Reassign failed worker's partition (to another worker or new process)
- [ ] Resume from checkpoint superstep
- [ ] Requires: checkpoint format supports partition metadata, coordinator knows partition map

### 4.3 Checkpoint Format & Partition Reassignment
- [ ] Checkpoint stores partition id + vertices; coordinator can reassign
- [ ] Protocol: `RecoverFromCheckpoint(checkpoint_step, new_partition_map)`
- [ ] Workers reload partition from checkpoint dir, advance to next superstep

---

## Phase 5: Result Aggregation

Extend algorithm metadata with **query** (per-worker extract) and **post-function** (coordinator combine) so the coordinator can return computation results to the client instead of only `{job_id, algo}`.

### 5.1 Algo Metadata: Query + Post-Function
- [ ] Extend `Algorithm` / add `AlgoMetadata` with:
  - **Query**: what to run on each worker's partition to extract "my piece" of the result
  - **Post-function**: how the coordinator combines worker outputs into the final answer
- [ ] Example mappings:
  - **CC**: Query = all `(vertex_id, component_id)` | Post = concat, sort by vid
  - **PageRank**: Query = all `(vertex_id, score)` | Post = concat or top-K
  - **ShortestPath**: Query = `(vertex_id, distance)` or subset | Post = concat or single value

### 5.2 Protocol & Implementation
- [ ] Coordinator: after all workers halt, request results per worker (gRPC or HTTP)
- [ ] Workers: implement query on local partition → return `Vec<(VertexId, Vec<u8>)>` (or typed)
- [ ] Coordinator: apply post-function → return JSON in `POST /jobs` response or `GET /jobs/:id/results`
- [ ] CLI: `pregel job` prints or streams the result

### 5.3 ResultQuery / PostFunction Types (sketch)
```rust
pub enum ResultQuery {
    AllVertexValues,              // CC, PR: full (vid, value) pairs
    VertexSubset(Vec<VertexId>),  // SSSP: just source/target
}
pub enum PostFunction {
    ConcatAndSort,   // CC: merge, sort by vid
    Concat,          // PR: merge as-is
    SingleValue,     // SSSP: one distance
}
```

---

## Later / Backlog

- **SDKs**: Flesh out pregel-sdk for Rust, Python (pyo3?), etc.
- **K8s Operator**: Deploy coordinator + workers as pods, manage lifecycle
- **Raft Consensus**: Replace single coordinator with Raft for coordinator HA

---

## Quick Reference: Files to Touch

| Area | Files |
|------|-------|
| Result aggregation | `pregel-core/src/algo.rs`, `pregel-coordinator/`, `pregel-worker/`, proto |
| Algos | `pregel-core/src/algo.rs`, `pregel-worker/src/native_algo.rs`, `vertex_loop.rs` |
| WASM | `pregel-wasm/`, `pregel-worker/src/bin/main.rs`, `vertex_loop.rs` |
| Observability | `pregel-observability/src/lib.rs`, worker/coordinator binaries |
| Checkpoint/Recovery | `pregel-checkpoint/`, `pregel-coordinator/`, proto |
| CLI | `pregel-cli/src/main.rs` |
