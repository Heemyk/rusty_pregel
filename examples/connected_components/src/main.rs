//! Connected Components example using pregel-sdk VertexProgram.
//!
//! Each vertex stores the minimum ID seen in its component. Sends to neighbors
//! (out-edges + reverse edges from message senders). Vote to halt when no update
//! and no sender has a larger value to inform.

use pregel_sdk::{Context, Vertex, VertexProgram};
use std::collections::HashSet;

struct ConnectedComponents;

impl VertexProgram for ConnectedComponents {
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

        // Neighbors = out-edges ∪ message senders (reverse edges)
        let mut neighbors: HashSet<u64> = vertex.edges.iter().copied().collect();
        for (src, _) in messages {
            neighbors.insert(*src);
        }

        if messages.is_empty() {
            // Superstep 0: send initial ID to all outgoing edges
            vertex.value = new_component;
            for &t in &vertex.edges {
                ctx.send(t, new_component);
            }
            return;
        }

        let any_sender_larger = messages.iter().any(|(_, v)| *v > current);
        let should_send = new_component < current || any_sender_larger;

        if !should_send {
            // Vote to halt: no update, no need to inform anyone
            return;
        }

        vertex.value = new_component;
        if new_component < current {
            for &t in &neighbors {
                ctx.send(t, new_component);
            }
        } else {
            // Only inform senders with larger value
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

fn main() {
    println!(
        "Connected Components: impl VertexProgram for ConnectedComponents\n\
         Run with: cargo run -p pregel-cli -- submit --graph <path> --algo cc"
    );
    println!("VertexValue=u64, Message=u64");

    // Quick sanity check: run one vertex through the adapter
    let input = pregel_common::ComputeInput {
        vertex_id: 5,
        value: bincode::serialize(&5u64).unwrap(),
        edges: vec![1, 2, 3],
        messages: vec![
            (1, bincode::serialize(&1u64).unwrap()),
            (2, bincode::serialize(&2u64).unwrap()),
        ],
        superstep: 1,
        total_vertices: 6,
    };
    let mut prog = ConnectedComponents;
    let out = pregel_sdk::vertex_program_compute(&mut prog, &input, 1);
    assert!(out.new_value.is_some());
    assert_eq!(bincode::deserialize::<u64>(out.new_value.as_ref().unwrap()).unwrap(), 1);
    println!("Adapter test passed: vertex 5 updated to component 1");
}
