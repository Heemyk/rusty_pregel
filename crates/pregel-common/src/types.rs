//! Core type definitions used throughout the Pregel system.
//!
//! These types define the "vocabulary" of the framework. Every message sent between
//! workers, every vertex lookup, and every partition calculation uses these types.

use serde::{Deserialize, Serialize};

/// Unique identifier for a vertex in the graph.
///
/// We use `u64` because real-world graphs (e.g., the web, social networks) can have
/// billions of vertices. A 64-bit ID gives us plenty of headroom.
///
/// **Rust note:** `type X = Y` creates a type alias. `VertexId` and `u64` are
/// interchangeable; the alias improves readability and documentation.
pub type VertexId = u64;

/// Identifier for a worker process in the cluster.
///
/// Workers are typically numbered 0, 1, 2, ... up to (worker_count - 1).
/// The partition function uses this to assign vertices to workers:
/// `vertex_id % worker_count` → which worker owns that vertex.
pub type WorkerId = u32;

/// A message sent from one vertex to another during a superstep.
///
/// In Pregel's BSP model, vertices send messages during `compute()`. Those messages
/// are delivered to target vertices at the start of the next superstep. The payload
/// is raw bytes because different algorithms use different message types (floats for
/// PageRank, vertex IDs for connected components, etc.). Serialization is handled
/// by the SDK layer.
///
/// # Fields
///
/// * `target` - The vertex this message is addressed to (can be on any worker)
/// * `payload` - The serialized message content. Interpretation depends on the algorithm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub target: VertexId,
    pub payload: Vec<u8>,
}
