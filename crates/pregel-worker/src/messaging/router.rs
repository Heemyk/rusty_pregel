//! Message routing: (vertex, payload) → MessageBatches by target worker.

use pregel_common::{Message, VertexId, WorkerId};
use pregel_core::partition;
use pregel_messaging::MessageBatch;
use std::collections::HashMap;

/// Routes outgoing messages into batches by target worker.
///
/// Takes a flat list of (target_vertex, payload) and groups by
/// `partition(target_vertex, worker_count)`. Returns one MessageBatch
/// per worker that has messages (empty batches are filtered out).
pub struct MessageRouter {
    worker_count: usize,
}

impl MessageRouter {
    pub fn new(worker_count: usize) -> Self {
        Self { worker_count }
    }

    /// Route outgoing pairs into MessageBatches. Skips empty batches.
    pub fn route(&self, outgoing: Vec<(VertexId, Vec<u8>)>) -> Vec<MessageBatch> {
        let mut batches: HashMap<WorkerId, MessageBatch> = HashMap::new();

        for (target_vertex, payload) in outgoing {
            let target_worker = partition(target_vertex, self.worker_count);
            batches
                .entry(target_worker)
                .or_insert_with(|| MessageBatch::new(target_worker))
                .push(Message {
                    target: target_vertex,
                    payload,
                });
        }

        batches.into_values().filter(|b| !b.is_empty()).collect()
    }
}
