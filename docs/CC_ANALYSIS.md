# Connected Components: Algorithm Analysis for sample.graph

## Graph Structure

```
0 1, 0 2, 1 2, 1 3, 2 3, 3 4, 4 0, 4 1, 4 2
```

**Directed out-edges** (loader adds dst to src's list only):
- v0 → [1, 2]
- v1 → [2, 3]
- v2 → [3]
- v3 → [4]
- v4 → [0, 1, 2]

**Partition (vid % 2):**
- Worker 0: v0, v2, v4
- Worker 1: v1, v3

**Expected CC result:** All vertices in one component; min ID = 0. All should converge to 0.

## Intended Algorithm Logic

1. **Superstep 0:** Each vertex sends its ID to outgoing neighbors.
2. **Superstep N > 0:** `new_value = min(current_value, min(messages))`. If changed, send to neighbors. Else vote to halt.

## Expected Trace

### Superstep 0 (matches output ✓)
- v0→v1:0, v2:0 | v2→v3:2 | v4→v0:4, v1:4, v2:4
- v1→v2:1, v3:1 | v3→v4:3

### Superstep 1 – Expected
Inbox: v0:[4], v1:[0,4], v2:[0,1,4], v3:[1,2], v4:[3]

- v0: current=0, min=4, new=0 → **halt** ✓
- v1: current=1, min=0, new=0 → **update, send 0** to v2,v3
- v2: current=2, min=0, new=0 → **update, send 0** to v3
- v3: current=3, min=1, new=1 → **update, send 1** to v4
- v4: current=4, min=3, new=3 → **update, send 3** to v0,v1,v2

Worker 1 should send **3 msgs** (from v1, v3). Worker 0 should send **4 msgs** (from v2, v4).

### Actual Output (Superstep 1)
- Worker 1: **0 msgs sent** – v1 and v3 didn't run
- Worker 1 inbox at step 1: **empty** (no "vertex X ←" lines)

## Root Cause: Message Delivery Race

Worker 1 advances to superstep 1 and drains `batch_rx` with `try_recv()` before cross-worker messages from worker 0 have been delivered. With an empty inbox, v1 and v3 are skipped (superstep > 0 only runs vertices with messages), so worker 1 sends 0 messages.

The coordinator advances as soon as all workers report; it does not wait for message delivery. On localhost this usually works, but a race can cause messages to arrive after the drain.

## Fix

Before computing each superstep, ensure incoming messages have been received. Options:
1. **Short sleep** after `wait_for_advance` to allow delivery (simple, localhost-friendly).
2. **Drain with timeout** – keep `try_recv` in a loop for ~50–100ms to gather late batches.
3. **Delivery barrier** – workers signal "ready" only after receiving expected messages (requires knowing expected count).
