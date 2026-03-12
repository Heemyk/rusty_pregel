//! Restoring a GraphPartition from a checkpoint.

use crate::snapshot::Checkpoint;
use pregel_storage::{GraphPartition, VertexData};
use std::collections::HashMap;

/// Utilities for recovering from a checkpoint.
pub struct Recovery;

impl Recovery {
    /// Convert a loaded checkpoint into a GraphPartition.
    ///
    /// Used when starting a new worker to replace a failed one. The new worker
    /// loads the checkpoint and gets a partition it can continue from.
    pub fn restore_partition(checkpoint: Checkpoint) -> GraphPartition {
        let mut vertices = HashMap::new();
        for vs in checkpoint.vertices {
            vertices.insert(
                vs.id,
                VertexData {
                    id: vs.id,
                    value: vs.value,
                    edges: vs.edges,
                },
            );
        }
        GraphPartition { vertices }
    }
}
