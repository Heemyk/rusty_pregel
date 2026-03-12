//! Observability hooks for testing and future Prometheus integration.
//!
//! ## Design
//!
//! Events are no-op by default. Swap in `TestObserver` for tests; `PrometheusObserver`
//! (to be added) will record metrics for production.
//!
//! ## Usage
//!
//! Call `observe().record(event)` from worker/coordinator at key points:
//! - SuperstepStarted / SuperstepCompleted
//! - MessagesSent
//! - VerticesComputed
//! - CheckpointSaved
//!
//! ## Prometheus (future)
//!
//! A `PrometheusObserver` will map events to:
//! - `pregel_superstep_duration_seconds` (histogram, labels: worker_id, superstep)
//! - `pregel_messages_sent_total` (counter, labels: worker_id)
//! - `pregel_vertices_computed_total` (counter, labels: worker_id)
//! - `pregel_checkpoints_saved_total` (counter, labels: worker_id)
//!
//! Set via `set_observer_for_test(Observer::prometheus(...))` at startup.

use std::io::Write;
use std::sync::{Arc, Mutex};

/// Event recorded by the observer.
#[derive(Debug, Clone)]
pub enum ObservableEvent {
    SuperstepStarted { worker_id: u32, superstep: u64 },
    SuperstepCompleted { worker_id: u32, superstep: u64, duration_ms: u64 },
    MessagesSent { worker_id: u32, count: usize, bytes: usize },
    VerticesComputed { worker_id: u32, count: usize },
    CheckpointSaved { worker_id: u32, superstep: u64 },
    /// Inbox contents at start of superstep (verbose=2).
    InboxSnapshot {
        worker_id: u32,
        superstep: u64,
        items: Vec<(u64, Vec<String>)>, // (target_vertex, formatted payloads)
    },
    /// Vertex states for this worker's partition (verbose=2).
    VertexSnapshot {
        worker_id: u32,
        superstep: u64,
        vertices: Vec<(u64, String, Vec<u64>)>, // (id, formatted_value, edges)
    },
    /// Outgoing messages grouped by target worker (verbose=2).
    OutgoingSnapshot {
        worker_id: u32,
        superstep: u64,
        batches: Vec<(u32, Vec<(u64, String)>)>, // (target_worker, [(target_vertex, formatted_payload)])
    },
    /// Count of message batches received from network before this superstep (verbose=2).
    BatchesReceived { worker_id: u32, superstep: u64, batch_count: usize, message_count: usize },
    /// Debug: mark progress through send→report→advance (verbose=2).
    PhaseMarker { worker_id: u32, phase: &'static str, superstep: u64 },
    /// Transport-level debug: connect_start, connect_done, open_bi_start, write_done, etc. (verbose=2).
    TransportDebug {
        worker_id: u32,
        target_worker: u32,
        transport: &'static str,
        phase: &'static str,
        addr: String,
    },
}

pub trait ObserverBackend: Send + Sync {
    fn record(&self, event: ObservableEvent);
}

/// No-op observer.
#[derive(Clone, Default)]
pub struct NoopObserver;

impl ObserverBackend for NoopObserver {
    fn record(&self, _event: ObservableEvent) {}
}

/// Test observer: records events for assertions.
#[derive(Clone, Default)]
pub struct TestObserver {
    events: Arc<Mutex<Vec<ObservableEvent>>>,
}

impl TestObserver {
    pub fn new() -> Self {
        Self { events: Arc::new(Mutex::new(Vec::new())) }
    }
    pub fn events(&self) -> Vec<ObservableEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl ObserverBackend for TestObserver {
    fn record(&self, event: ObservableEvent) {
        self.events.lock().unwrap().push(event);
    }
}

/// Lock used to serialize PrintObserver output within a process. For multi-process
/// workers, we buffer each event into one string and write atomically to reduce
/// character-level interleaving.
static PRINT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn print_block(buf: &str) {
    let _guard = PRINT_LOCK.lock().unwrap();
    let _ = std::io::stderr().write_all(buf.as_bytes());
    let _ = std::io::stderr().flush(); // ensure output appears when stderr is piped
}

/// Print observer: logs events to stderr for human-readable visibility.
/// level 1 = summary; level 2 = full dumps with clear phases (received, state, sent).
#[derive(Clone)]
pub struct PrintObserver {
    pub level: u8,
}

impl PrintObserver {
    pub fn new(level: u8) -> Self {
        Self { level }
    }
}

impl ObserverBackend for PrintObserver {
    fn record(&self, event: ObservableEvent) {
        match event {
            ObservableEvent::SuperstepStarted { worker_id, superstep } => {
                let line = "─".repeat(60);
                let buf = format!("\n{line}\n  [worker {worker_id}] SUPERSTEP {superstep}\n{line}\n");
                print_block(&buf);
            }
            ObservableEvent::SuperstepCompleted {
                worker_id,
                superstep,
                duration_ms,
            } => {
                let buf = format!("  [worker {worker_id}] ✓ step {superstep} done ({duration_ms}ms)\n\n");
                print_block(&buf);
            }
            ObservableEvent::MessagesSent {
                worker_id,
                count,
                bytes,
            } => {
                let buf = format!("  [worker {worker_id}]   → SENT: {count} msgs ({bytes} B)\n");
                print_block(&buf);
            }
            ObservableEvent::VerticesComputed { worker_id, count } => {
                let buf = format!("  [worker {worker_id}]   computed {count} vertices\n");
                print_block(&buf);
            }
            ObservableEvent::CheckpointSaved {
                worker_id,
                superstep,
            } => {
                let buf = format!("  [worker {worker_id}]   💾 checkpoint saved @ step {superstep}\n");
                print_block(&buf);
            }
            ObservableEvent::InboxSnapshot {
                worker_id,
                superstep,
                items,
            } if self.level >= 2 => {
                let mut buf = format!("  [worker {worker_id}] phase: RECEIVED (inbox for step {superstep})\n");
                if items.is_empty() {
                    buf.push_str("      (empty)\n");
                } else {
                    for (target_v, payloads) in items {
                        let preview: String = payloads.join(", ");
                        buf.push_str(&format!("      v{target_v} ← [{preview}]\n"));
                    }
                }
                print_block(&buf);
            }
            ObservableEvent::VertexSnapshot {
                worker_id,
                superstep,
                vertices,
            } if self.level >= 2 => {
                let mut buf = format!("  [worker {worker_id}] phase: STATE (vertices @ step {superstep})\n");
                for (vid, value_str, edges) in vertices {
                    let edges_str = edges.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(", ");
                    buf.push_str(&format!("      v{vid} = {value_str}  edges:[{edges_str}]\n"));
                }
                print_block(&buf);
            }
            ObservableEvent::OutgoingSnapshot {
                worker_id,
                superstep,
                batches,
            } if self.level >= 2 => {
                let mut buf = format!("  [worker {worker_id}] phase: SENT (outgoing @ step {superstep})\n");
                if batches.is_empty() {
                    buf.push_str("      (none)\n");
                } else {
                    let mut all: Vec<(u32, u64, String)> = Vec::new();
                    for (target_w, msgs) in batches {
                        for (target_v, payload_str) in msgs {
                            all.push((target_w, target_v, payload_str.clone()));
                        }
                    }
                    all.sort_by_key(|(w, v, _)| (*w, *v));
                    for (target_w, target_v, payload_str) in all {
                        buf.push_str(&format!("      → worker {target_w} v{target_v}: {payload_str}\n"));
                    }
                }
                print_block(&buf);
            }
            ObservableEvent::BatchesReceived {
                worker_id,
                superstep,
                batch_count,
                message_count,
            } if self.level >= 2 => {
                let buf = format!("  [worker {worker_id}] phase: DRAINED from network → {batch_count} batches, {message_count} msgs (for step {superstep})\n");
                print_block(&buf);
            }
            ObservableEvent::PhaseMarker { worker_id, phase, superstep } if self.level >= 2 => {
                let buf = format!("  [worker {worker_id}] DEBUG: {phase} (step {superstep})\n");
                print_block(&buf);
            }
            ObservableEvent::TransportDebug {
                worker_id,
                target_worker,
                transport,
                phase,
                addr,
            } if self.level >= 2 => {
                let buf = format!("  [worker {worker_id}] {transport} {phase} → worker {target_worker} ({addr})\n");
                print_block(&buf);
            }
            ObservableEvent::InboxSnapshot { .. }
            | ObservableEvent::VertexSnapshot { .. }
            | ObservableEvent::OutgoingSnapshot { .. }
            | ObservableEvent::BatchesReceived { .. }
            | ObservableEvent::PhaseMarker { .. }
            | ObservableEvent::TransportDebug { .. } => {
                // level < 2 or already handled
            }
        }
    }
}

/// Observer: delegates to backend.
pub struct Observer(Arc<dyn ObserverBackend>);

impl Observer {
    pub fn noop() -> Self {
        Self(Arc::new(NoopObserver))
    }
    pub fn test(backend: TestObserver) -> Self {
        Self(Arc::new(backend))
    }
    pub fn verbose() -> Self {
        Self(Arc::new(PrintObserver::new(1)))
    }
    pub fn verbose_level(level: u8) -> Self {
        Self(Arc::new(PrintObserver::new(level)))
    }
    pub fn record(&self, event: ObservableEvent) {
        self.0.record(event);
    }
}

impl Default for Observer {
    fn default() -> Self {
        Self::noop()
    }
}

impl Clone for Observer {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

static OBSERVER: std::sync::OnceLock<Observer> = std::sync::OnceLock::new();
static VERBOSE_LEVEL: std::sync::OnceLock<u8> = std::sync::OnceLock::new();

pub fn observe() -> Observer {
    OBSERVER.get().cloned().unwrap_or_default()
}

/// Returns the current verbose level (0, 1, or 2). Used to avoid building snapshot data when not needed.
pub fn verbose_level() -> u8 {
    VERBOSE_LEVEL.get().copied().unwrap_or(0)
}

/// Reusable timing: run a function and return (result, duration).
pub fn measure<F, T>(f: F) -> (T, std::time::Duration)
where
    F: FnOnce() -> T,
{
    let start = std::time::Instant::now();
    let result = f();
    (result, start.elapsed())
}

/// Set observer for testing. Panics if already set.
pub fn set_observer_for_test(obs: Observer) {
    OBSERVER.set(obs).ok().expect("observer already set");
}

/// Initialize the global observer (e.g. for --verbose). Panics if already set.
pub fn init_observer(obs: Observer) {
    OBSERVER.set(obs).ok().expect("observer already set");
}

/// Initialize the global observer with a verbose level. Panics if already set.
pub fn init_verbose_observer(level: u8) {
    VERBOSE_LEVEL.set(level).ok().expect("verbose level already set");
    OBSERVER
        .set(Observer::verbose_level(level))
        .ok()
        .expect("observer already set");
}
