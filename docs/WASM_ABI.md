# WASM ABI for Vertex Compute

The vertex compute function is invoked by the runtime for each vertex each superstep.

**See also:** `docs/ABI_SPEC.md` for the full formal specification (validation, error codes, security).

## Implementation

### Host (runtime) side

The host implementation lives in **`crates/pregel-wasm/src/engine.rs`**:

- `WasmExecutor::compute(module, input)` writes input to linear memory at offset 0, calls the `compute` export, and reads output from a fixed region.
- Uses wasmtime for execution.
- See the `compute` method for the exact ABI call: `compute(input_ptr, input_len, output_ptr, output_max_len) -> output_len`.

### Guest (user algorithm) side

The guest implementation is **future SDK work**. A Pregel SDK will emit WASM modules that export `compute` with the signature below. Algorithm authors will write Rust (or another language) and compile to WASM; the SDK handles the ABI glue.

---

## Export Name

`compute`

## Signature

```
compute(input_ptr: i32, input_len: i32, output_ptr: i32, output_max_len: i32) -> i32
```

- `input_ptr` / `input_len`: Offset and length in linear memory of the serialized input
- `output_ptr` / `output_max_len`: Where to write output and maximum bytes
- Returns: Number of bytes written to output, or negative on error

## Input Format (bincode)

```
(VertexInput, Vec<Vec<u8>>)
VertexInput = { id: u64, value: Vec<u8>, edges: Vec<u64> }
```

## Output Format (bincode)

```
ComputeResultWire = { new_value: Option<Vec<u8>>, outgoing: Vec<(u64, Vec<u8>)> }
```

- `new_value`: updated vertex value (or None to halt / no change)
- `outgoing`: [(target_vertex_id, message_payload), ...]

## Memory

The module must export `memory`. The host writes input to the given offset before calling compute.
