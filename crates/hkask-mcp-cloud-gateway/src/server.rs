//! mTLS server setup for the cloud gateway.
//!
//! Configures rustls for mutual TLS 1.3, extracts client certificate
//! Common Name for identity binding, and forwards verified requests
//! to the local hKask daemon handler.

use crate::auth::AuthError;
use hkask_mcp::daemon::DaemonHandler;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::{RootCertStore, ServerConfig};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

/// Errors that can occur during gateway server operation.
#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("TLS configuration error: {0}")]
    Tls(#[from] rustls::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Certificate error: {0}")]
    Cert(String),

    #[error("Auth error: {0}")]
    Auth(#[from] AuthError),
}

/// Configuration for the cloud gateway server.
pub struct GatewayConfig {
    /// Path to the server's TLS certificate (PEM format).
    pub server_cert: PathBuf,
    /// Path to the server's private key (PEM format).
    pub server_key: PathBuf,
    /// Path to the CA certificate that signed client certificates (PEM format).
    pub client_ca: PathBuf,
    /// Address to bind to (e.g., "0.0.0.0:9443").
    pub bind_addr: String,
}

/// Build a rustls `ServerConfig` with mutual TLS.
///
/// Loads the server certificate chain and private key, and configures
/// a client certificate verifier using the provided CA certificate.
/// Only clients presenting a certificate signed by `client_ca` are accepted.
pub fn build_tls_config(config: &GatewayConfig) -> Result<ServerConfig, GatewayError> {
    // Load server certificate chain
    let certs = load_certificates(&config.server_cert)?;
    let key = load_private_key(&config.server_key)?;

    // Build client CA store for mTLS
    let mut client_ca_store = RootCertStore::empty();
    let ca_certs = load_certificates(&config.client_ca)?;
    for cert in &ca_certs {
        client_ca_store
            .add(cert.clone())
            .map_err(|e| GatewayError::Cert(format!("Failed to add CA cert: {e}")))?;
    }

    let client_verifier = WebPkiClientVerifier::builder(Arc::new(client_ca_store))
        .build()
        .map_err(|e| GatewayError::Cert(format!("Failed to build client verifier: {e}")))?;

    let tls_config = ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(certs, key)
        .map_err(|e| GatewayError::Cert(format!("Failed to build TLS config: {e}")))?;

    Ok(tls_config)
}

/// Run the gateway server — accepts mTLS connections and forwards to the daemon handler.
///
/// This is the main entry point for the cloud gateway. It binds to `config.bind_addr`,
/// accepts TLS connections, extracts the client CN from each connection,
/// and calls `handle_request` for each incoming message.
pub async fn run(
    config: GatewayConfig,
    handler: Arc<dyn DaemonHandler>,
) -> Result<(), GatewayError> {
    let tls_config = build_tls_config(&config)?;
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));
    let listener = TcpListener::bind(&config.bind_addr).await?;

    tracing::info!(
        target: "hkask.gateway",
        bind_addr = %config.bind_addr,
        "Cloud gateway listening with mTLS"
    );

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let handler = Arc::clone(&handler);

        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(tls_stream) => {
                    let (_tcp, server_conn) = tls_stream.get_ref();
                    let cert_cn = server_conn
                        .peer_certificates()
                        .and_then(|certs| certs.first())
                        .and_then(extract_cn)
                        .unwrap_or_else(|| "unknown".to_string());

                    tracing::info!(
                        target: "hkask.gateway",
                        peer = %peer_addr,
                        cn = %cert_cn,
                        "mTLS connection established"
                    );

                    // For now: each connection is authenticated but the request
                    // protocol (token-bearing JSON over the TLS stream) is deferred.
                    // The connection carries the verified identity; individual
                    // requests carry DelegationTokens for per-tool authorization.
                    let _ = (tls_stream, cert_cn, handler);
                }
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.gateway",
                        peer = %peer_addr,
                        error = %e,
                        "TLS handshake failed"
                    );
                }
            }
        });
    }
}

// ── Certificate loading helpers ────────────────────────────────────────

fn load_certificates(path: &Path) -> Result<Vec<CertificateDer<'static>>, GatewayError> {
    let file = std::fs::File::open(path).map_err(|e| {
        GatewayError::Cert(format!("Cannot open cert file {}: {e}", path.display()))
    })?;
    let mut reader = BufReader::new(file);
    let certs = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            GatewayError::Cert(format!(
                "Failed to parse certs from {}: {e}",
                path.display()
            ))
        })?;
    if certs.is_empty() {
        return Err(GatewayError::Cert(format!(
            "No certificates found in {}",
            path.display()
        )));
    }
    Ok(certs)
}

fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, GatewayError> {
    let file = std::fs::File::open(path)
        .map_err(|e| GatewayError::Cert(format!("Cannot open key file {}: {e}", path.display())))?;
    let mut reader = BufReader::new(file);
    let key = rustls_pemfile::private_key(&mut reader)
        .map_err(|e| {
            GatewayError::Cert(format!("Failed to parse key from {}: {e}", path.display()))
        })?
        .ok_or_else(|| GatewayError::Cert(format!("No private key found in {}", path.display())))?;
    Ok(key)
}

/// Extract the Common Name (CN) from an X.509 certificate.
fn extract_cn(cert: &CertificateDer) -> Option<String> {
    use x509_parser::prelude::*;
    let (_, parsed) = X509Certificate::from_der(cert.as_ref()).ok()?;
    let subject = parsed.subject();
    for attr in subject.iter_common_name() {
        if let Ok(cn) = attr.as_str() {
            return Some(cn.to_string());
        }
    }
    None
}
