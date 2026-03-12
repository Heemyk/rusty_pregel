# PageRank Example

PageRank runs natively in the worker when no WASM module is provided. The algorithm:
- Initial value: 1/N for each vertex
- Each superstep: new_rank = 0.15 + 0.85 * sum(incoming_messages)
- Sends: rank/out_degree to each neighbor

## Run

```bash
# From workspace root - build everything first
cargo build --workspace

# Submit (uses native PageRank, no WASM needed)
cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --workers 2
```

Press Ctrl+C to stop. The workers will run until halted.
