# TCP vs QUIC: Why CC Behaves Differently

## Summary

**TCP** converges in 3 supersteps; **QUIC** takes 5–6. The difference comes from **when cross-worker message batches arrive**, not from the CC algorithm itself.

## Evidence from Traces

### Superstep 1 drain

| Transport | Worker 0 | Worker 1 |
|-----------|----------|----------|
| **TCP**   | 2 batches, 5 msgs (v0, v2, v4) | 2 batches, 4 msgs (v1, v3) |
| **QUIC**  | 1 batch, 3 msgs (v0, v2 only) | 1 batch, 1 msg (v3 only) |

On TCP both workers receive **2 batches** (local + remote). On QUIC they receive **1 batch** (only the local one).

### What’s in each batch?

- **Local batch**: Messages a worker sends to itself (`batch_tx_for_local`).
- **Remote batch**: Messages sent by the other worker over the network.

Worker 1’s step‑1 drain with QUIC: only `v3 ← [1]`, which is from its own step‑0 self‑send. Worker 0’s batch (v1 ← [0,4], v3 ← [1,2]) does not arrive during the drain.

## Root Cause: Connection Setup Cost

### TCP (`transport.rs`)

```rust
let mut stream = TcpStream::connect(addr).await?;
stream.write_u32_le(...).await?;
stream.write_all(&bytes).await?;
stream.flush().await?;
```

- One TCP connection per batch.
- Connect is a simple 3‑way handshake, very fast on loopback.
- Data is sent immediately after connect.

### QUIC (`transport_quic.rs`)

```rust
// Every quic_send_batch creates a NEW client endpoint and connection:
let mut endpoint = Endpoint::client(...)?;
let conn = endpoint.connect(addr, "localhost")?.await?;  // Full QUIC+TLS handshake
let (mut send, _recv) = conn.open_bi().await?;            // Open stream
send.write_u32_le(...).await?;
send.write_all(&bytes).await?;
send.finish()?;
```

- New `Endpoint::client` and new connection per batch.
- Full QUIC + TLS handshake each time.
- Handshake takes longer than TCP, especially under load.

## Drain Window

```rust
// main.rs ~line 202
let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(150);
while tokio::time::Instant::now() < deadline {
    while let Ok(batch) = batch_rx.try_recv() { ... }
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
}
```

We wait up to **150 ms** for batches to arrive before compute.

- **TCP**: Remote batches almost always arrive within this window.
- **QUIC**: Handshake + send can exceed 150 ms, so the remote batch may arrive after we stop draining.
- Late batches are consumed in the next superstep’s drain, so they’re applied in the wrong superstep and we need extra iterations to converge.

## Why QUIC Still Converges

Delayed messages are not dropped. They show up in the next superstep’s drain. Eventually all messages are delivered, so the CC algorithm converges, but it does more supersteps. Final state is the same (e.g. all vertices in component 0).

## Fix Applied: Connection Reuse

We implemented **QUIC connection reuse** via `QuicConnectionCache` in `transport_quic.rs`:

- One shared `Endpoint` (configured for both server and client) per worker
- `HashMap<SocketAddr, Connection>` caches one connection per peer
- Batches to the same worker reuse the cached connection; no handshake after the first
- On send failure (e.g. closed connection), we remove the cached entry and reconnect on next send

After this change, QUIC converges in **3 supersteps** like TCP, with both workers receiving **2 batches** in superstep 1.
