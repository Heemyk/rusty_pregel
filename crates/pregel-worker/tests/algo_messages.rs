//! Per-algorithm message format tests.
//!
//! Verifies that PageRank and Connected Components produce the expected message
//! shapes (target vertex, payload type) and values.

use pregel_common::{Message, VertexId};
use pregel_core::{Algorithm, HashPartition, PartitionStrategyImpl};
use pregel_storage::load_and_partition;
use pregel_worker::execution::vertex_loop::execute_superstep_parallel;
use pregel_worker::native_algo::{connected_components_compute, pagerank_compute};
use pregel_worker::execution::vertex_loop::ComputeInput;
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

fn make_temp_graph(edges: &[(u64, u64)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("graph.txt");
    let mut f = std::fs::File::create(&path).unwrap();
    for (a, b) in edges {
        writeln!(f, "{} {}", a, b).unwrap();
    }
    f.flush().unwrap();
    dir
}

#[test]
fn pagerank_messages_are_f64_contributions() {
    let input = ComputeInput {
        vertex_id: 0,
        value: bincode::serialize(&0.25_f64).unwrap(),
        edges: vec![1, 2],
        messages: vec![], // (source, payload)
    };
    let out = pagerank_compute(&input).outgoing;
    assert_eq!(out.len(), 2);
    for (_target, payload) in &out {
        let v: f64 = bincode::deserialize(payload).expect("PageRank payload must be f64");
        assert!(v > 0.0 && v <= 1.0);
    }
}

#[test]
fn cc_messages_are_u64_component_ids() {
    let input = ComputeInput {
        vertex_id: 10,
        value: bincode::serialize(&10u64).unwrap(),
        edges: vec![1, 2, 3],
        messages: vec![(0, bincode::serialize(&5u64).unwrap())],
    };
    let out = connected_components_compute(&input).outgoing;
    assert!(!out.is_empty());
    for (_target, payload) in &out {
        let v: u64 = bincode::deserialize(payload).expect("CC payload must be u64");
        assert_eq!(v, 5);
    }
}

#[test]
fn pagerank_via_execute_superstep_parallel() {
    let dir = make_temp_graph(&[(0, 1), (1, 0), (1, 2), (2, 1)]);
    let path = dir.path().join("graph.txt");
    let partition_impl: Arc<dyn PartitionStrategyImpl> = Arc::new(HashPartition);
    let partitions = load_and_partition(&path, 1, partition_impl.as_ref(), Algorithm::Pagerank).unwrap();
    let partition = Arc::new(partitions.into_iter().next().unwrap());
    let inbox: HashMap<VertexId, Vec<Message>> = HashMap::new();

    let (_updates, out) = execute_superstep_parallel(
        &partition,
        &inbox,
        0,
        Algorithm::Pagerank,
        None,
        None,
        partition_impl.as_ref(),
        1,
    );

    assert!(!out.is_empty());
    for (_source, target, payload) in &out {
        let v: f64 = bincode::deserialize(payload).expect("PageRank must emit f64");
        assert!(*target < 4);
        assert!(v > 0.0 && v <= 1.0);
    }
}

#[test]
fn cc_via_execute_superstep_parallel() {
    let dir = make_temp_graph(&[(0, 1), (1, 0), (2, 3), (3, 2)]);
    let path = dir.path().join("graph.txt");
    let partition_impl: Arc<dyn PartitionStrategyImpl> = Arc::new(HashPartition);
    let partitions = load_and_partition(&path, 1, partition_impl.as_ref(), Algorithm::ConnectedComponents).unwrap();
    let partition = Arc::new(partitions.into_iter().next().unwrap());
    let mut inbox: HashMap<VertexId, Vec<Message>> = HashMap::new();
    inbox.insert(
        1,
        vec![Message {
            source: 0,
            target: 1,
            payload: bincode::serialize(&0u64).unwrap(),
        }],
    );

    let (_updates, out) = execute_superstep_parallel(
        &partition,
        &inbox,
        1,
        Algorithm::ConnectedComponents,
        None,
        None,
        partition_impl.as_ref(),
        1,
    );

    for (_source, _target, payload) in &out {
        let v: u64 = bincode::deserialize(payload).expect("CC must emit u64");
        assert!(v <= 3);
    }
}
