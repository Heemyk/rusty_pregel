//! PageRank example using pregel-sdk VertexProgram.
//!
//! Each vertex stores its rank; sends rank/out_degree to each neighbor.
//! Formula: rank = 0.15 + 0.85 * sum(incoming contributions).
//! Vote to halt when rank change (delta) is below epsilon.

use pregel_sdk::{Context, Vertex, VertexProgram};

const EPSILON: f64 = 1e-6;

struct PageRank;

impl VertexProgram for PageRank {
    type VertexValue = f64;
    type Message = f64;

    fn compute(
        &mut self,
        vertex: &mut Vertex<Self::VertexValue>,
        messages: &[(u64, Self::Message)],
        ctx: &mut Context<Self::Message>,
    ) {
        let current = vertex.value;
        let sum: f64 = messages.iter().map(|(_, v)| *v).sum();
        let new_rank = if messages.is_empty() {
            current
        } else {
            0.15 + 0.85 * sum
        };

        let out_degree = vertex.edges.len() as f64;
        if out_degree <= 0.0 {
            return;
        }

        if !messages.is_empty() {
            let delta = (new_rank - current).abs();
            if delta < EPSILON {
                return; // Vote to halt
            }
        }

        vertex.value = new_rank;
        let contrib = new_rank / out_degree;
        for &t in &vertex.edges {
            ctx.send(t, contrib);
        }
    }
}

fn main() {
    println!(
        "PageRank: impl VertexProgram for PageRank\n\
         Run with: cargo run -p pregel-cli -- submit --graph <path> --algo pagerank"
    );
    println!("VertexValue=f64, Message=f64");

    // Quick sanity check: run one vertex through the adapter
    let input = pregel_common::ComputeInput {
        vertex_id: 1,
        value: bincode::serialize(&0.25_f64).unwrap(),
        edges: vec![2, 3],
        messages: vec![
            (0, bincode::serialize(&0.1_f64).unwrap()),
            (4, bincode::serialize(&0.2_f64).unwrap()),
        ],
        superstep: 1,
        total_vertices: 5,
    };
    let mut prog = PageRank;
    let out = pregel_sdk::vertex_program_compute(&mut prog, &input, 1);
    assert!(out.new_value.is_some());
    let rank = bincode::deserialize::<f64>(out.new_value.as_ref().unwrap()).unwrap();
    let expected = 0.15 + 0.85 * (0.1 + 0.2);
    assert!((rank - expected).abs() < 1e-9);
    assert_eq!(out.outgoing.len(), 2);
    let contrib = bincode::deserialize::<f64>(&out.outgoing[0].1).unwrap();
    assert!((contrib - rank / 2.0).abs() < 1e-9);
    println!("Adapter test passed: vertex 1 rank = {:.6}", rank);
}
