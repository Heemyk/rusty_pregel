//! Configuration structs for workers and jobs.
//!
//! These are typically loaded from a config file or constructed from CLI arguments,
//! then passed to workers and the coordinator when starting a job.

use serde::{Deserialize, Serialize};

use crate::types::WorkerId;

/// Configuration for a single worker process.
///
/// When you start a worker, it needs to know: who am I, where is the coordinator,
/// and how many workers are in the cluster? This struct holds that information.
///
/// # Fields
///
/// * `id` - This worker's unique ID (0, 1, 2, ...). Used for partitioning.
/// * `coordinator_addr` - Address to connect to for barrier sync, job info, etc.
///   (e.g., `"127.0.0.1:5000"` or `"coordinator.default.svc.cluster.local:5000"`)
/// * `worker_count` - Total number of workers. Needed for the partition function:
///   `vertex_id % worker_count` determines which worker owns a vertex.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub id: WorkerId,
    pub coordinator_addr: String,
    pub worker_count: usize,
}

/// Configuration for a Pregel job.
///
/// When you submit a job with `pregel submit`, the CLI collects these parameters
/// and the coordinator uses them to launch workers and load the right program/graph.
///
/// # Fields
///
/// * `workers` - How many worker processes to spawn (parallelism level)
/// * `program_path` - Path to the WASM module (e.g., `pagerank.wasm`)
/// * `graph_path` - Where the graph data lives (e.g., `s3://bucket/graph` or local path)
/// * `partition` - Optional partition strategy. Default: hash. Use CustomFile for user-defined mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    pub workers: usize,
    pub program_path: String,
    pub graph_path: String,
    #[serde(default)]
    pub partition: Option<PartitionConfig>,
}

/// Partition strategy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PartitionConfig {
    Hash,
    CustomFile { path: String },
}
