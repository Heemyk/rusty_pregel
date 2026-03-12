# pregel-checkpoint

**Purpose:** Fault tolerance. Periodically save worker state to disk. If a worker crashes, we can start a new one and restore from the last checkpoint.

**Who uses it:** `pregel-worker` calls `CheckpointManager::save()` every N supersteps. On failure, the coordinator uses `Recovery::restore_partition()` to rebuild a worker's state.

## How Checkpointing Works

1. **Save:** Every N supersteps (e.g., 10), each worker serializes its partition (vertex values, edges) and the current superstep number to a file.
2. **Storage:** Files go to a configurable path (local disk, or a mounted volume that backs to S3/MinIO).
3. **Recovery:** When a worker dies, the coordinator launches a replacement. The new worker loads the checkpoint file and resumes from that superstep.
4. **Re-send:** Messages in flight when the crash happened may be lost. The BSP model typically requires "rollback" semantics: we may need to re-execute from the last checkpoint and re-send messages. (Full implementation detail.)

## What's Inside

| Module | Purpose |
|--------|---------|
| `checkpoint_manager` | Save and load checkpoint files for a worker |
| `snapshot` | `Checkpoint` and `VertexState` – the serialized format |
| `recovery` | Convert a loaded checkpoint back into a `GraphPartition` |
