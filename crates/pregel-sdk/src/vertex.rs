//! The [`Vertex`] struct: a graph vertex with id, value, and edges.

use pregel_common::VertexId;

/// A single vertex in the graph.
///
/// Generic over `V`, the type of the vertex's value. Different algorithms
/// use different value types:
///
/// * **PageRank:** `V = f64` (the rank score)
/// * **Connected Components:** `V = u64` (the smallest vertex ID in the component)
/// * **Shortest Path:** `V = Option<u64>` (distance from source, or None if unreachable)
///
/// # Fields
///
/// * `id` - Unique identifier. Used as the message target when other vertices send to this one.
/// * `value` - Algorithm-specific state. You read and write this in `compute()`.
/// * `edges` - Outgoing edges: vertex IDs this vertex can send messages to.
///   For an undirected graph, both (a,b) and (b,a) would be in the edge lists.
///
/// # Rust Note: Generics
///
/// `Vertex<V>` is generic. When you write `Vertex<f64>`, the compiler generates
/// a version of Vertex where `value` is `f64`. No runtime cost – it's like C++ templates.
#[derive(Debug, Clone)]
pub struct Vertex<V> {
    pub id: VertexId,
    pub value: V,
    pub edges: Vec<VertexId>,
}

impl<V> Vertex<V> {
    /// Create a new vertex.
    pub fn new(id: VertexId, value: V, edges: Vec<VertexId>) -> Self {
        Self { id, value, edges }
    }
}
