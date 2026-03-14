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
/// 3. **Query total vertices** – `ctx.total_vertices()` for graph-wide values (e.g. PageRank 1/N).
/// 4. **Aggregators** (stub) – `ctx.aggregate(name, value)` for global reduction (future).
///
/// # Generic
///
/// `Context<M>` is generic over the message type `M`, matching your
/// `VertexProgram::Message` type.
#[derive(Debug, Default)]
pub struct Context<M> {
    /// Current superstep number (0-indexed).
    pub superstep: u64,
    /// Total vertices in the graph.
    pub total_vertices: u64,
    /// Messages to send. Pairs of (target_vertex_id, message).
    pub outgoing: Vec<(VertexId, M)>,
}

impl<M> Context<M> {
    /// Create a new context for the given superstep and total vertices.
    pub fn new(superstep: u64, total_vertices: u64) -> Self {
        Self {
            superstep,
            total_vertices,
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

    /// Returns the total number of vertices in the graph.
    pub fn total_vertices(&self) -> u64 {
        self.total_vertices
    }

    /// Contribute to a named aggregator (stub; not yet implemented by runtime).
    #[allow(unused_variables)]
    pub fn aggregate(&mut self, name: &str, value: impl std::any::Any) {
        // TODO: runtime will collect and reduce aggregator values
        let _ = (name, value);
    }
}
