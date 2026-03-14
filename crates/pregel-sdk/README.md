# pregel-sdk

**Purpose:** The developer-facing API for writing Pregel algorithms. When you implement a graph algorithm (PageRank, Connected Components, etc.), you use the types and traits from this crate.

**Who uses it:** Algorithm authors. The examples (pagerank, connected_components, shortest_path) depend on this crate. In the future, Python/Go/TypeScript SDKs will compile down to the same concepts.

**→ WASM:** Implement `VertexProgram`, then use `pregel_sdk::export_wasm_compute!(YourAlgo)` to emit the WASM ABI. Build with `--target wasm32-unknown-unknown`. See `docs/SDK_ARCHITECTURE.md` for the full flow.

## Core Idea: The VertexProgram Trait

In Pregel, you don't write a main loop. You implement the `VertexProgram` trait, which has one method: `compute`. The runtime calls `compute` for each vertex, in each superstep, passing in:

1. **The vertex** – its ID, value, and outgoing edges
2. **Incoming messages** – what other vertices sent to this vertex last superstep
3. **A context** – lets you send messages and access superstep number

Your job: update the vertex's value based on messages, and optionally send messages to neighbors.

## What's Inside

| Module | Purpose |
|--------|---------|
| `program` | The `VertexProgram` trait – the main abstraction you implement |
| `vertex` | `Vertex<V>` – a graph vertex with id, value, and edges |
| `context` | `Context<M>` – passed to compute(); use it to send messages |
| `message` | `Message` trait – marker for types that can be sent between vertices |
| `aggregator` | `Aggregator` trait – for global reductions (e.g., sum, max across all vertices) |

## Rust Concepts Used

### Generic Types
`Vertex<V>` and `Context<M>` use *generics* (the `<V>` and `<M>`). That means:
- `Vertex<f64>` – vertex value is a float (e.g., PageRank score)
- `Vertex<u64>` – vertex value is an ID (e.g., connected component root)
- `Context<f64>` – sending float messages

### Associated Types
`VertexProgram` has `type VertexValue` and `type Message`. These are *associated types*: when you implement the trait, you declare what types your algorithm uses. The compiler then knows the concrete types everywhere the trait is used.

### Send + Sync
The bounds `Send + Sync` mean the type can be safely shared across threads. Pregel workers run vertices in parallel, so we need this.
