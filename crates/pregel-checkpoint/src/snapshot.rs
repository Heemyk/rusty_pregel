//! The checkpoint format: what we serialize to disk.

use pregel_storage::VertexData;
use serde::{Deserialize, Serialize};

/// A checkpoint: snapshot of a worker's state at a given superstep.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub superstep: u64,
    pub vertices: Vec<VertexState>,
}

/// Serialized form of a single vertex for checkpointing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexState {
    pub id: u64,
    pub value: Vec<u8>,
    pub edges: Vec<u64>,
}

impl From<VertexData> for VertexState {
    fn from(v: VertexData) -> Self {
        Self {
            id: v.id,
            value: v.value,
            edges: v.edges,
        }
    }
}
