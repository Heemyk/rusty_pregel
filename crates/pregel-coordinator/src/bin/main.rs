//! Pregel coordinator binary.

use pregel_coordinator::grpc;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    let addr: SocketAddr = if args.len() >= 2 {
        args[1].parse()?
    } else {
        "0.0.0.0:5000".parse()?
    };
    let worker_count: usize = if args.len() >= 3 {
        args[2].parse()?
    } else {
        4
    };
    let verbose = args.iter().any(|a| a == "--verbose" || a == "-v");

    let mut http_port = None;
    let mut worker_timeout_secs = 60u64;
    let mut i = 3;
    while i < args.len() {
        if args[i] == "--http-port" && i + 1 < args.len() {
            http_port = Some(args[i + 1].parse()?);
            i += 2;
        } else if args[i] == "--worker-timeout" && i + 1 < args.len() {
            worker_timeout_secs = args[i + 1].parse()?;
            i += 2;
        } else {
            i += 1;
        }
    }

    println!("Coordinator listening on {} (expecting {} workers, worker timeout {}s)", addr, worker_count, worker_timeout_secs);
    grpc::run_coordinator_server(addr, worker_count, verbose, http_port, worker_timeout_secs).await
}
