//! Graph partitioning: assigning vertices to workers.
//!
//! Each vertex lives on exactly one worker. The partition function determines
//! which worker owns a given vertex. We use hash partitioning: `vertex_id % worker_count`.

use pregel_common::{VertexId, WorkerId};

/// Determine which worker owns a vertex.
///
/// Uses modulo: `vertex_id % worker_count`. This gives:
/// * Deterministic – same vertex always maps to same worker
/// * Balanced – for random-looking IDs, roughly even distribution
/// * Fast – single modulo operation
///
/// # Edge Cases
///
/// * `worker_count == 0` would panic (division by zero). Callers must ensure workers > 0.
/// * Vertex IDs don't need to be contiguous; any u64 is valid.
///
/// # Example
///
/// ```ignore
/// partition(100, 8)  // → worker 4  (100 % 8 == 4)
/// partition(17, 8)   // → worker 1  (17 % 8 == 1)
/// ```
pub fn partition(vertex_id: VertexId, worker_count: usize) -> WorkerId {
    (vertex_id % worker_count as u64) as WorkerId
}

/// Metadata about a partition (used for reporting to coordinator).
#[derive(Debug, Clone)]
pub struct PartitionMetadata {
    pub worker_id: WorkerId,
    pub vertex_count: u64,
}
