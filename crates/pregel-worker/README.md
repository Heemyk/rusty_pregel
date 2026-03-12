# pregel-worker

**Purpose:** The worker runtime – the heart of distributed execution. Each worker owns a partition of the graph, runs vertex compute, and exchanges messages with other workers.

**Who uses it:** The coordinator launches N worker processes. Each runs the worker loop (receive → compute → send → barrier).

## The Worker Loop (BSP)

```
loop {
    1. Receive messages from other workers (into inbox)
    2. For each vertex with messages (or active): run compute()
    3. Route outgoing messages to target workers (via partition function)
    4. Send message batches over the network
    5. Signal coordinator: "I'm done" (barrier)
    6. Wait for all workers to reach barrier
    7. Advance to next superstep
}
```

## Key Subsystems

### Execution
* `execute_superstep` – iterate vertices, run compute, collect outgoing
* `VertexExecutor` – invokes WASM (or native) compute
* `vertex_loop` – the inner loop over vertices

### Messaging
* `MessageInbox` – messages received this superstep, keyed by vertex ID
* `MessageOutbox` – outgoing messages, grouped by target worker
* `MessageRouter` – takes (vertex, payload) pairs, produces MessageBatches

### Partition
* `PartitionManager` – wraps the worker's GraphPartition

## What's Inside

| Module | Purpose |
|--------|---------|
| `worker` | Main Worker struct, `route_messages()` |
| `execution/` | Superstep execution, vertex loop, WASM executor |
| `messaging/` | Inbox, outbox, router |
| `partition/` | Partition manager |
