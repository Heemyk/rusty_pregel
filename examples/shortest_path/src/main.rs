//! Shortest Path (unweighted) example using pregel-sdk VertexProgram.
//!
//! Single-source shortest path from vertex 0. Each vertex stores distance from source.
//! Value = u64::MAX for unreachable. Message = sender's distance.
//! Vote to halt when no improvement.

use pregel_sdk::{Context, Vertex, VertexProgram};

const INF: u64 = u64::MAX;

struct ShortestPath;

impl VertexProgram for ShortestPath {
    type VertexValue = u64;
    type Message = u64;

    fn compute(
        &mut self,
        vertex: &mut Vertex<Self::VertexValue>,
        messages: &[(u64, Self::Message)],
        ctx: &mut Context<Self::Message>,
    ) {
        let current = vertex.value;
        let min_received = messages
            .iter()
            .map(|(_, d)| *d)
            .filter(|&d| d != INF)
            .min();
        let new_dist = match min_received {
            None => current,
            Some(d) => current.min(d.saturating_add(1)),
        };

        if new_dist == INF || vertex.edges.is_empty() {
            return;
        }

        // Send when: (1) we improved, or (2) source in superstep 0
        let should_send = new_dist < current || (messages.is_empty() && current == 0);
        if !should_send {
            return;
        }

        vertex.value = new_dist;
        for &t in &vertex.edges {
            ctx.send(t, new_dist);
        }
    }
}

fn main() {
    println!(
        "Shortest Path: impl VertexProgram for ShortestPath\n\
         Run with: cargo run -p pregel-cli -- submit --graph <path> --algo shortest_path"
    );
    println!("VertexValue=u64 (distance), Message=u64");

    // Quick sanity check: run one vertex through the adapter
    let input = pregel_common::ComputeInput {
        vertex_id: 2,
        value: bincode::serialize(&INF).unwrap(),
        edges: vec![3, 4],
        messages: vec![
            (0, bincode::serialize(&0u64).unwrap()),
            (1, bincode::serialize(&1u64).unwrap()),
        ],
        superstep: 1,
        total_vertices: 5,
    };
    let mut prog = ShortestPath;
    let out = pregel_sdk::vertex_program_compute(&mut prog, &input, 1);
    assert!(out.new_value.is_some());
    let dist = bincode::deserialize::<u64>(out.new_value.as_ref().unwrap()).unwrap();
    assert_eq!(dist, 1); // min(0+1, 1+1) = 1 (from vertex 0)
    assert_eq!(out.outgoing.len(), 2);
    println!("Adapter test passed: vertex 2 distance = {}", dist);
}
