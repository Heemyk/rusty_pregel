//! Native (Rust) algorithms. Used when no WASM module is provided.

use pregel_common::ComputeInput;
use crate::execution::vertex_loop::ComputeResult;

/// PageRank: each vertex sends rank/out_degree to each neighbor.
/// value = 0.15 + 0.85 * sum(messages)
/// Vote to halt when rank change (delta) is below epsilon (convergence).
const PAGERANK_EPSILON: f64 = 1e-6;

pub fn pagerank_compute(input: &ComputeInput) -> ComputeResult {
    let initial_rank = input
        .value
        .first()
        .and_then(|_| bincode::deserialize::<f64>(&input.value).ok())
        .unwrap_or(1.0);

    let sum: f64 = input
        .messages
        .iter()
        .filter_map(|(_source, payload)| bincode::deserialize::<f64>(payload).ok())
        .sum();

    let new_rank = if input.messages.is_empty() {
        initial_rank
    } else {
        0.15 + 0.85 * sum
    };

    let out_degree = input.edges.len() as f64;
    if out_degree <= 0.0 {
        return ComputeResult::halt(vec![]);
    }

    // Superstep 0: no messages yet, always send to bootstrap.
    // Later steps: vote to halt when converged (|Δ| < ε).
    if !input.messages.is_empty() {
        let delta = (new_rank - initial_rank).abs();
        if delta < PAGERANK_EPSILON {
            return ComputeResult::halt(vec![]);
        }
    }

    let contribution = new_rank / out_degree;
    let mut outgoing = Vec::new();
    for &target in &input.edges {
        let payload = bincode::serialize(&contribution).unwrap();
        outgoing.push((target, payload));
    }
    ComputeResult::update(bincode::serialize(&new_rank).unwrap(), outgoing)
}

/// Connected Components: each vertex stores min(own_id, min(messages)), sends to all neighbors.
///
/// Neighbors = original outgoing edges ∪ message senders (reverse edges—receiving establishes
/// a backward path). Per the Pregel paper: "Send ID[i] to all Nout". We only run when we receive
/// messages. Vote to halt when we don't update and no sender has a larger value to inform.
pub fn connected_components_compute(input: &ComputeInput) -> ComputeResult {
    let current = input
        .value
        .first()
        .and_then(|_| bincode::deserialize::<u64>(&input.value).ok())
        .unwrap_or(input.vertex_id);

    let messages_parsed: Vec<(u64, u64)> = input
        .messages
        .iter()
        .filter_map(|(src, payload)| bincode::deserialize::<u64>(payload).ok().map(|v| (*src, v)))
        .collect();

    let min_received = messages_parsed.iter().map(|(_, v)| v).min().copied().unwrap_or(current);
    let new_component = current.min(min_received);

    // Neighbors = original out-edges ∪ reverse edges (whoever sent us a message)
    let mut neighbors: std::collections::HashSet<u64> = input.edges.iter().copied().collect();
    for (source, _) in &messages_parsed {
        neighbors.insert(*source);
    }

    // Superstep 0: no messages, run all vertices. Send initial ID to outgoing edges only.
    if messages_parsed.is_empty() {
        let payload = bincode::serialize(&new_component).unwrap();
        let outgoing: Vec<(u64, Vec<u8>)> = input
            .edges
            .iter()
            .map(|&t| (t, payload.clone()))
            .collect();
        return ComputeResult::update(payload, outgoing);
    }

    // We received messages. Per paper: if we don't update, vote to halt—unless we need to
    // inform a sender with a larger value (reverse-edge propagation).
    let any_sender_larger = messages_parsed.iter().any(|(_, v)| *v > current);
    let should_send = new_component < current || any_sender_larger;

    if !should_send {
        return ComputeResult::halt(vec![]);
    }

    let payload = bincode::serialize(&new_component).unwrap();
    let targets: std::collections::HashSet<u64> = if new_component < current {
        // We updated: send to all neighbors (edges + reverse edges).
        neighbors.into_iter().collect()
    } else {
        // We didn't update but must inform senders with larger value. Only send to them;
        // our original edges already have our value from a prior superstep.
        messages_parsed
            .iter()
            .filter(|(_, v)| *v > current)
            .map(|(src, _)| *src)
            .collect()
    };
    let outgoing: Vec<(u64, Vec<u8>)> = targets
        .into_iter()
        .map(|t| (t, payload.clone()))
        .collect();
    ComputeResult::update(bincode::serialize(&new_component).unwrap(), outgoing)
}

/// Single-source shortest path (unweighted). Source = vertex 0.
/// Value = distance from source; u64::MAX = unreachable.
/// Message = sender's distance; receiver computes min(current, min(messages) + 1).
pub fn shortest_path_compute(input: &ComputeInput) -> ComputeResult {
    const INF: u64 = u64::MAX;

    let current = input
        .value
        .first()
        .and_then(|_| bincode::deserialize::<u64>(&input.value).ok())
        .unwrap_or(INF);

    let distances: Vec<u64> = input
        .messages
        .iter()
        .filter_map(|(_, p)| bincode::deserialize::<u64>(p).ok())
        .filter(|&d| d != INF)
        .collect();

    let min_received = distances.iter().min().copied();
    let new_dist = match min_received {
        None => current,
        Some(d) => current.min(d.saturating_add(1)),
    };

    if new_dist == INF || input.edges.is_empty() {
        return ComputeResult::halt(vec![]);
    }

    // Send when: (1) we improved, or (2) source in superstep 0 (current==0, no messages, has edges)
    let should_send = new_dist < current || (input.messages.is_empty() && current == 0);
    if !should_send {
        return ComputeResult::halt(vec![]);
    }

    let payload = bincode::serialize(&new_dist).unwrap();
    let outgoing: Vec<(u64, Vec<u8>)> = input.edges.iter().map(|&t| (t, payload.clone())).collect();
    ComputeResult::update(payload, outgoing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagerank_superstep0_sends_contribution_to_neighbors() {
        let input = ComputeInput {
            vertex_id: 1,
            value: bincode::serialize(&0.5_f64).unwrap(),
            edges: vec![2, 3],
            messages: vec![],
            superstep: 0,
            total_vertices: 4,
        };
        let out = pagerank_compute(&input).outgoing;
        assert_eq!(out.len(), 2);
        let contrib: f64 = bincode::deserialize(&out[0].1).unwrap();
        assert!((contrib - 0.25).abs() < 1e-10);
    }

    #[test]
    fn pagerank_with_messages_updates_rank() {
        let input = ComputeInput {
            vertex_id: 1,
            value: bincode::serialize(&0.2_f64).unwrap(),
            edges: vec![2],
            messages: vec![
                (0, bincode::serialize(&0.1_f64).unwrap()),
                (1, bincode::serialize(&0.2_f64).unwrap()),
            ],
            superstep: 1,
            total_vertices: 4,
        };
        let out = pagerank_compute(&input).outgoing;
        assert_eq!(out.len(), 1);
        let contrib: f64 = bincode::deserialize(&out[0].1).unwrap();
        let expected_rank = 0.15 + 0.85 * 0.3;
        assert!((contrib - expected_rank).abs() < 1e-10);
    }

    #[test]
    fn cc_sends_min_component_to_neighbors() {
        let input = ComputeInput {
            vertex_id: 5,
            value: bincode::serialize(&5u64).unwrap(),
            edges: vec![3, 7],
            messages: vec![(99, bincode::serialize(&2u64).unwrap())],
            superstep: 1,
            total_vertices: 100,
        };
        let out = connected_components_compute(&input).outgoing;
        assert_eq!(out.len(), 3); // 2 edges + 1 reverse edge (source 99)
        let comp: u64 = bincode::deserialize(&out[0].1).unwrap();
        assert_eq!(comp, 2);
    }

    #[test]
    fn cc_vote_to_halt_when_no_change() {
        let input = ComputeInput {
            vertex_id: 3,
            value: bincode::serialize(&3u64).unwrap(),
            edges: vec![1, 2],
            messages: vec![(0, bincode::serialize(&3u64).unwrap())],
            superstep: 1,
            total_vertices: 4,
        };
        let out = connected_components_compute(&input).outgoing;
        assert_eq!(out.len(), 0);
    }

    #[test]
    fn cc_superstep0_sends_initial_component_to_neighbors() {
        let input = ComputeInput {
            vertex_id: 2,
            value: bincode::serialize(&2u64).unwrap(),
            edges: vec![0, 1, 3],
            messages: vec![],
            superstep: 0,
            total_vertices: 4,
        };
        let out = connected_components_compute(&input).outgoing;
        assert_eq!(out.len(), 3);
        let comp: u64 = bincode::deserialize(&out[0].1).unwrap();
        assert_eq!(comp, 2);
    }

    /// Vertex 1 has value 1, receives 2 from vertex 2. Doesn't update but must send 1 back to 2
    /// (reverse edge) so 2 can converge to 1. Per Pregel CC: "send messages alongside and against the edge".
    #[test]
    fn sp_source_sends_in_superstep0() {
        let input = ComputeInput {
            vertex_id: 0,
            value: bincode::serialize(&0u64).unwrap(),
            edges: vec![1, 2],
            messages: vec![],
            superstep: 0,
            total_vertices: 5,
        };
        let out = shortest_path_compute(&input).outgoing;
        assert_eq!(out.len(), 2, "source sends to both neighbors");
        let d: u64 = bincode::deserialize(&out[0].1).unwrap();
        assert_eq!(d, 0);
    }

    #[test]
    fn sp_vertex_updates_from_message() {
        let input = ComputeInput {
            vertex_id: 3,
            value: bincode::serialize(&u64::MAX).unwrap(),
            edges: vec![4],
            messages: vec![(1, bincode::serialize(&2u64).unwrap())],
            superstep: 1,
            total_vertices: 5,
        };
        let res = shortest_path_compute(&input);
        assert!(res.new_value.is_some());
        let d: u64 = bincode::deserialize(res.new_value.unwrap().as_slice()).unwrap();
        assert_eq!(d, 3);
        assert_eq!(res.outgoing.len(), 1);
        let sent: u64 = bincode::deserialize(&res.outgoing[0].1).unwrap();
        assert_eq!(sent, 3);
    }

    #[test]
    fn cc_sends_back_when_we_have_smaller_value() {
        let input = ComputeInput {
            vertex_id: 1,
            value: bincode::serialize(&1u64).unwrap(),
            edges: vec![0],
            messages: vec![(2, bincode::serialize(&2u64).unwrap())],
            superstep: 1,
            total_vertices: 3,
        };
        let out = connected_components_compute(&input).outgoing;
        assert_eq!(out.len(), 1, "must send our value 1 back to vertex 2");
        assert_eq!(out[0].0, 2);
        let comp: u64 = bincode::deserialize(&out[0].1).unwrap();
        assert_eq!(comp, 1);
    }
}
