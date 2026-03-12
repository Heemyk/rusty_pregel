//! The graph partition: a worker's local vertex store.
//!
//! A [`GraphPartition`] is the subset of the graph assigned to one worker.
//! It's a map from vertex ID to vertex data.

use std::collections::HashMap;

use pregel_common::VertexId;

use crate::VertexData;

/// A worker's local partition of the graph.
///
/// Contains only the vertices assigned to this worker by the partition function.
/// Implemented as a HashMap for O(1) lookup by vertex ID.
///
/// # When vertices are added
///
/// Typically at job start: the coordinator (or a loader) reads the graph,
/// partitions it, and sends each worker its partition. Or workers load from
/// a distributed store (e.g., S3) with a filter for their partition.
#[derive(Debug, Default)]
pub struct GraphPartition {
    pub vertices: HashMap<VertexId, VertexData>,
}

impl GraphPartition {
    pub fn new() -> Self {
        Self {
            vertices: HashMap::new(),
        }
    }

    pub fn add_vertex(&mut self, vertex: VertexData) {
        self.vertices.insert(vertex.id, vertex);
    }

    pub fn get_vertex(&self, id: VertexId) -> Option<&VertexData> {
        self.vertices.get(&id)
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
}
