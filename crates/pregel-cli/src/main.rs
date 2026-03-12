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
    /// Submit a Pregel job (spawns coordinator + workers)
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
        Commands::Submit {
            program,
            graph,
            workers,
            host,
            algo,
            transport,
            checkpoint_dir,
            verbose,
        } => run_submit(&graph, workers, &host, &algo, &transport, program.as_deref(), checkpoint_dir.as_deref(), verbose.min(2)),
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

fn run_submit(
    graph: &str,
    workers: usize,
    host: &str,
    algo: &str,
    transport: &str,
    program: Option<&str>,
    checkpoint_dir: Option<&str>,
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
