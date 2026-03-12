# pregel-coordinator

**Purpose:** The control plane. Coordinates the cluster: tracks workers, manages the superstep barrier, and orchestrates job lifecycle.

**Who uses it:** A single coordinator process runs per cluster. Workers connect to it to register and participate in barrier sync.

## Key Responsibilities

1. **Worker registry** – Know which workers exist, their addresses, how many vertices each has.
2. **Barrier synchronization** – Each worker signals "I'm done" when it finishes a superstep. When all have reported, the coordinator broadcasts "advance to next superstep."
3. **Superstep advancement** – Tracks current superstep, tells workers when to proceed.
4. **Job management** – Tracks active jobs, which workers belong to which job.

## The Barrier

BSP requires a global barrier: no worker proceeds to superstep N+1 until all have finished superstep N. The coordinator implements this:

1. Workers send `BarrierAck { worker_id, superstep }` when done.
2. Coordinator's `Barrier` tracks who has reported.
3. When `Barrier::all_reported()`, coordinator broadcasts "advance."
4. Workers receive, increment local superstep, continue.

## What's Inside

| Module | Purpose |
|--------|---------|
| `coordinator` | Main Coordinator struct |
| `worker_registry` | WorkerInfo, WorkerRegistry |
| `barrier` | Barrier synchronization |
| `job_manager` | JobInfo, JobManager |
