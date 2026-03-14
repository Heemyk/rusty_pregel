# Session-Based Architecture

## Overview

Sessions separate **graph lifecycle** from **job execution**:

- **Session** = coordinator + workers running, graph loaded and partitioned. Workers stay alive, metrics server runs for session duration.
- **Job** = one algorithm run (CC, PageRank, etc.) on the graph. Jobs are submitted to an existing session.

## CLI Commands

```bash
# From the pregel directory
cd pregel

# Start a session: load graph, launch workers, wait for jobs
cargo run -p pregel-cli -- session --graph examples/data/sample.graph --workers 2 --metrics-port 9090
# Prints: Session ready. Submit jobs with: pregel job submit --session http://127.0.0.1:5100 --algo cc

# In another terminal: submit jobs to the session
cargo run -p pregel-cli -- job submit --session http://127.0.0.1:5100 --algo cc
cargo run -p pregel-cli -- job submit --session http://127.0.0.1:5100 --algo pagerank

# Stop session: Ctrl+C on session start terminal
```

**Note:** Run commands from `pregel/` (where `Cargo.toml` lives).

## Protocol

### Coordinator

- Runs gRPC (barrier, worker registration) + HTTP (job submission)
- State: `Idle` (waiting for job) | `Running` (BSP in progress)
- On `POST /jobs` with `{algo: "cc"}`: transitions to Running, superstep 0, notifies workers
- When all workers halt: transitions to Idle, notifies workers to reset and wait for next job

### Workers

- Load graph at session start (edges only; values reset per job)
- Outer loop: `WaitForJobStart` → run BSP until halt → reset vertex values for algo → `WaitForJobStart`
- Reset: re-initialize vertex values per algorithm (CC: vid, PageRank: 1/N, SP: 0 or ∞)

### Metrics & Logs

- Metrics server: runs for session duration, stays up between jobs
- Add `job_id` label to metrics (optional; can use run counter)
- Log prefix: `[job 1]` or `[job cc-abc]` so logs are clearly scoped per job

## Backward Compatibility

- `pregel submit --graph X --algo cc` remains as "single-shot" mode: start session, run one job, exit. Same behavior as before for quick runs.
