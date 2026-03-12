//! Re-export of the partition function from pregel-core.

use pregel_common::{VertexId, WorkerId};
use pregel_core::partition as core_partition;

/// Delegates to `pregel_core::partition`. See that crate for documentation.
pub fn partition(vertex_id: VertexId, worker_count: usize) -> WorkerId {
    core_partition(vertex_id, worker_count)
}
