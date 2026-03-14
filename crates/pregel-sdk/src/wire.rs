//! Adapter: bridge VertexProgram (typed API) to ComputeInput / ComputeResultWire (ABI).
//!
//! Use this to run any `VertexProgram` implementation against the wire format.
//! Needed for: native path (optional), WASM export macro.

use crate::{Context, Vertex, VertexProgram};
use pregel_common::{ComputeInput, ComputeResultWire, VertexId};
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Run a VertexProgram against wire-format input, return wire-format output.
///
/// Deserializes `ComputeInput` into typed `Vertex` + messages, calls `program.compute()`,
/// serializes the result to `ComputeResultWire`. Requires `VertexValue` and `Message`
/// to be bincode-serializable.
///
/// The runtime initializes vertex values per-algo before the first superstep, so
/// `input.value` should always be valid. For superstep-0 vertices with empty value,
/// we use `vertex_id` as u64 (CC) or `1.0` as f64 (PageRank) as fallbacks.
pub fn vertex_program_compute<P>(program: &mut P, input: &ComputeInput, superstep: u64) -> ComputeResultWire
where
    P: VertexProgram,
    P::VertexValue: Serialize + DeserializeOwned,
    P::Message: Serialize + DeserializeOwned,
{
    let value: P::VertexValue = bincode::deserialize(&input.value)
        .or_else(|_| bincode::deserialize(&bincode::serialize(&input.vertex_id).unwrap()))
        .or_else(|_| bincode::deserialize(&bincode::serialize(&1.0_f64).unwrap()))
        .expect("VertexValue must deserialize from input, or from u64/f64 fallback");

    let mut vertex = Vertex {
        id: input.vertex_id,
        value,
        edges: input.edges.clone(),
    };

    let messages: Vec<(VertexId, P::Message)> = input
        .messages
        .iter()
        .filter_map(|(src, p)| bincode::deserialize(p).ok().map(|m| (*src, m)))
        .collect();

    let total = input.total_vertices;
    let mut ctx = Context::new(superstep, total);
    program.compute(&mut vertex, &messages, &mut ctx);

    let new_value = Some(bincode::serialize(&vertex.value).expect("vertex value must serialize"));
    let outgoing: Vec<(VertexId, Vec<u8>)> = ctx
        .outgoing
        .iter()
        .map(|(target, msg)| (*target, bincode::serialize(msg).expect("message must serialize")))
        .collect();

    ComputeResultWire { new_value, outgoing }
}
