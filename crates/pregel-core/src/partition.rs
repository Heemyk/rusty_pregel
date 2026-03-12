//! Graph partitioning: assigning vertices to workers.
//!
//! Supports hash partitioning (default) and custom partition files for
//! user-defined vertex→worker mappings.

use pregel_common::{VertexId, WorkerId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Strategy for partitioning vertices across workers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartitionStrategy {
    /// Default: vertex_id % worker_count
    Hash,

    /// Load vertex→worker mapping from file. Format: one line per vertex,
    /// "vertex_id worker_id". Vertices not in the file fall back to hash.
    CustomFile { path: String },
}

impl Default for PartitionStrategy {
    fn default() -> Self {
        Self::Hash
    }
}

/// Partition function abstraction. Can use hash or custom mapping.
pub trait PartitionStrategyImpl: Send + Sync {
    fn partition(&self, vertex_id: VertexId, worker_count: usize) -> WorkerId;
}

/// Default hash partitioning: vertex_id % worker_count.
#[derive(Debug, Clone, Default)]
pub struct HashPartition;

impl PartitionStrategyImpl for HashPartition {
    fn partition(&self, vertex_id: VertexId, worker_count: usize) -> WorkerId {
        (vertex_id % worker_count as u64) as WorkerId
    }
}

/// Custom partitioning from a precomputed vertex→worker map.
#[derive(Debug, Clone)]
pub struct CustomPartition {
    map: HashMap<VertexId, WorkerId>,
    fallback: HashPartition,
}

impl CustomPartition {
    /// Load from file. Format: "vertex_id worker_id" per line. Comments (#) and empty lines ignored.
    pub fn from_path(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let map = content
            .lines()
            .filter_map(|line| {
                let line = line.split('#').next().unwrap_or(line).trim();
                if line.is_empty() {
                    return None;
                }
                let mut parts = line.split_whitespace();
                let vid: VertexId = parts.next()?.parse().ok()?;
                let wid: WorkerId = parts.next()?.parse().ok()?;
                Some((vid, wid))
            })
            .collect();
        Ok(Self {
            map,
            fallback: HashPartition,
        })
    }

    pub fn from_map(map: HashMap<VertexId, WorkerId>) -> Self {
        Self {
            map,
            fallback: HashPartition,
        }
    }
}

impl PartitionStrategyImpl for CustomPartition {
    fn partition(&self, vertex_id: VertexId, worker_count: usize) -> WorkerId {
        self.map
            .get(&vertex_id)
            .copied()
            .unwrap_or_else(|| self.fallback.partition(vertex_id, worker_count))
    }
}

/// Determine which worker owns a vertex (convenience function using default hash).
pub fn partition(vertex_id: VertexId, worker_count: usize) -> WorkerId {
    HashPartition.partition(vertex_id, worker_count)
}

/// Load partition strategy from config.
pub fn load_strategy(strategy: &PartitionStrategy) -> std::io::Result<Box<dyn PartitionStrategyImpl>> {
    match strategy {
        PartitionStrategy::Hash => Ok(Box::new(HashPartition)),
        PartitionStrategy::CustomFile { path } => {
            let cp = CustomPartition::from_path(path)?;
            Ok(Box::new(cp))
        }
    }
}

impl From<Option<pregel_common::PartitionConfig>> for PartitionStrategy {
    fn from(cfg: Option<pregel_common::PartitionConfig>) -> Self {
        match cfg {
            None => PartitionStrategy::Hash,
            Some(pregel_common::PartitionConfig::Hash) => PartitionStrategy::Hash,
            Some(pregel_common::PartitionConfig::CustomFile { path }) => {
                PartitionStrategy::CustomFile { path }
            }
        }
    }
}

/// Metadata about a partition (used for reporting to coordinator).
#[derive(Debug, Clone)]
pub struct PartitionMetadata {
    pub worker_id: WorkerId,
    pub vertex_count: u64,
}
