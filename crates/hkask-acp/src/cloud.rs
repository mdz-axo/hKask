//! Cloud transport client for hkask-acp and other MCP clients.
//!
//! Provides an mTLS + DelegationToken transport layer for connecting
//! to a remote hKask cloud gateway instead of the local Unix socket daemon.
//!
//! # Usage
//!
//! ```text
//! HKASK_CLOUD_GATEWAY=hkask.example.com:9443
//! HKASK_CLIENT_CERT=/path/to/alice.crt
//! HKASK_CLIENT_KEY=/path/to/alice.key
//! HKASK_SERVER_CA=/path/to/ca.crt
//! HKASK_DELEGATION_TOKEN=eyJ...
//! hkask-acp
//! ```
//!
//! When `HKASK_CLOUD_GATEWAY` is set, the ACP agent connects via mTLS
//! instead of the local Unix socket. Discovery is automatic at startup.

use hkask_capability::DelegationToken;
use rustls::ClientConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::BufReader;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as AsyncBufReader};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

/// A request sent to the cloud gateway.
#[derive(Debug, Serialize)]
struct CloudRequest {
    tool: String,
    #[serde(skip_serializing_if = "Value::is_null")]
    params: Value,
    token: DelegationToken,
}

/// A response from the cloud gateway.
#[derive(Debug, Deserialize)]
struct CloudResponse {
    ok: bool,
    output: Option<Value>,
    error: Option<String>,
}

/// Client for connecting to a remote hKask cloud gateway via mTLS.
///
/// Replaces `DaemonClient` (Unix socket) for cloud deployments.
/// Uses the same DelegationToken protocol for per-request authorization.
pub struct CloudClient {
    gateway_addr: String,
    tls_config: Arc<ClientConfig>,
    token: DelegationToken,
    server_name: ServerName<'static>,
}

impl CloudClient {
    /// Build a CloudClient from environment variables.
    ///
    /// Reads:
    /// - `HKASK_CLOUD_GATEWAY` — host:port of the cloud gateway
    /// - `HKASK_CLIENT_CERT` — path to client TLS certificate (PEM)
    /// - `HKASK_CLIENT_KEY` — path to client private key (PEM)
    /// - `HKASK_SERVER_CA` — path to server CA certificate (PEM)
    /// - `HKASK_DELEGATION_TOKEN` — JSON DelegationToken string
    pub fn from_env() -> Result<Option<Self>, String> {
        let gateway_addr = match std::env::var("HKASK_CLOUD_GATEWAY") {
            Ok(a) if !a.is_empty() => a,
            _ => return Ok(None),
        };
        let host = gateway_addr
            .split(':')
            .next()
            .unwrap_or(&gateway_addr)
            .to_string();
        let server_name =
            ServerName::try_from(host).map_err(|e| format!("Invalid gateway hostname: {e}"))?;

        let client_cert = load_certs(
            &std::env::var("HKASK_CLIENT_CERT")
                .map_err(|_| "HKASK_CLIENT_CERT not set".to_string())?,
        )?;
        let client_key = load_key(
            &std::env::var("HKASK_CLIENT_KEY")
                .map_err(|_| "HKASK_CLIENT_KEY not set".to_string())?,
        )?;
        let mut ca_store = rustls::RootCertStore::empty();
        let ca_certs = load_certs(
            &std::env::var("HKASK_SERVER_CA").map_err(|_| "HKASK_SERVER_CA not set".to_string())?,
        )?;
        for cert in ca_certs {
            ca_store
                .add(cert)
                .map_err(|e| format!("Bad CA cert: {e}"))?;
        }

        let tls_config = ClientConfig::builder()
            .with_root_certificates(ca_store)
            .with_client_auth_cert(client_cert, client_key)
            .map_err(|e| format!("TLS config error: {e}"))?;

        let token_json = std::env::var("HKASK_DELEGATION_TOKEN")
            .map_err(|_| "HKASK_DELEGATION_TOKEN not set".to_string())?;
        let token: DelegationToken =
            serde_json::from_str(&token_json).map_err(|e| format!("Invalid token JSON: {e}"))?;

        Ok(Some(Self {
            gateway_addr,
            tls_config: Arc::new(tls_config),
            token,
            server_name,
        }))
    }

    /// Dispatch a tool call to the cloud gateway.
    pub async fn dispatch_tool(
        &self,
        tool: &str,
        params: &Value,
    ) -> Result<(bool, Option<Value>, Option<String>), String> {
        let connector = TlsConnector::from(Arc::clone(&self.tls_config));
        let stream = TcpStream::connect(&self.gateway_addr)
            .await
            .map_err(|e| format!("TCP connect: {e}"))?;
        let tls_stream = connector
            .connect(self.server_name.clone(), stream)
            .await
            .map_err(|e| format!("TLS connect: {e}"))?;

        let request = CloudRequest {
            tool: tool.to_string(),
            params: params.clone(),
            token: self.token.clone(),
        };

        let (reader, mut writer) = tokio::io::split(tls_stream);
        let mut json = serde_json::to_string(&request).map_err(|e| format!("JSON: {e}"))?;
        json.push('\n');
        writer
            .write_all(json.as_bytes())
            .await
            .map_err(|e| format!("Write: {e}"))?;

        let mut buf_reader = AsyncBufReader::new(reader);
        let mut line = String::new();
        buf_reader
            .read_line(&mut line)
            .await
            .map_err(|e| format!("Read: {e}"))?;
        let response: CloudResponse =
            serde_json::from_str(&line).map_err(|e| format!("JSON parse: {e}"))?;

        Ok((response.ok, response.output, response.error))
    }
}

// ── Certificate helpers ────────────────────────────────────────────────

fn load_certs(path: &str) -> Result<Vec<CertificateDer<'static>>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("Cannot open cert {}: {e}", path))?;
    let mut reader = BufReader::new(file);
    rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to parse certs: {e}"))
}

fn load_key(path: &str) -> Result<PrivateKeyDer<'static>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("Cannot open key {}: {e}", path))?;
    let mut reader = BufReader::new(file);
    rustls_pemfile::private_key(&mut reader)
        .map_err(|e| format!("Failed to parse key: {e}"))?
        .ok_or_else(|| format!("No private key in {}", path))
}
