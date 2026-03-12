//! Network transport for worker-to-worker message passing.
//!
//! Uses TCP with length-prefixed bincode. QUIC can be added later.

use pregel_common::{PregelError, Result};
use pregel_messaging::MessageBatch;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Send a MessageBatch to a worker. Serializes with bincode, length-prefixed.
pub async fn send_batch(addr: SocketAddr, batch: &MessageBatch) -> Result<()> {
    let mut stream = TcpStream::connect(addr).await.map_err(|e| PregelError::Network(e.to_string()))?;
    let bytes = bincode::serialize(batch).map_err(|e| pregel_common::PregelError::Serialization(e.to_string()))?;
    stream.write_u32_le(bytes.len() as u32).await?;
    stream.write_all(&bytes).await?;
    stream.flush().await?;
    Ok(())
}

/// Receive MessageBatches. Runs a TCP server; each connection sends one length-prefixed batch.
pub async fn run_receiver(listener: TcpListener, tx: tokio::sync::mpsc::Sender<MessageBatch>) {
    loop {
        let Ok((mut stream, _)) = listener.accept().await else { continue };
        let tx = tx.clone();
        tokio::spawn(async move {
            if let Ok(len) = stream.read_u32_le().await {
                let mut buf = vec![0u8; len as usize];
                if stream.read_exact(&mut buf).await.is_ok() {
                    if let Ok(batch) = bincode::deserialize::<MessageBatch>(&buf) {
                        let _ = tx.send(batch).await;
                    }
                }
            }
        });
    }
}


/// Resolve worker addresses. For now, assume workers are at base_addr + worker_id.
pub fn worker_addresses(base_host: &str, base_port: u16, worker_count: usize) -> HashMap<u32, SocketAddr> {
    let mut m = HashMap::new();
    for i in 0..worker_count {
        let port = base_port + 1 + i as u16; // coordinator at base_port, workers at base_port+1, +2, ...
        m.insert(i as u32, format!("{}:{}", base_host, port).parse().unwrap());
    }
    m
}
