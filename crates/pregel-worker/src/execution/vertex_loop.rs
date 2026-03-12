//! The inner loop: for each vertex, get messages, run compute, collect outgoing.

use pregel_common::{Message, VertexId};
use pregel_storage::GraphPartition;
use std::collections::HashMap;

/// Execute one superstep for this partition.
///
/// For each vertex, fetches messages from the inbox, runs compute (placeholder:
/// currently just passes through), and collects (target, payload) pairs.
///
/// **Full implementation** would: deserialize vertex + messages, call
/// `VertexExecutor::execute()` (WASM or native), serialize results.
pub fn execute_superstep(
    partition: &GraphPartition,
    inbox: &HashMap<VertexId, Vec<Message>>,
) -> Vec<(VertexId, Vec<u8>)> {
    let mut outgoing = Vec::new();

    for (vertex_id, _vertex_data) in &partition.vertices {
        let messages = inbox.get(vertex_id).map(|m| m.as_slice()).unwrap_or(&[]);

        // Placeholder: actual execution would invoke WASM or native vertex program
        for msg in messages {
            outgoing.push((msg.target, msg.payload.clone()));
        }
    }

    outgoing
}
