//! Inbox: messages received for vertices this worker owns.

use pregel_common::{Message, VertexId};
use std::collections::HashMap;

/// Messages received this superstep, keyed by target vertex ID.
///
/// When other workers send messages to our vertices, they arrive (via network)
/// and we add them here with `add(target_vertex, msg)`. During compute, we
/// call `get(vertex_id)` to retrieve messages for each vertex.
#[derive(Debug, Default)]
pub struct MessageInbox {
    messages: HashMap<VertexId, Vec<Message>>,
}

impl MessageInbox {
    pub fn new() -> Self {
        Self {
            messages: HashMap::new(),
        }
    }

    /// Add a message for a vertex. Called when we receive a MessageBatch.
    pub fn add(&mut self, target: VertexId, msg: Message) {
        self.messages.entry(target).or_default().push(msg);
    }

    /// Get all messages for a vertex. Returns empty vec if none.
    pub fn get(&self, vertex: VertexId) -> Vec<Message> {
        self.messages.get(&vertex).cloned().unwrap_or_default()
    }

    /// Clear after a superstep (before next round of receives).
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}
