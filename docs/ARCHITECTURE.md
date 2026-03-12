# Pregel Architecture Guide

This document explains how the crates fit together and how data flows through the system. It's written for developers new to Rust and/or distributed systems.

## 1. The Big Picture

Pregel runs graph algorithms in parallel. The graph is split across workers. Each worker:
1. Receives messages for its vertices
2. Runs `compute()` for each vertex (via WASM or native code)
3. Sends outgoing messages to other workers
4. Waits at a barrier until all workers are done
5. Repeats

The **coordinator** runs the barrier: it waits for all workers to say "I'm done," then tells everyone to advance to the next superstep.

## 2. Data Flow

### Message Flow

```
Vertex A (on Worker 0) calls ctx.send(vertex_B_id, message)
    → Worker 0's outbox gets (vertex_B_id, message)
    → MessageRouter: partition(vertex_B_id) = Worker 2
    → MessageBatch for Worker 2 is built
    → Network send to Worker 2
    → Worker 2's inbox: add(vertex_B_id, message)
    → Next superstep: Worker 2 runs compute for vertex B, gets the message
```

### Key Types

| Type | Where | Purpose |
|------|-------|---------|
| `Message` | pregel-common | `{ target: VertexId, payload: Vec<u8> }` |
| `MessageBatch` | pregel-messaging | Messages grouped by target worker |
| `MessageInbox` | pregel-worker | Per-vertex message storage |
| `MessageOutbox` | pregel-worker | Outgoing messages by target worker |

## 3. Crate Dependencies

```
pregel-cli
    └── pregel-coordinator, pregel-worker

pregel-coordinator
    └── pregel-core, pregel-common

pregel-worker
    └── pregel-core, pregel-storage, pregel-messaging, pregel-wasm, pregel-checkpoint, pregel-common

pregel-core, pregel-storage, pregel-messaging, pregel-wasm, pregel-checkpoint
    └── pregel-common
```

Everything ultimately depends on **pregel-common**. The worker has the most dependencies because it does the most.

## 4. Key Rust Concepts Used

### Traits
A trait is like an interface. `VertexProgram` is a trait: you implement `compute()` for your struct. The runtime doesn't care about your concrete type; it just calls `compute()`.

### Generics
`Vertex<V>` and `Context<M>` are generic. `V` and `M` are type parameters. When you use `Vertex<f64>`, the compiler generates a version where `value` is `f64`. No runtime overhead.

### Option and Result
- `Option<T>`: either `Some(value)` or `None`. Rust's way of representing "maybe there's a value."
- `Result<T, E>`: either `Ok(value)` or `Err(error)`. For fallible operations. Use `?` to propagate errors.

### Ownership and Borrowing
Rust has no garbage collector. Values have a single owner. You can *borrow* with `&` (read-only) or `&mut` (read-write). The compiler ensures no use-after-free, no data races.

### Async (Tokio)
The worker could use `async` for network I/O. Tokio is Rust's async runtime. We've scaffolded with `tokio` in dependencies; the full async loop would use `tokio::spawn` for concurrent tasks.

## 5. Where to Implement Next

1. **Worker main loop** – A real `main()` that: load partition, loop { receive, compute, send, barrier }, checkpoint periodically.
2. **Network transport** – gRPC or QUIC for worker↔worker and worker↔coordinator.
3. **WASM ABI** – Define how vertex/messages/context are passed to the WASM `compute` export.
4. **Graph loading** – Load from file (e.g., edge list) or S3, partition, and distribute to workers.
5. **CLI submit** – Actually start the coordinator and workers when you run `pregel submit`.
