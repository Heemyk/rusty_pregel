//! The [`Context`] passed to `compute()`: how vertices send messages.

use pregel_common::VertexId;

/// Context provided to each vertex's `compute()` call.
///
/// This is your interface to the rest of the system. You use it to:
///
/// 1. **Send messages** – `ctx.send(target_vertex_id, message)` 
///    Messages are buffered and delivered at the start of the next superstep.
///
/// 2. **Query superstep** – `ctx.superstep()` tells you which superstep you're in.
///    Superstep 0: no messages yet. Superstep 1: first round of messages.
///
/// # Generic
///
/// `Context<M>` is generic over the message type `M`, matching your
/// `VertexProgram::Message` type.
///
/// # Internal Structure
///
/// The runtime builds `Context` before calling your compute. After compute
/// returns, it drains `outgoing` and routes those messages to the correct
/// workers (based on partition(target_id)).
#[derive(Debug, Default)]
pub struct Context<M> {
    /// Current superstep number (0-indexed).
    pub superstep: u64,

    /// Messages to send. Pairs of (target_vertex_id, message).
    /// The runtime reads this after compute() returns.
    pub outgoing: Vec<(VertexId, M)>,
}

impl<M> Context<M> {
    /// Create a new context for the given superstep.
    pub fn new(superstep: u64) -> Self {
        Self {
            superstep,
            outgoing: Vec::new(),
        }
    }

    /// Send a message to another vertex.
    ///
    /// The message will be delivered at the start of the next superstep.
    /// The target can be on any worker; the runtime handles routing.
    pub fn send(&mut self, target: VertexId, msg: M) {
        self.outgoing.push((target, msg));
    }

    /// Returns the current superstep number.
    pub fn superstep(&self) -> u64 {
        self.superstep
    }
}
