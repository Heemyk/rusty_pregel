//! # pregel-cli
//!
//! Command-line interface for the Pregel distributed graph processing framework.
//!
//! Commands: `submit`, `build`, `cluster`, `init`.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pregel")]
#[command(about = "Distributed graph processing framework", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Submit a Pregel job
    Submit {
        /// Path to the WASM program
        #[arg(short, long)]
        program: String,

        /// Path to the graph (e.g. s3://graphs/webgraph)
        #[arg(short, long)]
        graph: String,

        /// Number of workers
        #[arg(short, long, default_value = "8")]
        workers: usize,
    },

    /// Build a Pregel program
    Build {
        /// Source directory
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
        /// Project name (e.g. pagerank)
        name: String,
    },
}

#[derive(Subcommand)]
enum ClusterAction {
    /// Start a local cluster
    Start,

    /// Stop the cluster
    Stop,

    /// Show cluster status
    Status,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Submit { program, graph, workers } => {
            println!("Submitting job: program={}, graph={}, workers={}", program, graph, workers);
        }
        Commands::Build { path } => {
            println!("Building from: {}", path);
        }
        Commands::Cluster { action } => {
            match action {
                Some(ClusterAction::Start) => println!("Starting cluster..."),
                Some(ClusterAction::Stop) => println!("Stopping cluster..."),
                Some(ClusterAction::Status) => println!("Cluster status..."),
                None => println!("Use: pregel cluster [start|stop|status]"),
            }
        }
        Commands::Init { name } => {
            println!("Initializing project: {}", name);
        }
    }
}
