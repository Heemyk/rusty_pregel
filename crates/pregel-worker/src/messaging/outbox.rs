//! Outbox: outgoing messages grouped by target worker.

use pregel_common::{Message, WorkerId};
use pregel_messaging::MessageBatch;
use std::collections::HashMap;

/// Outgoing messages buffered by target worker.
///
/// After compute, we have many (vertex, payload) pairs. We push them here
/// with `push(target_worker, msg)`. When ready to send, `take_batches()` drains
/// and returns one MessageBatch per target worker.
#[derive(Debug, Default)]
pub struct MessageOutbox {
    batches: HashMap<WorkerId, MessageBatch>,
}

impl MessageOutbox {
    pub fn new() -> Self {
        Self {
            batches: HashMap::new(),
        }
    }

    pub fn push(&mut self, target_worker: WorkerId, msg: Message) {
        self.batches
            .entry(target_worker)
            .or_insert_with(|| MessageBatch::new(target_worker))
            .push(msg);
    }

    /// Take all batches and clear the outbox.
    pub fn take_batches(&mut self) -> Vec<MessageBatch> {
        self.batches.drain().map(|(_, b)| b).collect()
    }

    pub fn clear(&mut self) {
        self.batches.clear();
    }
}
