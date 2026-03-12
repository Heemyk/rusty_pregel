//! QUIC transport for worker-to-worker message passing.
//!
//! Each worker generates its own self-signed cert for its server. Cross-worker
//! clients use SkipServerVerification so they can connect to any worker's server
//! (dev/testing only; not for production).
//!
//! Connection reuse: we cache one outgoing connection per peer to avoid per-batch
//! handshakes, which were causing messages to arrive after the drain window.

use pregel_common::{PregelError, Result};
use pregel_messaging::MessageBatch;
use quinn::{ClientConfig, Connection, Endpoint, ServerConfig};
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Dummy verifier that accepts any server cert. Only for dev/testing.
#[derive(Debug)]
struct SkipServerVerification(Arc<rustls::crypto::CryptoProvider>);

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self(Arc::new(rustls::crypto::ring::default_provider())))
    }
}

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

fn make_server_config() -> Result<ServerConfig> {
    let cert = generate_simple_self_signed(["localhost".into()])
        .map_err(|e| PregelError::Network(e.to_string()))?;
    let cert_der = CertificateDer::from(cert.cert.der().as_ref().to_vec());
    let key_der = cert.key_pair.serialize_der();
    let key = PrivateKeyDer::try_from(key_der).map_err(|e| PregelError::Network(e.to_string()))?;

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut server_crypto = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| PregelError::Network(e.to_string()))?
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key)
        .map_err(|e| PregelError::Network(e.to_string()))?;

    server_crypto.alpn_protocols = vec![b"pregel".to_vec()];

    let config = quinn::ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)
            .map_err(|e| PregelError::Network(e.to_string()))?,
    ));

    Ok(config)
}

fn make_client_config() -> Result<ClientConfig> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut crypto = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| PregelError::Network(e.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(SkipServerVerification::new())
        .with_no_client_auth();

    crypto.alpn_protocols = vec![b"pregel".to_vec()];

    let config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(crypto)
            .map_err(|e| PregelError::Network(e.to_string()))?,
    ));

    Ok(config)
}

/// Create QUIC server endpoint for receiving message batches.
/// Also configures it for outgoing connections (connect).
pub fn quic_server(addr: SocketAddr) -> Result<Endpoint> {
    let server_config = make_server_config()?;
    let mut endpoint =
        Endpoint::server(server_config, addr).map_err(|e| PregelError::Network(e.to_string()))?;
    endpoint.set_default_client_config(make_client_config()?);
    Ok(endpoint)
}

/// Sender that reuses QUIC connections per peer to avoid per-batch handshakes.
pub struct QuicConnectionCache {
    endpoint: Endpoint,
    connections: HashMap<SocketAddr, Connection>,
}

impl QuicConnectionCache {
    pub fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint,
            connections: HashMap::new(),
        }
    }

    /// Send a batch, reusing cached connection to addr or creating one.
    pub async fn send_batch(&mut self, addr: SocketAddr, batch: &MessageBatch) -> Result<()> {
        let batch = batch.clone();
        let result = self.send_batch_inner(addr, batch).await;
        if result.is_err() {
            self.connections.remove(&addr);
        }
        result
    }

    async fn send_batch_inner(&mut self, addr: SocketAddr, batch: MessageBatch) -> Result<()> {
        let conn = if let Some(c) = self.connections.get_mut(&addr) {
            c.clone()
        } else {
            let connecting = self
                .endpoint
                .connect(addr, "localhost")
                .map_err(|e| PregelError::Network(e.to_string()))?;
            let conn = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                connecting,
            )
            .await
            .map_err(|_| PregelError::Network("QUIC connect timeout (5s)".into()))?
            .map_err(|e| PregelError::Network(e.to_string()))?;
            self.connections.insert(addr, conn.clone());
            conn
        };

        let send_fut = async {
            let (mut send, _recv) = conn
                .open_bi()
                .await
                .map_err(|e| PregelError::Network(e.to_string()))?;

            let bytes = bincode::serialize(&batch).map_err(|e| PregelError::Serialization(e.to_string()))?;
            send.write_u32_le(bytes.len() as u32)
                .await
                .map_err(|e| PregelError::Network(e.to_string()))?;
            send.write_all(&bytes)
                .await
                .map_err(|e| PregelError::Network(e.to_string()))?;
            send.finish().map_err(|e| PregelError::Network(e.to_string()))?;
            Ok::<(), PregelError>(())
        };

        tokio::time::timeout(std::time::Duration::from_secs(5), send_fut)
            .await
            .map_err(|_| PregelError::Network("quic send_batch timeout (5s)".into()))?
    }
}

/// Run QUIC receiver: accept connections, read length-prefixed batches, send to channel.
pub async fn quic_run_receiver(
    endpoint: quinn::Endpoint,
    tx: tokio::sync::mpsc::Sender<MessageBatch>,
) {
    loop {
        let Some(incoming) = endpoint.accept().await else { break };
        let Ok(conn) = incoming.await else { continue };
        let tx = tx.clone();
        tokio::spawn(async move {
            loop {
                let Ok((_send, mut recv)) = conn.accept_bi().await else { break };
                if let Ok(len) = recv.read_u32_le().await {
                    let mut buf = vec![0u8; len as usize];
                    if recv.read_exact(&mut buf).await.is_ok() {
                        if let Ok(batch) = bincode::deserialize::<MessageBatch>(&buf) {
                            let _ = tx.send(batch).await;
                        }
                    }
                }
            }
        });
    }
}
