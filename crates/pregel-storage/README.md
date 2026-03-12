# pregel-storage

**Purpose:** Graph storage and partitioning. Each worker owns a *partition* of the graph – a subset of vertices. This crate defines how that partition is stored and how we decide which worker owns which vertex.

**Who uses it:** `pregel-worker` uses `GraphPartition` to store its vertices. `pregel-checkpoint` uses it for serializing/restoring state.

## Key Concepts

### Graph Partition
The full graph is split across workers. Each worker's `GraphPartition` is a `HashMap<VertexId, VertexData>` – the vertices this worker owns. When a message arrives for vertex V, we look up which worker has V (via `partition(V, worker_count)`) and route the message there.

### VertexData
Vertices are stored with ID, value (as raw bytes), and edges. Values are `Vec<u8>` because different algorithms use different types; serialization happens at the SDK/WASM boundary.

### Partitioning
Re-exports `pregel_core::partition` for convenience. Same hash-based partitioning: `vertex_id % worker_count`.

## What's Inside

| Module | Purpose |
|--------|---------|
| `graph` | `GraphPartition` – a worker's local vertex store |
| `vertex_store` | `VertexData` – id, value, edges for one vertex |
| `partitioner` | Re-export of partition function |
