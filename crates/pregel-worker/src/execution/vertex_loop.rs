//! Parallel vertex execution within a superstep.

use pregel_common::{ComputeInput, ComputeResultWire, Message, Result, VertexId};

use pregel_core::{Algorithm, PartitionStrategyImpl};
use pregel_storage::GraphPartition;
use pregel_wasm::{WasmExecutor, WasmModule};
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

/// Output from WASM compute: (target, payload) pairs.
pub type ComputeOutput = Vec<(VertexId, Vec<u8>)>;

/// Result of compute: optional value update + outgoing messages.
#[derive(Debug, Clone)]
pub struct ComputeResult {
    pub new_value: Option<Vec<u8>>,
    pub outgoing: ComputeOutput,
}

impl ComputeResult {
    pub fn halt(outgoing: ComputeOutput) -> Self {
        Self { new_value: None, outgoing }
    }
    pub fn update(new_value: Vec<u8>, outgoing: ComputeOutput) -> Self {
        Self { new_value: Some(new_value), outgoing }
    }
}

/// Execute one superstep in parallel using rayon (CPU-bound work).
/// Runs inside tokio::spawn_blocking so we don't block the async runtime.
/// In superstep 0, all vertices run (no messages yet). In superstep > 0, only vertices with messages run.
/// Returns error on WASM guest failure (ABI error codes propagated).
pub fn execute_superstep_parallel(
    partition: &Arc<GraphPartition>,
    inbox: &HashMap<VertexId, Vec<Message>>,
    superstep: u64,
    total_vertices: u64,
    algo: Algorithm,
    wasm_executor: Option<&WasmExecutor>,
    wasm_module: Option<&WasmModule>,
    _partition_impl: &dyn PartitionStrategyImpl,
    _worker_count: usize,
) -> Result<(Vec<(VertexId, Vec<u8>)>, Vec<(VertexId, VertexId, Vec<u8>)>)> {
    let vertices_to_run: Vec<_> = if superstep == 0 {
        partition.vertices.iter().map(|(vid, v)| (*vid, v.clone())).collect()
    } else {
        partition
            .vertices
            .iter()
            .filter(|(vid, _)| inbox.get(vid).map_or(false, |msgs| !msgs.is_empty()))
            .map(|(vid, v)| (*vid, v.clone()))
            .collect()
    };

    let results: Result<Vec<_>> = vertices_to_run
        .par_iter()
        .map(|(vertex_id, vertex_data)| -> Result<(Option<Vec<u8>>, Vec<(VertexId, Vec<u8>)>)> {
            let messages: Vec<(VertexId, Vec<u8>)> = inbox
                .get(vertex_id)
                .map(|m| m.iter().map(|msg| (msg.source, msg.payload.clone())).collect())
                .unwrap_or_default();

            let input = ComputeInput {
                vertex_id: vertex_data.id,
                value: vertex_data.value.clone(),
                edges: vertex_data.edges.clone(),
                messages,
                superstep,
                total_vertices,
            };

            let result = if let (Some(exec), Some(modu)) = (wasm_executor, wasm_module) {
                let bytes = bincode::serialize(&input).unwrap();
                let output = exec.compute(modu, &bytes)?;
                let wire: ComputeResultWire = bincode::deserialize(&output)
                    .map_err(|e| pregel_common::PregelError::Serialization(e.to_string()))?;
                (wire.new_value, wire.outgoing)
            } else {
                let res = match algo {
                    Algorithm::Pagerank => crate::native_algo::pagerank_compute(&input),
                    Algorithm::ConnectedComponents => crate::native_algo::connected_components_compute(&input),
                    Algorithm::ShortestPath => crate::native_algo::shortest_path_compute(&input),
                };
                (res.new_value, res.outgoing)
            };
            Ok(result)
        })
        .collect();

    let results = results?;
    let mut value_updates = Vec::new();
    let mut outgoing = Vec::new();
    for ((vertex_id, _), (new_val, msgs)) in vertices_to_run.iter().zip(results.iter()) {
        if let Some(v) = new_val {
            value_updates.push((*vertex_id, v.clone()));
        }
        for (target, payload) in msgs {
            outgoing.push((*vertex_id, *target, payload.clone()));
        }
    }
    Ok((value_updates, outgoing))
}
