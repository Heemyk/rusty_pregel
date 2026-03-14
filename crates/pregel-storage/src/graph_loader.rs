//! Graph loading from edge list format.
//!
//! Format: one edge per line, "src dest" or "src dest weight". Comments (#) and empty lines ignored.

use pregel_common::{Result, VertexId};
use pregel_core::{Algorithm, AlgoMetadata, PartitionStrategyImpl, ResultQuery};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::{GraphPartition, VertexData};

/// Load a graph from an edge list file and partition across workers.
///
/// Returns one GraphPartition per worker. Vertices get empty initial value (algorithm-dependent;
/// the WASM/runtime will initialize).
pub fn load_and_partition(
    path: impl AsRef<Path>,
    worker_count: usize,
    partition_impl: &dyn PartitionStrategyImpl,
    algo: Algorithm,
) -> Result<Vec<GraphPartition>> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);

    // First pass: collect edges, build adjacency lists
    let mut adj: HashMap<VertexId, Vec<VertexId>> = HashMap::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.split('#').next().unwrap_or(&line).trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let src: VertexId = parts[0].parse().map_err(|_| {
            pregel_common::PregelError::Serialization(format!("Invalid vertex id: {}", parts[0]))
        })?;
        let dst: VertexId = parts[1].parse().map_err(|_| {
            pregel_common::PregelError::Serialization(format!("Invalid vertex id: {}", parts[1]))
        })?;

        adj.entry(src).or_default().push(dst);
        adj.entry(dst).or_default(); // ensure dst exists as vertex
    }

    // Second pass: assign vertices to partitions
    let n = adj.len() as f64;
    let mut partitions: Vec<GraphPartition> = (0..worker_count).map(|_| GraphPartition::new()).collect();

    for (vid, edges) in adj {
        let worker_id = partition_impl.partition(vid, worker_count);
        let worker_id = worker_id as usize;
        if worker_id < partitions.len() {
            let value = match algo {
                Algorithm::Pagerank => {
                    let initial = 1.0 / n;
                    bincode::serialize(&initial).unwrap_or_default()
                }
                Algorithm::ConnectedComponents => bincode::serialize(&vid).unwrap_or_default(), // init with self
                Algorithm::ShortestPath => {
                    let dist = if vid == 0 { 0u64 } else { u64::MAX };
                    bincode::serialize(&dist).unwrap_or_default()
                }
            };
            partitions[worker_id].add_vertex(VertexData {
                id: vid,
                value,
                edges,
            });
        }
    }

    Ok(partitions)
}

/// Reset vertex values in a partition for a new algorithm run.
/// Edges stay unchanged; only value bytes are updated per algo.
pub fn reset_partition_for_algo(
    partition: &mut crate::GraphPartition,
    algo: Algorithm,
    total_vertices: u64,
) {
    let n = total_vertices as f64;
    for (vid, v) in partition.vertices.iter_mut() {
        v.value = match algo {
            Algorithm::Pagerank => {
                let initial = 1.0 / n;
                bincode::serialize(&initial).unwrap_or_default()
            }
            Algorithm::ConnectedComponents => bincode::serialize(vid).unwrap_or_default(),
            Algorithm::ShortestPath => {
                let dist = if *vid == 0 { 0u64 } else { u64::MAX };
                bincode::serialize(&dist).unwrap_or_default()
            }
        };
    }
}

/// Extract vertex results from a partition per algorithm metadata.
/// Used when workers halt to report job results to the coordinator.
pub fn extract_partition_results(
    partition: &crate::GraphPartition,
    algo: Algorithm,
) -> Vec<(VertexId, Vec<u8>)> {
    let meta = AlgoMetadata::for_algo(algo);
    match &meta.query {
        ResultQuery::AllVertexValues => partition
            .vertices
            .iter()
            .map(|(vid, v)| (*vid, v.value.clone()))
            .collect(),
        ResultQuery::VertexSubset(ids) => ids
            .iter()
            .filter_map(|vid| {
                partition
                    .vertices
                    .get(vid)
                    .map(|v| (*vid, v.value.clone()))
            })
            .collect(),
    }
}
