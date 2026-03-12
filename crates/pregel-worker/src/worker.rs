//! The Worker: owns a partition and routes messages.

use pregel_common::{VertexId, WorkerId};
use pregel_core::partition;
use pregel_messaging::MessageBatch;
use pregel_storage::GraphPartition;
use std::collections::HashMap;

/// A Pregel worker process.
///
/// Each worker owns a subset of the graph (its partition) and executes
/// vertex compute for that subset. Workers exchange messages via the
/// partition function: messages for vertex V go to worker `partition(V, N)`.
pub struct Worker {
    pub id: WorkerId,
    pub partition: GraphPartition,
    pub worker_count: usize,
}

impl Worker {
    pub fn new(id: WorkerId, partition: GraphPartition, worker_count: usize) -> Self {
        Self {
            id,
            partition,
            worker_count,
        }
    }

    /// Group outgoing (source, target, payload) triples into MessageBatches by target worker.
    ///
    /// After compute runs, we have many (source_vertex, target_vertex, payload) triples.
    /// This groups them by target worker. The worker then sends each batch to the appropriate peer.
    pub fn route_messages(
        &self,
        outgoing: Vec<(VertexId, VertexId, Vec<u8>)>,
    ) -> HashMap<WorkerId, MessageBatch> {
        let mut batches: HashMap<WorkerId, MessageBatch> = HashMap::new();

        for (source_vertex, target_vertex, payload) in outgoing {
            let target_worker = partition(target_vertex, self.worker_count);
            let batch = batches
                .entry(target_worker)
                .or_insert_with(|| MessageBatch::new(target_worker));
            batch.push(pregel_common::Message {
                source: source_vertex,
                target: target_vertex,
                payload,
            });
        }

        batches
    }
}
