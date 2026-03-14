//! Core type definitions used throughout the Pregel system.
//!
//! These types define the "vocabulary" of the framework. Every message sent between
//! workers, every vertex lookup, and every partition calculation uses these types.

use serde::{Deserialize, Serialize};

/// ABI limit constants. See docs/ABI_SPEC.md.
pub const MAX_VALUE_LEN: usize = 1024 * 1024;      // 1 MB
pub const MAX_EDGES_LEN: usize = 1_000_000;
pub const MAX_MESSAGES_LEN: usize = 1_000_000;
pub const MAX_MESSAGE_PAYLOAD_LEN: usize = 64 * 1024;  // 64 KB

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
/// * `source` - The vertex that sent this message. Used by CC for reverse edges.
/// * `target` - The vertex this message is addressed to (can be on any worker)
/// * `payload` - The serialized message content. Interpretation depends on the algorithm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub source: VertexId,
    pub target: VertexId,
    pub payload: Vec<u8>,
}

/// Input for vertex compute (WASM or native). Shared ABI with WASM modules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeInput {
    pub vertex_id: VertexId,
    pub value: Vec<u8>,
    pub edges: Vec<VertexId>,
    pub messages: Vec<(VertexId, Vec<u8>)>,
    /// Current superstep (0-indexed). Needed for superstep-dependent logic.
    #[serde(default)]
    pub superstep: u64,
    /// Total vertices in the graph. Needed for e.g. PageRank 1/N.
    #[serde(default)]
    pub total_vertices: u64,
}

impl ComputeInput {
    /// Validate input per ABI spec. Returns `Ok(())` or an error description.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.value.len() > MAX_VALUE_LEN {
            return Err("value exceeds MAX_VALUE_LEN");
        }
        if self.edges.len() > MAX_EDGES_LEN {
            return Err("edges exceeds MAX_EDGES_LEN");
        }
        if self.messages.len() > MAX_MESSAGES_LEN {
            return Err("messages exceeds MAX_MESSAGES_LEN");
        }
        for (_, p) in &self.messages {
            if p.len() > MAX_MESSAGE_PAYLOAD_LEN {
                return Err("message payload exceeds MAX_MESSAGE_PAYLOAD_LEN");
            }
        }
        Ok(())
    }
}

/// Output from WASM compute: value update + outgoing messages. Bincode-serialized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeResultWire {
    pub new_value: Option<Vec<u8>>,
    pub outgoing: Vec<(VertexId, Vec<u8>)>,
}

impl ComputeResultWire {
    /// Validate output per ABI spec.
    pub fn validate(&self) -> Result<(), &'static str> {
        if let Some(ref v) = self.new_value {
            if v.len() > MAX_VALUE_LEN {
                return Err("new_value exceeds MAX_VALUE_LEN");
            }
        }
        if self.outgoing.len() > MAX_EDGES_LEN {
            return Err("outgoing exceeds MAX_EDGES_LEN");
        }
        for (_, p) in &self.outgoing {
            if p.len() > MAX_MESSAGE_PAYLOAD_LEN {
                return Err("outgoing payload exceeds MAX_MESSAGE_PAYLOAD_LEN");
            }
        }
        Ok(())
    }
}

/// ABI guest error codes. See docs/ABI_SPEC.md §2.2.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbiErrorCode {
    Invalid = -1,
    Deserialize = -2,
    Serialize = -3,
    OutputOverrun = -4,
    Alloc = -5,
    User = -6,
}

impl AbiErrorCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
    pub fn from_i32(v: i32) -> Option<Self> {
        match v {
            -1 => Some(Self::Invalid),
            -2 => Some(Self::Deserialize),
            -3 => Some(Self::Serialize),
            -4 => Some(Self::OutputOverrun),
            -5 => Some(Self::Alloc),
            -6 => Some(Self::User),
            _ => None,
        }
    }
}
