//! # pregel-storage
//!
//! Graph storage and partitioning for the Pregel runtime.
//!
//! Each worker holds a [`GraphPartition`] – its subset of the graph. Vertices
//! are stored as [`VertexData`] with id, value (bytes), and edges. Use [`partition`]
//! to determine which worker owns a given vertex.

pub mod graph;
pub mod graph_loader;
pub mod partitioner;
pub mod vertex_store;

pub use graph::GraphPartition;
pub use graph_loader::{load_and_partition, reset_partition_for_algo};
pub use partitioner::partition;
pub use vertex_store::VertexData;
