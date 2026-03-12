//! Vertex execution: running compute for each vertex in a superstep.

pub mod executor;
pub mod vertex_loop;

pub use executor::VertexExecutor;
pub use vertex_loop::execute_superstep;
