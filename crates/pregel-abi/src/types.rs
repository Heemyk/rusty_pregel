//! ABI wire types: strongly typed, validated.

use serde::{Deserialize, Serialize};

/// Vertex ID. 64-bit to support large graphs.
pub type VertexId = u64;

/// Input passed to vertex compute. Bincode-serialized by host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeInput {
    pub vertex_id: VertexId,
    /// Serialized vertex value (algorithm-specific)
    pub value: Vec<u8>,
    /// Outgoing edge targets
    pub edges: Vec<VertexId>,
    /// (source_vertex_id, serialized_message) from previous superstep
    pub messages: Vec<(VertexId, Vec<u8>)>,
}

/// Output from vertex compute. Bincode-serialized by guest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeResultWire {
    /// Updated vertex value, or None to vote halt / no change
    pub new_value: Option<Vec<u8>>,
    /// (target_vertex_id, serialized_message) to send
    pub outgoing: Vec<(VertexId, Vec<u8>)>,
}

impl ComputeInput {
    /// Validate input before passing to algorithm. Returns error message if invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.edges.len() > 1_000_000 {
            return Err("edges count exceeds limit (1M)".into());
        }
        if self.messages.len() > 100_000 {
            return Err("messages count exceeds limit (100K)".into());
        }
        if self.value.len() > 64 * 1024 {
            return Err("vertex value exceeds 64KB".into());
        }
        Ok(())
    }
}

impl ComputeResultWire {
    /// Validate output before serialization. Returns error message if invalid.
    pub fn validate(&self, max_outgoing: usize) -> Result<(), String> {
        if self.outgoing.len() > max_outgoing {
            return Err(format!(
                "outgoing messages {} exceed limit {}",
                self.outgoing.len(),
                max_outgoing
            ));
        }
        if let Some(ref v) = self.new_value {
            if v.len() > 64 * 1024 {
                return Err("new_value exceeds 64KB".into());
            }
        }
        for (_, p) in &self.outgoing {
            if p.len() > 64 * 1024 {
                return Err("outgoing payload exceeds 64KB".into());
            }
        }
        Ok(())
    }
}
