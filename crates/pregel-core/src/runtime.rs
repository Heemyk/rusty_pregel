//! Runtime configuration for a Pregel job.

/// Configuration for the execution runtime.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Number of worker processes in the cluster.
    pub worker_count: usize,

    /// Checkpoint every N supersteps. None = no checkpointing (faster, but no recovery).
    pub checkpoint_interval: Option<u64>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            worker_count: 4,
            checkpoint_interval: Some(10),
        }
    }
}
