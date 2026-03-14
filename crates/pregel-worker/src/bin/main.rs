//! Pregel worker binary: runs the BSP superstep loop.

use pregel_checkpoint::CheckpointManager;
use pregel_common::WorkerId;
use pregel_core::{Algorithm, HashPartition, PartitionStrategyImpl};
use pregel_messaging::MessageBatch;
use pregel_observability::{init_prometheus_observer, init_verbose_observer, observe, verbose_level, ObservableEvent};
use pregel_storage::{load_and_partition, reset_partition_for_algo, GraphPartition};
use pregel_worker::coordinator_client::CoordinatorGrpcClient;
use pregel_worker::execution::execute_superstep_parallel;
use pregel_worker::messaging::MessageInbox;
use pregel_worker::transport::{run_receiver, send_batch, worker_addresses};
use pregel_worker::transport_quic::{quic_run_receiver, quic_server, QuicConnectionCache};
use pregel_worker::Worker;
use pregel_wasm::{WasmExecutor, WasmModule};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

const DEFAULT_ALGO: Algorithm = Algorithm::Pagerank;
const CHECKPOINT_INTERVAL: u64 = 10;

#[derive(Clone, Copy, Default)]
enum TransportKind {
    #[default]
    Tcp,
    Quic,
}

impl std::str::FromStr for TransportKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tcp" => Ok(Self::Tcp),
            "quic" => Ok(Self::Quic),
            _ => Err(format!("Unknown transport: {}", s)),
        }
    }
}

/// Parse verbose level: --verbose, -v = 1; --verbose=2 = 2.
fn parse_verbose_level(args: &[String]) -> u8 {
    let mut level = 0u8;
    let mut i = 6;
    while i < args.len() {
        if args[i].starts_with("--verbose") {
            if let Some(suffix) = args[i].strip_prefix("--verbose=") {
                level = level.max(suffix.parse().unwrap_or(1));
            } else {
                level = level.max(1);
            }
            i += 1;
        } else if args[i] == "-v" {
            level += 1;
            i += 1;
        } else {
            i += 1;
        }
    }
    level.min(2)
}

fn parse_args(
    args: &[String],
) -> Result<
    (
        WorkerId,
        String,
        PathBuf,
        usize,
        u16,
        Algorithm,
        TransportKind,
        Option<PathBuf>,
        Option<String>,
        Option<u16>,
        u8,
        bool,
    ),
    String,
> {
    if args.len() < 6 {
        return Err(format!(
            "Usage: {} <worker_id> <coordinator_addr> <graph_path> <worker_count> <listen_port> [--session] [--algo pagerank|cc] [--program <wasm-path>] [--transport tcp|quic] [--checkpoint-dir <path>] [--metrics-port <port>] [--verbose[=2]|-v|-vv]",
            args.first().unwrap_or(&"pregel-worker".into())
        ));
    }

    let worker_id: WorkerId = args[1].parse().map_err(|_| "Invalid worker_id")?;
    let coordinator_addr = args[2].clone();
    let graph_path = PathBuf::from(&args[3]);
    let worker_count: usize = args[4].parse().map_err(|_| "Invalid worker_count")?;
    let listen_port: u16 = args[5].parse().map_err(|_| "Invalid listen_port")?;

    let mut algo = DEFAULT_ALGO;
    let mut transport = TransportKind::default();
    let mut program = None;
    let mut checkpoint_dir = None;
    let mut metrics_port = None;
    let mut session = false;

    let mut i = 6;
    while i < args.len() {
        if args[i] == "--session" {
            session = true;
            i += 1;
        } else if args[i] == "--algo" && i + 1 < args.len() {
            algo = args[i + 1].parse().map_err(|e: String| e)?;
            i += 2;
        } else if args[i] == "--program" && i + 1 < args.len() {
            program = Some(PathBuf::from(&args[i + 1]));
            i += 2;
        } else if args[i] == "--transport" && i + 1 < args.len() {
            transport = args[i + 1].parse().map_err(|e: String| e)?;
            i += 2;
        } else if args[i] == "--checkpoint-dir" && i + 1 < args.len() {
            checkpoint_dir = Some(args[i + 1].clone());
            i += 2;
        } else if args[i] == "--metrics-port" && i + 1 < args.len() {
            metrics_port = Some(args[i + 1].parse().map_err(|_| "Invalid metrics-port")?);
            i += 2;
        } else if args[i].starts_with("--verbose") || args[i] == "-v" {
            i += 1;
        } else {
            i += 1;
        }
    }

    let verbose = parse_verbose_level(args);

    Ok((
        worker_id,
        coordinator_addr,
        graph_path,
        worker_count,
        listen_port,
        algo,
        transport,
        program,
        checkpoint_dir,
        metrics_port,
        verbose,
        session,
    ))
}

/// Format a payload for display based on algorithm.
fn format_payload(payload: &[u8], algo: Algorithm) -> String {
    match algo {
        Algorithm::Pagerank => {
            bincode::deserialize::<f64>(payload)
                .map(|v| format!("{v:.6}"))
                .unwrap_or_else(|_| format!("<{}B>", payload.len()))
        }
        Algorithm::ConnectedComponents => {
            bincode::deserialize::<u64>(payload)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| format!("<{}B>", payload.len()))
        }
        Algorithm::ShortestPath => {
            bincode::deserialize::<u64>(payload)
                .map(|v| if v == u64::MAX { "∞".into() } else { v.to_string() })
                .unwrap_or_else(|_| format!("<{}B>", payload.len()))
        }
    }
}

/// Format vertex value for display.
fn format_value(value: &[u8], algo: Algorithm) -> String {
    format_payload(value, algo)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Must run before any rustls code (e.g. quinn, platform verifier). Prevents
    // "Could not automatically determine CryptoProvider" when both ring and aws-lc-rs
    // are in the dep tree via different crates.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let args: Vec<String> = std::env::args().collect();
    let (worker_id, coordinator_addr, graph_path, worker_count, listen_port, mut algo, transport, program, checkpoint_dir, metrics_port, verbose, session) =
        match parse_args(&args) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };

    if let Some(port) = metrics_port {
        let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()
            .map_err(|e: std::net::AddrParseError| format!("invalid metrics-port: {}", e))?;
        init_prometheus_observer(Some(addr), verbose)
            .map_err(|e| format!("prometheus init failed: {}", e))?;
        // Yield so the metrics server task can bind before we proceed.
        tokio::task::yield_now().await;
    } else if verbose > 0 {
        init_verbose_observer(verbose);
    }

    let (wasm_executor, wasm_module) = match &program {
        Some(path) => {
            let module = WasmModule::from_path(path).expect("failed to load WASM module");
            (Some(WasmExecutor::new()), Some(module))
        }
        None => (None, None),
    };
    let wasm_executor = wasm_executor.map(Arc::new);
    let wasm_module = wasm_module.map(Arc::new);

    let host = coordinator_addr
        .split(':')
        .next()
        .unwrap_or("127.0.0.1");
    let coordinator_port: u16 = coordinator_addr
        .split(':')
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(5000);

    let partition_impl: Arc<dyn PartitionStrategyImpl> = Arc::new(HashPartition);
    let load_algo = if session { Algorithm::ConnectedComponents } else { algo };
    let partitions = load_and_partition(&graph_path, worker_count, partition_impl.as_ref(), load_algo)?;
    let partition = partitions
        .into_iter()
        .nth(worker_id as usize)
        .unwrap_or_else(GraphPartition::new);

    let mut coordinator = CoordinatorGrpcClient::connect(format!("http://{}", coordinator_addr)).await?;

    // Spawn receiver BEFORE registering so we're listening when the barrier fires.
    // Keep a clone of batch_tx for local injection when we send to ourselves.
    let (batch_tx, mut batch_rx) = mpsc::channel::<MessageBatch>(1024);
    let batch_tx_for_local = batch_tx.clone();
    let mut quic_cache: Option<QuicConnectionCache> = None;
    match transport {
        TransportKind::Tcp => {
            let listener = TcpListener::bind(format!("0.0.0.0:{}", listen_port)).await?;
            tokio::spawn(run_receiver(listener, batch_tx));
        }
        TransportKind::Quic => {
            let addr: SocketAddr = format!("0.0.0.0:{}", listen_port).parse()?;
            let endpoint = quic_server(addr)?;
            let endpoint_for_sender = endpoint.clone();
            tokio::spawn(quic_run_receiver(endpoint, batch_tx));
            quic_cache = Some(QuicConnectionCache::new(endpoint_for_sender));
        }
    }

    coordinator
        .register_worker(worker_id, format!("{}:{}", host, listen_port), partition.vertex_count() as u64)
        .await?;

    // Barrier: don't start until all workers are registered and listening.
    coordinator.wait_for_all_ready().await?;

    let worker_partition = Arc::new(RwLock::new(partition.clone()));
    let worker = Worker::new(worker_id, partition, worker_count);
    let addresses = worker_addresses(host, coordinator_port, worker_count);

    let checkpoint_manager = checkpoint_dir.map(|d| CheckpointManager::new(d));

    'job_loop: loop {
        if session {
            let (job_id, algo_str, _program_str, total_vertices) =
                coordinator.wait_for_job_start().await?;
            algo = algo_str.parse().map_err(|e: String| format!("invalid algo from coordinator: {}", e))?;
            reset_partition_for_algo(
                &mut worker_partition.write().unwrap(),
                algo,
                total_vertices,
            );
            eprintln!("  [worker {worker_id}] job {job_id} started (algo={algo_str})");
        }

        let mut current_superstep = 0u64;
        let mut inbox = MessageInbox::new();

        'bsp: loop {
        // Allow cross-worker messages to arrive before draining. Superstep 0 has no prior msgs.
        let mut batch_count = 0usize;
        if current_superstep > 0 {
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(150);
            while tokio::time::Instant::now() < deadline {
                while let Ok(batch) = batch_rx.try_recv() {
                    batch_count += 1;
                    for msg in batch.messages {
                        inbox.add(msg.target, msg);
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        }
        while let Ok(batch) = batch_rx.try_recv() {
            batch_count += 1;
            for msg in batch.messages {
                inbox.add(msg.target, msg);
            }
        }
        let message_count = inbox.as_map().values().map(|v| v.len()).sum::<usize>();

        if verbose_level() >= 2 && current_superstep > 0 {
            observe().record(ObservableEvent::BatchesReceived {
                worker_id,
                superstep: current_superstep,
                batch_count,
                message_count,
            });
        }

        let start = Instant::now();
        observe().record(ObservableEvent::SuperstepStarted {
            worker_id,
            superstep: current_superstep,
        });

        if verbose_level() >= 2 {
            let mut inbox_items: Vec<(u64, Vec<String>)> = inbox
                .as_map()
                .iter()
                .map(|(target, msgs)| {
                    let payloads: Vec<String> = msgs
                        .iter()
                        .map(|m| format_payload(&m.payload, algo))
                        .collect();
                    (*target, payloads)
                })
                .collect();
            inbox_items.sort_by_key(|(v, _)| *v);
            observe().record(ObservableEvent::InboxSnapshot {
                worker_id,
                superstep: current_superstep,
                items: inbox_items,
            });

            let mut verts: Vec<(u64, String, Vec<u64>)> = worker_partition
                .read()
                .unwrap()
                .vertices
                .iter()
                .map(|(vid, v)| (*vid, format_value(&v.value, algo), v.edges.clone()))
                .collect();
            verts.sort_by_key(|(v, _, _)| *v);
            observe().record(ObservableEvent::VertexSnapshot {
                worker_id,
                superstep: current_superstep,
                vertices: verts,
            });
        }

        let partition_guard = worker_partition.read().unwrap().clone();
        let partition_for_blocking = Arc::new(partition_guard);
        let inbox_map = inbox.as_map().clone();
        let partition_impl_clone = Arc::clone(&partition_impl);
        let exec_clone = wasm_executor.clone();
        let mod_clone = wasm_module.clone();

        let (value_updates, outgoing) = tokio::task::spawn_blocking(move || {
            execute_superstep_parallel(
                &partition_for_blocking,
                &inbox_map,
                current_superstep,
                algo,
                exec_clone.as_deref(),
                mod_clone.as_deref(),
                partition_impl_clone.as_ref(),
                worker_count,
            )
        })
        .await?;

        for (vid, new_value) in value_updates {
            if let Some(v) = worker_partition.write().unwrap().vertices.get_mut(&vid) {
                v.value = new_value;
            }
        }

        let vertex_count = worker_partition.read().unwrap().vertices.len();
        observe().record(ObservableEvent::VerticesComputed {
            worker_id,
            count: vertex_count,
        });

        let duration_ms = start.elapsed().as_millis() as u64;
        observe().record(ObservableEvent::SuperstepCompleted {
            worker_id,
            superstep: current_superstep,
            duration_ms,
        });

        inbox.clear();

        let batches = worker.route_messages(outgoing);
        let mut total_msgs = 0usize;
        let mut total_bytes = 0usize;
        for (_target_worker, batch) in &batches {
            let count = batch.messages.len();
            let bytes: usize = batch.messages.iter().map(|m| m.payload.len() + 8).sum();
            total_msgs += count;
            total_bytes += bytes;
        }
        observe().record(ObservableEvent::MessagesSent {
            worker_id,
            count: total_msgs,
            bytes: total_bytes,
        });

        if verbose_level() >= 2 {
            let batch_snapshots: Vec<(u32, Vec<(u64, String)>)> = batches
                .iter()
                .map(|(tw, b)| {
                    let msgs: Vec<(u64, String)> = b
                        .messages
                        .iter()
                        .map(|m| (m.target, format_payload(&m.payload, algo)))
                        .collect();
                    (*tw, msgs)
                })
                .collect();
            observe().record(ObservableEvent::OutgoingSnapshot {
                worker_id,
                superstep: current_superstep,
                batches: batch_snapshots,
            });
        }

        tokio::task::yield_now().await;
        eprintln!("  [worker {worker_id}] DBG: send_loop step {current_superstep}");
        for (target_worker, batch) in batches {
            if target_worker == worker_id {
                // Bypass network for self-send: inject directly into local inbox channel.
                if let Err(e) = batch_tx_for_local.send(batch).await {
                    eprintln!("  [worker {worker_id}] WARN: local batch injection failed: {e}");
                }
                continue;
            }
            if let Some(&addr) = addresses.get(&target_worker) {
                let res = match transport {
                    TransportKind::Tcp => send_batch(addr, &batch).await,
                    TransportKind::Quic => quic_cache
                        .as_mut()
                        .expect("quic_cache set when transport=Quic")
                        .send_batch(addr, &batch)
                        .await,
                };
                if let Err(e) = res {
                    eprintln!("  [worker {worker_id}] WARN: send to worker {target_worker} ({addr}) failed: {e}");
                }
            }
            tokio::task::yield_now().await;
        }
        tokio::task::yield_now().await;
        eprintln!("  [worker {worker_id}] DBG: report step {current_superstep}");
        coordinator
            .report_superstep_done(worker_id, current_superstep, total_msgs as u64)
            .await?;
        eprintln!("  [worker {worker_id}] DBG: wait_advance step {current_superstep}");
        current_superstep = coordinator.wait_for_advance(current_superstep).await?;
        eprintln!("  [worker {worker_id}] DBG: advanced to {current_superstep}");

        if current_superstep == u64::MAX {
            eprintln!("  [worker {worker_id}] job complete (all halted)");
            if session {
                break 'bsp;
            } else {
                return Ok(());
            }
        }

        // Checkpoint every N supersteps
        if let Some(ref ckpt) = checkpoint_manager {
            if current_superstep > 0 && current_superstep % CHECKPOINT_INTERVAL == 0 {
                ckpt.save(worker_id, current_superstep - 1, &*worker_partition.read().unwrap())?;
                observe().record(ObservableEvent::CheckpointSaved {
                    worker_id,
                    superstep: current_superstep - 1,
                });
            }
        }
        } // end 'bsp loop

        if session {
            continue 'job_loop;
        } else {
            break;
        }
    }
    Ok(())
}
