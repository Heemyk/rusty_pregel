//! The core trait: [`VertexProgram`].
//!
//! This is the main abstraction for Pregel algorithms. Implement this trait
//! to define what happens at each vertex during each superstep.

use crate::{Context, Vertex};
use pregel_common::VertexId;

/// The main trait for implementing a Pregel algorithm.
///
/// You implement this trait for a struct (often unit struct like `struct PageRank;`).
/// The runtime loads your program as a WASM module (or native for Rust), and calls
/// `compute` for each vertex that has messages or is active.
///
/// # Type Parameters (Associated Types)
///
/// * `VertexValue` - The type stored at each vertex. E.g., `f64` for PageRank scores,
///   `u64` for component IDs in Connected Components.
/// * `Message` - The type sent between vertices. Must be serializable and clonable.
///
/// # The compute() Method
///
/// * `vertex` - The current vertex. You can read and *modify* its `value`.
/// * `messages` - `(source_id, payload)` pairs from the previous superstep.
///   Empty on superstep 0 (no messages yet). Use `source_id` for reverse edges (e.g. CC).
/// * `ctx` - Use `ctx.send(target, msg)` to send messages. They'll be delivered
///   at the start of the next superstep.
///
/// # Halt Semantics
///
/// In standard Pregel, vertices can "vote to halt" when they've converged. Our
/// model: if you send no messages and don't change state, the vertex may be
/// considered inactive. (Implementation detail in the runtime.)
///
/// # Example
///
/// PageRank: each vertex sends its rank / out_degree to each neighbor.
///
/// ```ignore
/// fn compute(&mut self, vertex: &mut Vertex<f64>, messages: &[(VertexId, f64)], ctx: &mut Context<f64>) {
///     let sum: f64 = messages.iter().map(|(_, m)| *m).sum();
///     vertex.value = 0.15 + 0.85 * sum;
///     let contribution = vertex.value / vertex.edges.len() as f64;
///     for &neighbor in &vertex.edges {
///         ctx.send(neighbor, contribution);
///     }
/// }
/// ```
pub trait VertexProgram: Send + Sync {
    /// The type of value stored at each vertex.
    type VertexValue: Send + Sync;

    /// The type of message sent between vertices.
    type Message: Send + Sync + Clone;

    /// Called once per vertex per superstep. Update `vertex.value` and use
    /// `ctx.send()` to send messages to other vertices.
    ///
    /// `messages` are `(source_vertex_id, payload)` from the previous superstep.
    /// Use `source` to build reverse edges (e.g. CC neighbors = edges ∪ senders).
    fn compute(
        &mut self,
        vertex: &mut Vertex<Self::VertexValue>,
        messages: &[(VertexId, Self::Message)],
        ctx: &mut Context<Self::Message>,
    );
}
