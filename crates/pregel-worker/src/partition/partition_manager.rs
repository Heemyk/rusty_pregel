//! Wrapper for the worker's graph partition.

use pregel_storage::GraphPartition;

/// Manages the worker's partition: the vertices it owns.
pub struct PartitionManager {
    pub partition: GraphPartition,
}

impl PartitionManager {
    pub fn new(partition: GraphPartition) -> Self {
        Self { partition }
    }

    pub fn vertex_count(&self) -> usize {
        self.partition.vertex_count()
    }
}
