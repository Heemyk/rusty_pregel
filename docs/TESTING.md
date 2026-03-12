# Testing Strategy

## Unit Tests

- **`pregel-worker/src/native_algo.rs`** – PageRank and Connected Components compute logic
- **`pregel-core`** – Partition functions (HashPartition, CustomPartition)
- **`pregel-storage`** – Graph loading and initial value assignment

Run: `cargo test -p pregel-worker` (includes unit tests)

## Per-Algorithm Message Tests

`crates/pregel-worker/tests/algo_messages.rs`:

- **PageRank**: Messages are f64 contributions; `execute_superstep_parallel` produces valid payloads
- **Connected Components**: Messages are u64 component IDs; CC convergence behavior

These tests assert the **shape** of messages (type, value ranges) for each algorithm.

## Integration Tests

Run coordinator + workers (e.g. via `pregel submit`):

```bash
cargo build -p pregel-worker -p pregel-coordinator -p pregel-cli
cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --workers 2 --algo pagerank --transport tcp
```

For QUIC:

```bash
cargo run -p pregel-cli -- submit --graph examples/data/sample.graph --workers 2 --transport quic
```

## End-to-End

1. Submit a job with `pregel submit`
2. Let it run for a few supersteps (or until convergence for algorithms with halt)
3. Inspect outputs (or add assertions in a separate E2E harness)

## Observability in Tests

Use `pregel_observability::set_observer_for_test(Observer::test(TestObserver::new()))` to capture events:

```rust
#[test]
fn test_superstep_events() {
    let backend = TestObserver::new();
    pregel_observability::set_observer_for_test(Observer::test(backend.clone()));
    // ... run worker loop for one superstep ...
    let events = backend.events();
    assert!(events.iter().any(|e| matches!(e, ObservableEvent::SuperstepStarted { .. })));
}
```

Prometheus integration will map these same events to metrics later.
