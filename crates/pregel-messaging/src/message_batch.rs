//! Message batching: grouping messages by target worker.
//!
//! When a worker runs compute(), it produces many (vertex_id, message) pairs.
//! We group them by which worker owns each target vertex, then send one batch
//! per target worker. This reduces network round-trips.

use pregel_common::{Message, WorkerId};

/// A batch of messages all destined for the same worker.
///
/// After executing vertices, the worker has many outgoing messages. Rather than
/// sending each one individually, we group by target worker. Worker 2 gets one
/// `MessageBatch` containing all messages for vertices it owns.
#[derive(Debug, Clone)]
pub struct MessageBatch {
    pub target_worker: WorkerId,
    pub messages: Vec<Message>,
}

impl MessageBatch {
    pub fn new(target_worker: WorkerId) -> Self {
        Self {
            target_worker,
            messages: Vec::new(),
        }
    }

    pub fn push(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}
