//! The [`VertexData`] struct: how we store a vertex on disk and in memory.
//!
//! This is the "runtime" representation – generic over value type at the SDK
//! level, but stored as bytes here for serialization and WASM compatibility.

use pregel_common::VertexId;

/// Raw vertex data as stored by the runtime.
///
/// Values and messages are stored as `Vec<u8>` because:
/// * Different algorithms use different types (f64, u64, etc.)
/// * We need to serialize for checkpoints and network
/// * The WASM runtime works with raw memory/bytes
///
/// The SDK layer (or WASM guest) deserializes `value` into the algorithm's
/// concrete type when running `compute()`.
#[derive(Debug, Clone)]
pub struct VertexData {
    pub id: VertexId,
    pub value: Vec<u8>,
    pub edges: Vec<VertexId>,
}
