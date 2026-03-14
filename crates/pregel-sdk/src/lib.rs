//! # pregel-sdk
//!
//! The developer-facing API for writing Pregel graph algorithms.
//!
//! Implement the [`VertexProgram`] trait to define your algorithm. The runtime will
//! invoke your `compute` method for each vertex in each superstep. Use [`Context`]
//! to send messages to other vertices.
//!
//! # Example (conceptual)
//!
//! ```ignore
//! struct PageRank;
//!
//! impl VertexProgram for PageRank {
//!     type VertexValue = f64;
//!     type Message = f64;
//!
//!     fn compute(&mut self, vertex: &mut Vertex<f64>, messages: &[f64], ctx: &mut Context<f64>) {
//!         let sum: f64 = messages.iter().sum();
//!         vertex.value = 0.15 + 0.85 * sum;
//!         for &neighbor in &vertex.edges {
//!             ctx.send(neighbor, vertex.value / vertex.edges.len() as f64);
//!         }
//!     }
//! }
//! ```

pub mod aggregator;
pub mod context;
pub mod message;
pub mod program;
pub mod vertex;
pub mod wasm_export;
pub mod wire;

pub use context::Context;
pub use program::VertexProgram;
pub use vertex::Vertex;
pub use wire::vertex_program_compute;

/// Re-export for convenience in algorithm code.
pub use pregel_common::VertexId;
