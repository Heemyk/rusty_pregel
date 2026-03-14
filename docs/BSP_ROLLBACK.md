# BSP Rollback Semantics

When a worker crashes, we need to recover to a consistent state. Standard approach:

## Flow

1. **Checkpoint**: Every N supersteps, each worker saves its partition (vertex values, edges) and superstep number.
2. **Crash**: Worker dies (e.g., OOM, network partition).
3. **Detection**: Coordinator notices worker hasn't reported for the current superstep (timeout).
4. **Recovery**:
   - Coordinator signals remaining workers to pause.
   - Spawn replacement worker.
   - Load last checkpoint into replacement (from shared storage: S3, NFS).
   - All workers rewind to last checkpoint superstep.
   - Re-execute from that superstep (messages are deterministic).

## Implementation Status

- [x] CheckpointManager saves/loads partition state
- [x] Recovery restores GraphPartition from checkpoint
- [x] Coordinator failure detection: tracks `worker_last_seen`, background task detects report timeout; on timeout aborts job (advance to u64::MAX)
- [x] Worker: gRPC calls use 60s timeout; coordinator unreachable → worker exits with error
- [ ] Pause/resume signaling to workers
- [ ] Replacement worker spawn (K8s does this; local needs manual)
- [ ] Rewind coordination (broadcast "rollback to superstep K")
- [ ] Re-delivery of messages from checkpoint (workers resend what they would have sent)

## Determinism

For rollback to work, vertex compute must be deterministic. Same inputs → same outputs. Avoid:
- Random number generation
- Wall-clock time
- Non-deterministic float ops (use fixed-point or documented float semantics)
