//! Reusable timing benchmarks for Pregel superstep execution.
//!
//! Run with: cargo bench -p pregel-worker

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pregel_common::Message;
use pregel_core::{Algorithm, HashPartition, PartitionStrategyImpl};
use pregel_storage::{load_and_partition, GraphPartition};
use pregel_worker::execution::execute_superstep_parallel;
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

/// Create a temp graph file from edges.
fn make_graph(edges: &[(u64, u64)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("graph.txt");
    let mut f = std::fs::File::create(&path).unwrap();
    for (a, b) in edges {
        writeln!(f, "{} {}", a, b).unwrap();
    }
    f.flush().unwrap();
    dir
}

/// Load partition for worker 0 from a graph path.
fn load_partition(path: &std::path::Path, workers: usize, algo: Algorithm) -> Arc<GraphPartition> {
    let part = Arc::new(HashPartition);
    let partitions = load_and_partition(path, workers, part.as_ref(), algo).unwrap();
    Arc::new(partitions.into_iter().next().unwrap())
}

fn bench_cc_superstep_0(c: &mut Criterion) {
    let dir = make_graph(&[(0, 1), (1, 2), (2, 0), (3, 4), (4, 5)]);
    let path = dir.path().join("graph.txt");
    let partition = load_partition(&path, 2, Algorithm::ConnectedComponents);
    let inbox: HashMap<u64, Vec<Message>> = HashMap::new();
    let part_impl = Arc::new(HashPartition);

    c.bench_function("cc_superstep_0_small", |b| {
        b.iter(|| {
            execute_superstep_parallel(
                &partition,
                black_box(&inbox),
                black_box(0u64),
                Algorithm::ConnectedComponents,
                None,
                None,
                part_impl.as_ref(),
                2,
            )
        })
    });
}

fn bench_cc_superstep_with_messages(c: &mut Criterion) {
    let dir = make_graph(&[(0, 1), (1, 2), (2, 0), (3, 4), (4, 5)]);
    let path = dir.path().join("graph.txt");
    let partition = load_partition(&path, 2, Algorithm::ConnectedComponents);
    let mut inbox: HashMap<u64, Vec<Message>> = HashMap::new();
    inbox.insert(1, vec![Message { source: 0, target: 1, payload: bincode::serialize(&0u64).unwrap() }]);
    inbox.insert(2, vec![Message { source: 1, target: 2, payload: bincode::serialize(&0u64).unwrap() }]);
    inbox.insert(0, vec![Message { source: 2, target: 0, payload: bincode::serialize(&0u64).unwrap() }]);
    let part_impl = Arc::new(HashPartition);

    c.bench_function("cc_superstep_1_with_messages", |b| {
        b.iter(|| {
            execute_superstep_parallel(
                &partition,
                black_box(&inbox),
                black_box(1u64),
                Algorithm::ConnectedComponents,
                None,
                None,
                part_impl.as_ref(),
                2,
            )
        })
    });
}

fn bench_pagerank_superstep(c: &mut Criterion) {
    let dir = make_graph(&[(0, 1), (1, 2), (2, 0), (3, 4), (4, 5)]);
    let path = dir.path().join("graph.txt");
    let partition = load_partition(&path, 2, Algorithm::Pagerank);
    let mut inbox: HashMap<u64, Vec<Message>> = HashMap::new();
    inbox.insert(1, vec![Message { source: 0, target: 1, payload: bincode::serialize(&0.2_f64).unwrap() }]);
    inbox.insert(2, vec![Message { source: 1, target: 2, payload: bincode::serialize(&0.2_f64).unwrap() }]);
    let part_impl = Arc::new(HashPartition);

    c.bench_function("pagerank_superstep_1", |b| {
        b.iter(|| {
            execute_superstep_parallel(
                &partition,
                black_box(&inbox),
                black_box(1u64),
                Algorithm::Pagerank,
                None,
                None,
                part_impl.as_ref(),
                2,
            )
        })
    });
}

criterion_group!(benches, bench_cc_superstep_0, bench_cc_superstep_with_messages, bench_pagerank_superstep);
criterion_main!(benches);
