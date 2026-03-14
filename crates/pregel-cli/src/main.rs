//! # pregel-cli
//!
//! Command-line interface for the Pregel distributed graph processing framework.
//!
//! Run from the `pregel/` directory. See `docs/CLI_REFERENCE.md` for full reference.

use clap::{Parser, Subcommand};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Parser)]
#[command(name = "pregel")]
#[command(about = "Distributed graph processing framework")]
#[command(long_about = "Pregel CLI: run graph algorithms (CC, PageRank, SSSP) in single-shot or session mode.\n\nRun from pregel/ directory. Use `pregel <command> --help` for per-command options.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Algorithm: cc|connected_components, pagerank|pr, shortest_path|sssp|sp
const ALGO_HELP: &str = "Algorithm: cc, pagerank, shortest_path (aliases: pr, sssp, sp)";

/// Transport for inter-worker messaging: tcp or quic
const TRANSPORT_HELP: &str = "Transport for inter-worker messaging: tcp (default) or quic";

/// Verbose: -v = summary, --verbose=2 = full dumps
const VERBOSE_HELP: &str = "Verbosity: 0=quiet, 1=summary (-v), 2=full dumps (messages, vertex states)";

#[derive(Subcommand)]
enum Commands {
    /// Start a session: load graph, launch coordinator+workers, wait for job submissions
    #[command(long_about = "Start coordinator and workers. Graph is loaded and partitioned. Workers stay alive between jobs. Submit jobs with `pregel job --session <url>`.")]
    Session {
        /// Path to graph file (edge list: "src dst" per line)
        #[arg(short, long, value_name = "PATH")]
        graph: String,

        /// Number of workers
        #[arg(short, long, default_value = "2", value_name = "N")]
        workers: usize,

        /// Host for binding
        #[arg(long, default_value = "127.0.0.1", value_name = "HOST")]
        host: String,

        /// Coordinator gRPC port
        #[arg(long, default_value = "5000", value_name = "PORT")]
        coordinator_port: u16,

        /// Coordinator HTTP API port (for job submission)
        #[arg(long, default_value = "5100", value_name = "PORT")]
        http_port: u16,

        #[arg(short, long, default_value = "tcp", value_name = "tcp|quic", help = TRANSPORT_HELP)]
        transport: String,

        /// Dir for periodic checkpoints (workers save every 10 supersteps)
        #[arg(long, value_name = "PATH")]
        checkpoint_dir: Option<String>,

        /// Metrics port base (workers get base, base+1, ...); enables /metrics endpoint
        #[arg(long, value_name = "PORT")]
        metrics_port: Option<u16>,

        /// Worker report timeout in seconds (coordinator aborts job if worker doesn't report)
        #[arg(long, default_value = "60", value_name = "SECS")]
        worker_timeout: u64,

        #[arg(short = 'v', long, value_name = "0|1|2", num_args = 0..=1, default_value = "0", default_missing_value = "1", help = VERBOSE_HELP)]
        verbose: u8,
    },

    /// Submit a job to an existing session
    #[command(long_about = "POST to session HTTP API. Session must be running (from `pregel session`).")]
    Job {
        /// Session HTTP URL (e.g. http://127.0.0.1:5100)
        #[arg(short, long, value_name = "URL")]
        session: String,

        /// Algorithm to run
        #[arg(short, long, default_value = "cc", value_name = "ALGO", help = ALGO_HELP)]
        algo: String,

        /// Path to WASM module (optional; native algo if omitted)
        #[arg(short, long, alias = "wasm", value_name = "PATH")]
        program: Option<String>,

        /// Wait for job completion and return result (blocking). Without this, returns job_id immediately (fire-and-forget).
        #[arg(long, default_value_t = false)]
        await_result: bool,
    },

    /// Single-shot: spawn coordinator+workers, run one job, exit
    #[command(long_about = "Start coordinator and workers, run one algorithm, then exit. No session persistence.")]
    Submit {
        /// Path to WASM module (optional; native algo if omitted)
        #[arg(short, long, alias = "wasm", value_name = "PATH")]
        program: Option<String>,

        /// Path to graph file (edge list format)
        #[arg(short, long, value_name = "PATH")]
        graph: String,

        /// Number of workers
        #[arg(short, long, default_value = "2", value_name = "N")]
        workers: usize,

        /// Host for binding
        #[arg(long, default_value = "127.0.0.1", value_name = "HOST")]
        host: String,

        /// Coordinator gRPC port
        #[arg(long, default_value = "5000", value_name = "PORT")]
        coordinator_port: u16,

        /// Algorithm to run
        #[arg(short, long, default_value = "pagerank", value_name = "ALGO", help = ALGO_HELP)]
        algo: String,

        #[arg(short, long, default_value = "tcp", value_name = "tcp|quic", help = TRANSPORT_HELP)]
        transport: String,

        /// Dir for periodic checkpoints
        #[arg(long, value_name = "PATH")]
        checkpoint_dir: Option<String>,

        /// Metrics port base; enables /metrics
        #[arg(long, value_name = "PORT")]
        metrics_port: Option<u16>,

        /// Worker report timeout (coordinator)
        #[arg(long, default_value = "60", value_name = "SECS")]
        worker_timeout: u64,

        /// Wait for job completion and print result (default: true for submit). Use --no-await-result for fire-and-forget style.
        #[arg(long, default_value = "true", default_missing_value = "true")]
        await_result: bool,

        #[arg(short = 'v', long, value_name = "0|1|2", num_args = 0..=1, default_value = "0", default_missing_value = "1", help = VERBOSE_HELP)]
        verbose: u8,
    },

    /// Build a Pregel program (WASM or native)
    #[command(long_about = "Build from project directory. Produces .wasm for use with --program.")]
    Build {
        #[arg(default_value = ".", value_name = "PATH")]
        path: String,
    },

    /// Cluster management (start, stop, status)
    Cluster {
        #[command(subcommand)]
        action: Option<ClusterAction>,
    },

    /// Initialize a new Pregel project scaffold
    Init {
        #[arg(value_name = "NAME")]
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
            coordinator_port,
            http_port,
            transport,
            checkpoint_dir,
            metrics_port,
            worker_timeout,
            verbose,
        } => run_session(
            &graph,
            workers,
            &host,
            coordinator_port,
            http_port,
            &transport,
            checkpoint_dir.as_deref(),
            metrics_port,
            worker_timeout,
            verbose.min(2),
        ),
        Commands::Job {
            session,
            algo,
            program,
            await_result,
        } => run_job(&session, &algo, program.as_deref(), await_result),
        Commands::Submit {
            program,
            graph,
            workers,
            host,
            coordinator_port,
            algo,
            transport,
            checkpoint_dir,
            metrics_port,
            worker_timeout,
            await_result,
            verbose,
        } => run_submit(
            &graph,
            workers,
            &host,
            coordinator_port,
            &algo,
            &transport,
            program.as_deref(),
            checkpoint_dir.as_deref(),
            metrics_port,
            worker_timeout,
            await_result,
            verbose.min(2),
        ),
        Commands::Build { path } => run_build(&path),
        Commands::Cluster { action } => run_cluster(action.as_ref()),
        Commands::Init { name } => run_init(&name),
    }
}

fn run_session(
    graph: &str,
    workers: usize,
    host: &str,
    coordinator_port: u16,
    http_port: u16,
    transport: &str,
    checkpoint_dir: Option<&str>,
    metrics_port: Option<u16>,
    worker_timeout: u64,
    verbose: u8,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let graph_path = PathBuf::from(graph);
    if !graph_path.exists() {
        return Err(format!("Graph file not found: {}", graph).into());
    }

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

    println!(
        "Starting coordinator on {} (HTTP API on {})...",
        coordinator_addr, session_url
    );
    let workers_str = workers.to_string();
    let http_port_str = http_port.to_string();
    let worker_timeout_str = worker_timeout.to_string();
    let mut coord_args = vec![
        coordinator_addr.as_str(),
        workers_str.as_str(),
        "--http-port",
        &http_port_str,
        "--worker-timeout",
        &worker_timeout_str,
    ];
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
        println!(
            "Starting worker {} on port {} (session mode, transport={})...",
            i, listen_port, transport
        );
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
        if let Some(dir) = checkpoint_dir {
            args.push("--checkpoint-dir".into());
            args.push(dir.to_string());
        }
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

    println!(
        "Session ready. Submit jobs with: cargo run -p pregel-cli -- job --session {} --algo cc",
        session_url
    );
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

fn run_job(
    session_url: &str,
    algo: &str,
    program: Option<&str>,
    await_result: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/jobs", session_url.trim_end_matches('/'));
    let program_str = program.unwrap_or("");
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()?;
    let res = client
        .post(&url)
        .json(&serde_json::json!({
            "algo": algo,
            "program": program_str,
            "await": await_result,
        }))
        .send()?;
    if !res.status().is_success() {
        return Err(format!("Job submit failed: {} {}", res.status(), res.text()?).into());
    }
    let body: serde_json::Value = res.json()?;
    if let Some(result) = body.get("result") {
        println!("Result: {}", serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string()));
    } else {
        println!("{}", serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()));
    }
    Ok(())
}

fn run_submit(
    graph: &str,
    workers: usize,
    host: &str,
    coordinator_port: u16,
    algo: &str,
    transport: &str,
    program: Option<&str>,
    checkpoint_dir: Option<&str>,
    metrics_port: Option<u16>,
    worker_timeout: u64,
    await_result: bool,
    verbose: u8,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let graph_path = PathBuf::from(graph);
    if !graph_path.exists() {
        return Err(format!("Graph file not found: {}", graph).into());
    }

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
    let worker_timeout_str = worker_timeout.to_string();
    let mut coord_args = vec![
        coordinator_addr.as_str(),
        workers_str.as_str(),
        "--http-port", "5100",
        "--worker-timeout", &worker_timeout_str,
    ];
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

    // Wait for workers to connect and register before POSTing job
    std::thread::sleep(std::time::Duration::from_millis(1500));

    let session_url = format!("http://{}:5100", host);
    let mut worker_handles: Vec<(usize, Child)> = Vec::new();
    for i in 0..workers {
        let listen_port = coordinator_port + 1 + i as u16;
        let prog_str = program.unwrap_or("native");
        println!("Starting worker {} on port {} (session, algo={}, program={}, transport={})...", i, listen_port, algo, prog_str, transport);
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

    let url = format!("{}/jobs", session_url.trim_end_matches('/'));
    let program_str = program.unwrap_or("");
    println!("Submitting job (algo={})...", algo);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()?;
    // Submit always awaits (single-shot: we need job to complete before killing processes)
    let res = client
        .post(&url)
        .json(&serde_json::json!({
            "algo": algo,
            "program": program_str,
            "await": true,
        }))
        .send()?;
    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().unwrap_or_default();
        for (_, mut child) in worker_handles {
            let _ = child.kill();
        }
        let _ = coordinator.kill();
        return Err(format!("Job failed: {} {}", status, text).into());
    }
    let body: serde_json::Value = res.json()?;
    if await_result {
        if let Some(result) = body.get("result") {
            println!("Result: {}", serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string()));
        } else {
            println!("{}", serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()));
        }
    } else {
        let job_id = body.get("job_id").and_then(|v| v.as_u64()).unwrap_or(0);
        println!("Job completed (job_id={})", job_id);
    }

    for (_, mut child) in worker_handles {
        let _ = child.kill();
    }
    for h in reader_handles {
        let _ = h.join();
    }
    let _ = coordinator.kill();
    Ok(())
}

fn run_build(path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = PathBuf::from(path);
    let manifest = path.join("Cargo.toml");
    if !manifest.exists() {
        return Err(format!(
            "No Cargo.toml at {}. Run from a crate that uses pregel-sdk and exports compute.",
            path.display()
        ).into());
    }
    let status = std::process::Command::new("cargo")
        .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
        .current_dir(path.canonicalize()?)
        .status()?;
    if !status.success() {
        return Err("WASM build failed".into());
    }
    let crate_name = std::fs::read_to_string(&manifest)?
        .lines()
        .find(|l| l.trim_start().starts_with("name = "))
        .and_then(|l| l.split('"').nth(1))
        .unwrap_or("unknown")
        .replace('-', "_");
    let wasm = path
        .join("target/wasm32-unknown-unknown/release")
        .join(format!("lib{}.wasm", crate_name));
    if wasm.exists() {
        println!("Built: {}", wasm.display());
        println!("Run with: pregel submit --graph <path> --program {} --algo cc", wasm.display());
    } else {
        println!("Build finished. Look for lib*.wasm in target/wasm32-unknown-unknown/release/");
    }
    Ok(())
}

fn run_cluster(action: Option<&ClusterAction>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match action {
        Some(ClusterAction::Start) => println!("Start: pregel session --graph <path> --workers N"),
        Some(ClusterAction::Stop) => println!("Stop: Ctrl+C on session terminal"),
        Some(ClusterAction::Status) => println!("Status: check if coordinator responds on port 5000"),
        None => println!("Usage: pregel cluster [start|stop|status]"),
    }
    Ok(())
}

fn run_init(name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Init: creating project scaffold for '{}'", name);
    let dir = PathBuf::from(name);
    if dir.exists() {
        return Err(format!("Directory {} already exists", name).into());
    }
    std::fs::create_dir_all(&dir)?;
    std::fs::create_dir_all(dir.join("src"))?;
    let toml = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
pregel-sdk = {{ path = "../crates/pregel-sdk" }}
pregel-common = {{ path = "../crates/pregel-common" }}
bincode = "1.3"

[target.'cfg(target_arch = "wasm32")'.dependencies]
"#, name.replace('-', "_"));
    std::fs::write(dir.join("Cargo.toml"), toml)?;
    let lib_rs = r#"//! Vertex compute module. Build: cargo build --target wasm32-unknown-unknown --release
use pregel_sdk::{Context, Vertex, VertexProgram};

struct MyAlgo;
impl Default for MyAlgo { fn default() -> Self { Self } }
impl VertexProgram for MyAlgo {
    type VertexValue = u64;
    type Message = u64;
    fn compute(&mut self, v: &mut Vertex<u64>, msgs: &[(u64, u64)], ctx: &mut Context<u64>) {
        let min = msgs.iter().map(|(_, m)| *m).min().unwrap_or(v.value);
        v.value = v.value.min(min);
        for &t in &v.edges { ctx.send(t, v.value); }
    }
}
pregel_sdk::export_wasm_compute!(MyAlgo);
"#;
    std::fs::write(dir.join("src/lib.rs"), lib_rs)?;
    println!("Created {}/ with Cargo.toml and src/lib.rs. Add to workspace and run: pregel build {}", name, name);
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
