# pregel-core

**Purpose:** Core execution abstractions used by both the coordinator and workers. It defines the superstep model, partitioning logic, and runtime configuration.

**Who uses it:** `pregel-worker`, `pregel-coordinator`, and `pregel-storage` all depend on this crate. It's the "glue" that defines how the BSP execution model works.

## Key Concepts

### Superstep
Pregel uses Bulk Synchronous Parallel (BSP). Time is divided into supersteps. In each superstep:
1. All vertices receive messages from the previous superstep
2. All vertices run `compute()` in parallel
3. All vertices send messages (delivered next superstep)
4. **Barrier** – everyone waits before moving to the next superstep

`Superstep` is a simple counter: 0, 1, 2, ...

### Partitioning
The graph is partitioned across workers. Each vertex belongs to exactly one worker. We use hash partitioning: `vertex_id % worker_count`. Simple, deterministic, and works well for many graphs. Future: could add range partitioning or custom partitioners.

### AggregatorValues
When using aggregators (e.g., global sum), the coordinator collects values, reduces them, and broadcasts. `AggregatorValues` is a map of named aggregates (e.g., "total_messages" → bytes) that gets sent to workers.

## What's Inside

| Module | Purpose |
|--------|---------|
| `superstep` | The superstep counter – which round we're in |
| `partition` | `partition(vertex_id, workers)` → which worker owns a vertex |
| `scheduler` | Trait for ordering vertex execution (e.g., round-robin) |
| `aggregator` | `AggregatorValues` – coordinator's aggregate result storage |
| `runtime` | `RuntimeConfig` – worker count, checkpoint interval |
