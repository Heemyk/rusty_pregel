//! Vertex scheduling: determines the order in which vertices are executed.
//!
//! Within a superstep, a worker may process vertices in different orders.
//! A scheduler implements a policy (e.g., round-robin, by in-degree, random).

/// Abstraction for selecting which vertex to execute next.
///
/// The worker repeatedly calls `next_vertex()` until it returns `None`
/// (no more vertices to process this superstep).
///
/// # Implementations (future)
///
/// * **RoundRobin** – iterate through partition in ID order
/// * **MessageCount** – prioritize vertices with more messages (often faster convergence)
/// * **Random** – for load balancing across threads
pub trait VertexScheduler {
    /// Returns the next vertex ID to process, or None if done.
    fn next_vertex(&mut self) -> Option<pregel_common::VertexId>;
}
