# Pregel

A distributed graph processing framework in Rust, inspired by [Google's Pregel](https://research.google/pubs/pub37272/). Implements the Bulk Synchronous Parallel (BSP) model with support for multi-language vertex programs via WebAssembly.

## Overview

Pregel processes large graphs by partitioning them across workers. Each vertex runs a `compute` function in lockstep (supersteps). Vertices send messages to neighbors; messages are delivered at the start of the next superstep. This is the BSP model.

## Project Structure

```
pregel/
├── crates/
│   ├── pregel-common/      # Shared types, errors, config
│   ├── pregel-sdk/         # VertexProgram trait, Vertex, Context (for algorithm authors)
│   ├── pregel-core/        # Superstep, partition, runtime config
│   ├── pregel-storage/     # GraphPartition, VertexData
│   ├── pregel-messaging/   # MessageBatch, protocol
│   ├── pregel-wasm/        # WASM execution (wasmtime)
│   ├── pregel-checkpoint/  # Fault tolerance
│   ├── pregel-worker/      # Worker runtime (the heart of execution)
│   ├── pregel-coordinator/ # Control plane, barrier sync
│   └── pregel-cli/         # Command-line tool
├── examples/               # PageRank, Connected Components, Shortest Path
├── sdk/                    # Python, Go, TypeScript (future)
├── k8s/                    # Kubernetes operator (future)
└── benchmarks/             # Performance tests (future)
```

**Each crate has its own README** with detailed documentation. Start with `crates/pregel-common/README.md` and work your way up.

## Quick Start

```bash
# Build everything
cargo build --workspace

# Run the CLI
cargo run -p pregel-cli -- --help
cargo run -p pregel-cli -- submit --program pagerank.wasm --graph /path/to/graph --workers 4
```

## Documentation Guide (for Rust Beginners)

1. **pregel-common** – Types, errors, config. Understand `Result`, `Message`, `VertexId`.
2. **pregel-sdk** – The `VertexProgram` trait. This is what you implement to write algorithms.
3. **pregel-core** – Superstep, partitioning. How BSP works.
4. **pregel-storage** – How vertices are stored per worker.
5. **pregel-messaging** – How messages are batched and sent.
6. **pregel-wasm** – How vertex compute runs in WASM.
7. **pregel-checkpoint** – Fault tolerance.
8. **pregel-worker** – The worker loop: receive → compute → send → barrier.
9. **pregel-coordinator** – Barrier synchronization, worker registry.
10. **pregel-cli** – User-facing commands.

## Architecture Diagram

```
                    ┌─────────────────┐
                    │   Coordinator   │
                    │  (barrier sync) │
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
   ┌─────────┐         ┌─────────┐         ┌─────────┐
   │ Worker 0│         │ Worker 1│         │ Worker 2│  ...
   │partition│         │partition│         │partition│
   │  + WASM │         │  + WASM │         │  + WASM │
   └────┬────┘         └────┬────┘         └────┬────┘
        │                   │                   │
        └───────────────────┼───────────────────┘
                            │
                    Message passing
                    (peer-to-peer)
```

## License

MIT or Apache-2.0 (your choice)
