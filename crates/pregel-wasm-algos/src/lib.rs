//! WASM vertex compute modules for Pregel.
//!
//! Build with: cargo build -p pregel-wasm-algos --target wasm32-unknown-unknown --release
//!
//! Exports `compute(input_ptr, input_len, output_ptr, output_max_len) -> output_len`.
//! Input: bincode ComputeInput. Output: bincode ComputeResultWire.

use pregel_sdk::{Context, Vertex, VertexProgram};
use std::collections::HashSet;

/// Connected Components via SDK VertexProgram. Exported as WASM ABI.
struct WasmCc;

impl Default for WasmCc {
    fn default() -> Self {
        Self
    }
}

impl VertexProgram for WasmCc {
    type VertexValue = u64;
    type Message = u64;

    fn compute(
        &mut self,
        vertex: &mut Vertex<Self::VertexValue>,
        messages: &[(u64, Self::Message)],
        ctx: &mut Context<Self::Message>,
    ) {
        let current = vertex.value;
        let min_received = messages.iter().map(|(_, v)| *v).min().unwrap_or(current);
        let new_component = current.min(min_received);

        let mut neighbors: HashSet<u64> = vertex.edges.iter().copied().collect();
        for (src, _) in messages {
            neighbors.insert(*src);
        }

        if messages.is_empty() {
            vertex.value = new_component;
            for &t in &vertex.edges {
                ctx.send(t, new_component);
            }
            return;
        }

        let any_sender_larger = messages.iter().any(|(_, v)| *v > current);
        let should_send = new_component < current || any_sender_larger;

        if !should_send {
            return;
        }

        vertex.value = new_component;
        if new_component < current {
            for &t in &neighbors {
                ctx.send(t, new_component);
            }
        } else {
            let targets: HashSet<u64> = messages
                .iter()
                .filter(|(_, v)| *v > current)
                .map(|(src, _)| *src)
                .collect();
            for t in targets {
                ctx.send(t, new_component);
            }
        }
    }
}

pregel_sdk::export_wasm_compute!(WasmCc);
