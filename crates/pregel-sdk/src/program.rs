//! The core trait: [`VertexProgram`].
//!
//! This is the main abstraction for Pregel algorithms. Implement this trait
//! to define what happens at each vertex during each superstep.

use crate::{Context, Vertex};

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
/// * `messages` - Messages sent TO this vertex in the previous superstep.
///   Empty on superstep 0 (no messages yet).
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
/// fn compute(&mut self, vertex: &mut Vertex<f64>, messages: &[f64], ctx: &mut Context<f64>) {
///     let sum: f64 = messages.iter().sum();
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
    fn compute(
        &mut self,
        vertex: &mut Vertex<Self::VertexValue>,
        messages: &[Self::Message],
        ctx: &mut Context<Self::Message>,
    );
}
