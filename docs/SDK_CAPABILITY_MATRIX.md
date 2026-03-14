# SDK Capability Matrix

**Purpose:** Ensure all SDKs (Rust, Go, TypeScript, AssemblyScript) can express the same graph framework features that the CLI and server support. This document maps framework capabilities to SDK surface area and usage.

---

## 1. Framework Features (CLI / Server)

### 1.1 Execution Modes

| Mode | CLI | Description |
|------|-----|-------------|
| **Single-shot** | `pregel submit` | Spawn coordinator + workers, run one job, exit |
| **Session** | `pregel session` + `pregel job` | Graph loaded once; workers stay alive; multiple job submissions via HTTP API |

### 1.2 Transport

| Transport | CLI flag | Description |
|-----------|----------|-------------|
| TCP | `--transport tcp` | Default; inter-worker messages over TCP |
| QUIC | `--transport quic` | QUIC-based messaging |

*SDK impact:* None. Transport is a runtime concern. Vertex programs are transport-agnostic.

### 1.3 Algorithms

| Algo | CLI alias | Built-in native | WASM custom |
|------|-----------|-----------------|--------------|
| Connected Components | cc | ✅ | ✅ |
| PageRank | pagerank, pr | ✅ | ✅ |
| Shortest Path | shortest_path, sssp, sp | ✅ | ✅ |
| Custom | — | — | ✅ `--program algo.wasm` |

### 1.4 Vertex Compute ABI

Every vertex compute (native or WASM) receives:

| Input | Type | Purpose |
|-------|------|---------|
| `vertex_id` | u64 | This vertex's ID |
| `value` | Vec\<u8\> | Current vertex value (serialized) |
| `edges` | Vec\<u64\> | Outgoing edge targets |
| `messages` | Vec\<(u64, Vec\<u8\>)\> | (source, payload) from previous superstep |
| `superstep` | u64 | Current superstep (0-indexed) |
| `total_vertices` | u64 | Total vertices in graph (e.g. PageRank 1/N) |

Output: `ComputeResultWire` = `{ new_value?: Vec<u8>, outgoing: [(target, payload)] }`.

### 1.5 Context API (Typed SDK)

When using the typed `VertexProgram` API (Rust SDK, local TS/Go testing), compute receives a `Context`:

| Method / Field | Purpose |
|----------------|---------|
| `ctx.send(target, msg)` | Send message to vertex at next superstep |
| `ctx.superstep()` | Current superstep number |
| `ctx.total_vertices()` | Total vertices in graph |
| `ctx.aggregate(name, value)` | Global aggregator (stub; runtime not yet implemented) |

### 1.6 Other Framework Features

| Feature | CLI / Server | SDK Impact |
|---------|--------------|------------|
| Checkpointing | `--checkpoint-dir` | Runtime-managed; no SDK API |
| Metrics | `--metrics-port` | Runtime-managed; no SDK API |
| Vote-to-halt | Implicit | Return `new_value: None` (or equivalent) = no update, no outgoing |
| Max supersteps | (future) | Runtime config; no SDK API |

---

## 2. SDK Feature Parity

### 2.1 ComputeInput Fields

| SDK | vertex_id | value | edges | messages | superstep | total_vertices |
|-----|-----------|-------|-------|----------|-----------|----------------|
| Rust | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Go | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| TypeScript | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| AssemblyScript | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

### 2.2 Context API

| SDK | send | superstep | total_vertices | aggregate |
|-----|------|-----------|----------------|-----------|
| Rust | ✅ | ✅ | ✅ | stub |
| Go | ✅ | ✅ | ✅ | stub |
| TypeScript | ✅ | ✅ | ✅ | stub |
| AssemblyScript | (manual) | (from input) | (from input) | — |

AssemblyScript: low-level ABI; no Context object. Algorithms read `superstep` and `total_vertices` from deserialized `ComputeInput`.

### 2.3 WASM Export

| SDK | Produces .wasm | ABI-compliant |
|-----|----------------|---------------|
| Rust | `export_wasm_compute!` | ✅ |
| Go | tinygo / GOOS=js GOARCH=wasm | ✅ |
| AssemblyScript | `asc` | ✅ |
| TypeScript | — | Use AS or Rust for WASM |

### 2.4 Local Testing (No Cluster)

| SDK | runCompute / equivalent |
|-----|-------------------------|
| Rust | `vertex_program_compute` |
| Go | Manual: NewContext, compute, collect outgoing |
| TypeScript | `runCompute(program, input)` |
| AssemblyScript | — (WASM-only) |

---

## 3. Usage Summary

To write a vertex program with **full expressivity** (superstep, total_vertices, vote-to-halt):

1. **Rust:** Implement `VertexProgram`, use `Context::superstep()`, `Context::total_vertices()`, `ctx.aggregate()` (stub). Export via `export_wasm_compute!` for cluster.
2. **Go:** Implement `VertexProgram`, use `Context.Superstep`, `Context.TotalVertices`, `Context.Aggregate()` (stub). Build with tinygo for WASM.
3. **TypeScript:** Implement `VertexProgram`, use `ctx.superstep`, `ctx.total_vertices`, `ctx.aggregate()` (stub). Use `runCompute` for local tests; use AS/Rust for production WASM.
4. **AssemblyScript:** Parse `ComputeInput` (including `superstep`, `total_vertices`), implement logic, serialize `ComputeResultWire`. Build with `asc` for WASM.

---

## 4. ABI Reference

See `docs/ABI_SPEC.md` for the full wire format, error codes, and validation rules.
