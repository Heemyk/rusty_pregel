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

    println!("Coordinator listening on {} (expecting {} workers)", addr, worker_count);
    grpc::run_coordinator_server(addr, worker_count, verbose).await
}
