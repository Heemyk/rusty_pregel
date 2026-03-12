//! Protocol: what gets sent over the network.
//!
//! We distinguish between data messages (vertex-to-vertex) and control messages
//! (barrier synchronization).

use pregel_common::{Message, WorkerId};

/// Top-level protocol message.
///
/// When workers communicate, they send one of these. The receiver matches
/// on the variant to handle it appropriately.
#[derive(Debug, Clone)]
pub enum MessagePayload {
    /// A batch of vertex messages for a specific worker.
    /// Sent from Worker A to Worker B when A has messages for vertices owned by B.
    VertexMessages {
        target_worker: WorkerId,
        messages: Vec<Message>,
    },

    /// A worker signaling it has finished the current superstep.
    /// Sent from each worker to the coordinator for barrier synchronization.
    BarrierAck {
        worker_id: WorkerId,
        superstep: u64,
    },
}
