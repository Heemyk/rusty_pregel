# pregel-messaging

**Purpose:** The networking layer for worker-to-worker and worker-to-coordinator communication. Defines message batches and the protocol (what gets sent over the wire).

**Who uses it:** `pregel-worker` uses this to batch and send messages. The actual transport (gRPC, QUIC, etc.) would be implemented in a separate layer or as part of the worker.

## Key Concepts

### Message Batching
Sending one network packet per message would be very slow. Instead, we batch messages by target worker: all messages going to Worker 3 go in one `MessageBatch`, then we send one packet (or a few) per batch.

### MessagePayload
The protocol distinguishes between:
* **VertexMessages** – a batch of messages for vertices on a specific worker
* **BarrierAck** – a worker signaling it has finished the current superstep (for barrier sync)

## What's Inside

| Module | Purpose |
|--------|---------|
| `message_batch` | `MessageBatch` – messages grouped by target worker |
| `protocol` | `MessagePayload` – enum of message types for the wire protocol |
