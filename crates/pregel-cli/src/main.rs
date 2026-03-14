//! # pregel-cli
//!
//! Command-line interface for the Pregel distributed graph processing framework.

use clap::{Parser, Subcommand};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Parser)]
#[command(name = "pregel")]
#[command(about = "Distributed graph processing framework", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a session: load graph, launch workers, wait for jobs (metrics server runs for session duration)
    Session {
        /// Path to the graph (edge list format)
        #[arg(short, long)]
        graph: String,

        /// Number of workers
        #[arg(short, long, default_value = "2")]
        workers: usize,

        /// Host for binding (default: 127.0.0.1)
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Transport: tcp or quic
        #[arg(short, long, default_value = "tcp")]
        transport: String,

        /// Metrics port base (workers get base, base+1, ...)
        #[arg(long)]
        metrics_port: Option<u16>,

        /// Verbose level
        #[arg(short = 'v', long = "verbose", value_name = "LEVEL", num_args = 0..=1, default_value = "0", default_missing_value = "1")]
        verbose: u8,
    },

    /// Submit a job to an existing session
    Job {
        /// Session URL (e.g. http://127.0.0.1:5001)
        #[arg(short, long)]
        session: String,

        /// Algorithm: cc, pagerank, shortest_path
        #[arg(short, long, default_value = "cc")]
        algo: String,
    },

    /// Submit a Pregel job (single-shot: spawns coordinator + workers, runs one job, exits)
    Submit {
        /// Path to WASM module (optional; use native algo if omitted). Alias: --wasm
        #[arg(short, long, alias = "wasm")]
        program: Option<String>,

        /// Path to the graph (edge list format)
        #[arg(short, long)]
        graph: String,

        /// Number of workers
        #[arg(short, long, default_value = "2")]
        workers: usize,

        /// Host for binding (default: 127.0.0.1)
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Algorithm: pagerank, cc, shortest_path (or pr, sssp, sp)
        #[arg(short, long, default_value = "pagerank")]
        algo: String,

        /// Transport: tcp or quic
        #[arg(short, long, default_value = "tcp")]
        transport: String,

        /// Checkpoint directory (optional; enables periodic checkpointing)
        #[arg(long)]
        checkpoint_dir: Option<String>,

        /// Metrics port for Prometheus scraping (base port; workers use base, base+1, ...). Enables /metrics endpoint.
        #[arg(long)]
        metrics_port: Option<u16>,

        /// Verbose: -v/--verbose = summary, --verbose=2 = full dumps (messages, vertex states)
        #[arg(short = 'v', long = "verbose", value_name = "LEVEL", num_args = 0..=1, default_value = "0", default_missing_value = "1")]
        verbose: u8,
    },

    /// Build a Pregel program
    Build {
        #[arg(default_value = ".")]
        path: String,
    },

    /// Cluster management
    Cluster {
        #[command(subcommand)]
        action: Option<ClusterAction>,
    },

    /// Initialize a new Pregel project
    Init {
        name: String,
    },
}

#[derive(Subcommand)]
enum ClusterAction {
    Start,
    Stop,
    Status,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match cli.command {
        Commands::Session {
            graph,
            workers,
            host,
            transport,
            metrics_port,
            verbose,
        } => run_session(&graph, workers, &host, &transport, metrics_port, verbose.min(2)),
        Commands::Job { session, algo } => run_job(&session, &algo),
        Commands::Submit {
            program,
            graph,
            workers,
            host,
            algo,
            transport,
            checkpoint_dir,
            metrics_port,
            verbose,
        } => run_submit(&graph, workers, &host, &algo, &transport, program.as_deref(), checkpoint_dir.as_deref(), metrics_port, verbose.min(2)),
        Commands::Build { path } => {
            println!("Building from: {}", path);
            Ok(())
        }
        Commands::Cluster { action } => {
            match action {
                Some(ClusterAction::Start) => println!("Use: pregel submit --graph <path>"),
                Some(ClusterAction::Stop) => println!("Stop: kill coordinator and worker processes"),
                Some(ClusterAction::Status) => println!("Status: check if coordinator responds"),
                None => println!("Use: pregel cluster [start|stop|status]"),
            }
            Ok(())
        }
        Commands::Init { name } => {
            println!("Initializing project: {}", name);
            Ok(())
        }
    }
}

fn run_session(
    graph: &str,
    workers: usize,
    host: &str,
    transport: &str,
    metrics_port: Option<u16>,
    verbose: u8,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let graph_path = PathBuf::from(graph);
    if !graph_path.exists() {
        return Err(format!("Graph file not found: {}", graph).into());
    }

    let coordinator_port = 5000u16;
    let http_port = 5100u16; // HTTP API, separate from worker ports (5001, 5002, ...)
    let coordinator_addr = format!("{}:{}", host, coordinator_port);
    let session_url = format!("http://{}:{}", host, http_port);

    let bin_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let worker_bin = bin_dir.join("pregel-worker");
    let coordinator_bin = bin_dir.join("pregel-coordinator");

    if !worker_bin.exists() || !coordinator_bin.exists() {
        return Err(format!(
            "Binaries not found. Run: cargo build -p pregel-worker -p pregel-coordinator"
        )
        .into());
    }

    println!("Starting coordinator on {} (HTTP API on {})...", coordinator_addr, session_url);
    let workers_str = workers.to_string();
    let mut coord_args = vec![coordinator_addr.as_str(), workers_str.as_str(), "--http-port", "5100"];
    if verbose >= 2 {
        coord_args.push("--verbose=2");
    } else if verbose >= 1 {
        coord_args.push("--verbose");
    }
    let mut coordinator = std::process::Command::new(&coordinator_bin)
        .args(coord_args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    std::thread::sleep(std::time::Duration::from_millis(500));

    let mut worker_handles: Vec<(usize, Child)> = Vec::new();
    for i in 0..workers {
        let listen_port = coordinator_port + 1 + i as u16;
        println!("Starting worker {} on port {} (session mode, transport={})...", i, listen_port, transport);
        let mut args = vec![
            i.to_string(),
            coordinator_addr.clone(),
            graph_path.to_str().unwrap().to_string(),
            workers.to_string(),
            listen_port.to_string(),
            "--session".into(),
            "--transport".into(),
            transport.to_string(),
        ];
        if let Some(base) = metrics_port {
            args.push("--metrics-port".into());
            args.push((base + i as u16).to_string());
        }
        if verbose >= 2 {
            args.push("--verbose=2".into());
        } else if verbose >= 1 {
            args.push("--verbose".into());
        }
        let child = std::process::Command::new(&worker_bin)
            .args(&args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::piped())
            .spawn()?;
        worker_handles.push((i, child));
    }

    let stderr_lock: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    let mut reader_handles = Vec::<_>::new();
    for (_, child) in worker_handles.iter_mut() {
        if let Some(stderr) = child.stderr.take() {
            let lock = Arc::clone(&stderr_lock);
            reader_handles.push(thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(l) = line {
                        let _g = lock.lock().unwrap();
                        eprintln!("{l}");
                        let _ = std::io::Write::flush(&mut std::io::stderr());
                    }
                }
            }));
        }
    }

    println!("Session ready. Submit jobs with: cargo run -p pregel-cli -- job --session {} --algo cc", session_url);
    println!("Ctrl+C to stop.");
    ctrlc_handler();

    for (_, mut child) in worker_handles {
        let _ = child.kill();
    }
    for h in reader_handles {
        let _ = h.join();
    }
    let _ = coordinator.kill();
    Ok(())
}

fn run_job(session_url: &str, algo: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/jobs", session_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::new();
    let res = client
        .post(&url)
        .json(&serde_json::json!({ "algo": algo, "program": "" }))
        .send()?;
    if !res.status().is_success() {
        return Err(format!("Job submit failed: {} {}", res.status(), res.text()?).into());
    }
    let body: serde_json::Value = res.json()?;
    println!("Job submitted: {}", body);
    Ok(())
}

fn run_submit(
    graph: &str,
    workers: usize,
    host: &str,
    algo: &str,
    transport: &str,
    program: Option<&str>,
    checkpoint_dir: Option<&str>,
    metrics_port: Option<u16>,
    verbose: u8,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let graph_path = PathBuf::from(graph);
    if !graph_path.exists() {
        return Err(format!("Graph file not found: {}", graph).into());
    }

    let coordinator_port = 5000u16;
    let coordinator_addr = format!("{}:{}", host, coordinator_port);

    let bin_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let worker_bin = bin_dir.join("pregel-worker");
    let coordinator_bin = bin_dir.join("pregel-coordinator");

    if !worker_bin.exists() {
        return Err(format!(
            "pregel-worker not found at {}. Run: cargo build -p pregel-worker -p pregel-coordinator",
            worker_bin.display()
        )
        .into());
    }
    if !coordinator_bin.exists() {
        return Err(format!(
            "pregel-coordinator not found at {}. Run: cargo build -p pregel-worker -p pregel-coordinator",
            coordinator_bin.display()
        )
        .into());
    }

    println!("Starting coordinator on {}...", coordinator_addr);
    let workers_str = workers.to_string();
    let mut coord_args = vec![coordinator_addr.as_str(), workers_str.as_str()];
    if verbose >= 2 {
        coord_args.push("--verbose=2");
    } else if verbose >= 1 {
        coord_args.push("--verbose");
    }
    let mut coordinator = std::process::Command::new(&coordinator_bin)
        .args(coord_args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    std::thread::sleep(std::time::Duration::from_millis(500));

    let mut worker_handles: Vec<(usize, Child)> = Vec::new();
    for i in 0..workers {
        let listen_port = coordinator_port + 1 + i as u16;
        let prog_str = program.unwrap_or("native");
        println!("Starting worker {} on port {} (algo={}, program={}, transport={})...", i, listen_port, algo, prog_str, transport);
        let mut args = vec![
            i.to_string(),
            coordinator_addr.clone(),
            graph_path.to_str().unwrap().to_string(),
            workers.to_string(),
            listen_port.to_string(),
            "--algo".into(),
            algo.to_string(),
            "--transport".into(),
            transport.to_string(),
        ];
        if let Some(p) = program {
            args.push("--program".into());
            args.push(p.to_string());
        }
        if let Some(dir) = checkpoint_dir {
            args.push("--checkpoint-dir".into());
            args.push(dir.to_string());
        }
        if let Some(base) = metrics_port {
            let port = base + i as u16;
            args.push("--metrics-port".into());
            args.push(port.to_string());
        }
        if verbose >= 2 {
            args.push("--verbose=2".into());
        } else if verbose >= 1 {
            args.push("--verbose".into());
        }
        let child = std::process::Command::new(&worker_bin)
            .args(&args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::piped())
            .spawn()?;
        worker_handles.push((i, child));
    }

    // Serialize worker stderr through CLI so output isn't garbled.
    // Print immediately upon read so we don't block workers' stderr pipes.
    let stderr_lock: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    let mut reader_handles = Vec::<_>::new();
    for (_, child) in worker_handles.iter_mut() {
        if let Some(stderr) = child.stderr.take() {
            let lock = Arc::clone(&stderr_lock);
            reader_handles.push(thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(l) = line {
                        let _g = lock.lock().unwrap();
                        eprintln!("{l}");
                        let _ = std::io::Write::flush(&mut std::io::stderr());
                    }
                }
            }));
        }
    }

    println!("Job submitted. Coordinator and {} workers running. Ctrl+C to stop.", workers);
    ctrlc_handler();

    for (_, mut child) in worker_handles {
        let _ = child.kill();
    }
    for h in reader_handles {
        let _ = h.join();
    }
    let _ = coordinator.kill();
    Ok(())
}

fn ctrlc_handler() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
    while running.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
