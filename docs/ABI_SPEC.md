# Pregel Vertex Compute ABI Specification

**Version:** 1.0  
**Status:** Stable  
**Serialization:** bincode (Rust-native; other languages use compatible encoding)

---

## 1. Overview

The ABI defines the contract between the Pregel runtime (host) and vertex compute modules (guest). Any module that implements this contract can run in the Pregel cluster regardless of implementation language.

---

## 2. WASM Export Contract

### 2.1 Function Signature

```
compute(input_ptr: i32, input_len: i32, output_ptr: i32, output_max_len: i32) -> i32
```

| Parameter | Type | Meaning |
|-----------|------|---------|
| `input_ptr` | i32 | Offset in linear memory where serialized `ComputeInput` starts |
| `input_len` | i32 | Byte length of input. Must be > 0. |
| `output_ptr` | i32 | Offset where guest must write serialized `ComputeResultWire` |
| `output_max_len` | i32 | Maximum bytes guest may write. Must be > 0. |

**Return value:** Number of bytes written to `output_ptr`, or a **negative error code**.

### 2.2 Error Codes (Guest → Host)

| Code | Name | Meaning |
|------|------|---------|
| -1 | `EINVALID` | Invalid arguments (e.g. `input_len <= 0`, `output_max_len <= 0`) |
| -2 | `EDESERIALIZE` | Failed to deserialize input (malformed or unknown format) |
| -3 | `ESERIALIZE` | Failed to serialize output |
| -4 | `EOUTPUT_OVERRUN` | Output exceeds `output_max_len` |
| -5 | `EALLOC` | Memory allocation failed (guest OOM) |
| -6 | `EUSER` | User algorithm error (e.g. assertion, panic) |

### 2.3 Memory Layout

- **Input region:** `[input_ptr, input_ptr + input_len)` — host owns; guest reads only.
- **Output region:** `[output_ptr, output_ptr + min(return_value, output_max_len))` — guest writes; host reads after call returns.
- **Contract:** Guest MUST NOT read outside input region. Guest MUST NOT write outside `[output_ptr, output_ptr + min(actual_output_len, output_max_len))`.
- **Host layout (reference):** Input at 0, output at 32KB. Both limited to 32KB each. See `pregel-wasm/engine.rs`.

### 2.4 Security

- **Sandbox:** WASM modules run in a sandbox. No system calls, no network, no filesystem unless explicitly provided by host.
- **Bounds:** Host validates `input_len` and `output_max_len` before call. Guest must respect them.
- **Determinism:** For checkpointing/recovery, compute MUST be deterministic. No RNG, wall-clock, or shared mutable state.
- **Timeouts:** Host may enforce execution time limits (implementation-specific).

---

## 3. Wire Format (Bincode)

### 3.1 ComputeInput

```rust
struct ComputeInput {
    vertex_id: u64,           // This vertex's ID
    value: Vec<u8>,          // Current vertex value (serialized)
    edges: Vec<u64>,         // Outgoing edge targets (vertex IDs)
    messages: Vec<(u64, Vec<u8>)>,  // (source_vertex_id, serialized_message)
    superstep: u64,          // Current superstep (0-indexed). Default 0 if omitted.
    total_vertices: u64,     // Total vertices in graph (e.g. PageRank 1/N). Default 0 if omitted.
}
```

**Validation:**
- `vertex_id`: any u64
- `value`: 0–1MB (configurable; default 1MB)
- `edges`: length 0–1M; each element valid VertexId
- `messages`: length 0–1M; each payload 0–64KB

### 3.2 ComputeResultWire

```rust
struct ComputeResultWire {
    new_value: Option<Vec<u8>>,   // Updated vertex value; None = vote to halt / no change
    outgoing: Vec<(u64, Vec<u8>)>, // (target_vertex_id, serialized_message)
}
```

**Validation:**
- `new_value`: if `Some`, 0–1MB
- `outgoing`: length 0–1M; each target valid VertexId; each payload 0–64KB

---

## 4. Validation (Host-Side)

The host (pregel-worker) MAY validate `ComputeInput` before invoking the guest. If validation fails, the host MUST NOT call the guest and MUST treat the job as failed.

The host SHOULD validate `ComputeResultWire` after the call. Invalid output (e.g. malformed, out-of-bounds targets) MAY be rejected.

---

## 5. Schema (CDDL) — Informational

```
ComputeInput = {
  vertex_id: uint64,
  value: bstr,
  edges: [* uint64],
  messages: [* [ source: uint64, payload: bstr ]],
  ? superstep: uint64,       ; default 0
  ? total_vertices: uint64   ; default 0
}

ComputeResultWire = {
  new_value: bstr / null,
  outgoing: [* [ target: uint64, payload: bstr ]]
}
```

---

## 6. Versioning

- **ABI version:** Embedded in module metadata (future) or implied by runtime compatibility.
- **Wire format:** Backward-compatible additions (new optional fields) allowed. Breaking changes require a new major ABI version.
