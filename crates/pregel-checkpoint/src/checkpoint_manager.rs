//! Saving and loading checkpoint files.

use crate::snapshot::{Checkpoint, VertexState};
use pregel_common::{Result, WorkerId};
use pregel_storage::GraphPartition;

/// Manages checkpoint storage for a worker.
///
/// Checkpoints are stored as files: `{storage_path}/worker_{id}_step_{superstep}.ckpt`.
/// The format is bincode-serialized `Checkpoint` structs.
pub struct CheckpointManager {
    storage_path: String,
}

impl CheckpointManager {
    pub fn new(storage_path: impl Into<String>) -> Self {
        Self {
            storage_path: storage_path.into(),
        }
    }

    /// Save the worker's partition to disk.
    pub fn save(
        &self,
        worker_id: WorkerId,
        superstep: u64,
        partition: &GraphPartition,
    ) -> Result<()> {
        let vertices: Vec<VertexState> = partition
            .vertices
            .values()
            .cloned()
            .map(VertexState::from)
            .collect();

        let checkpoint = Checkpoint {
            superstep,
            vertices,
        };

        let path = format!("{}/worker_{}_step_{}.ckpt", self.storage_path, worker_id, superstep);
        let bytes = bincode::serialize(&checkpoint)
            .map_err(|e| pregel_common::PregelError::Checkpoint(e.to_string()))?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Load a checkpoint from disk.
    pub fn load(&self, worker_id: WorkerId, superstep: u64) -> Result<Checkpoint> {
        let path = format!("{}/worker_{}_step_{}.ckpt", self.storage_path, worker_id, superstep);
        let bytes = std::fs::read(path)?;
        bincode::deserialize(&bytes).map_err(|e| pregel_common::PregelError::Checkpoint(e.to_string()))
    }
}
