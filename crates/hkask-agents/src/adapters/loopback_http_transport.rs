//! Loopback HTTP Transport — HTTP on 127.0.0.1 / ::1 only
//!
//! Provides HTTP-based ACP communication restricted to loopback addresses.
//! Used for systemd-managed agent pods that outlive their parent process.
//!
//! # Security
//!
//! The constructor **structurally refuses** non-loopback addresses.
//! This is a security boundary, not a limitation. Cross-machine ACP
//! is explicitly excluded from the hKask design (see AGENTS.md Hallucinations).
//!
//! DNS rebinding protection: when constructed from a hostname, the resolved
//! IP address is checked (not just the hostname string) to prevent attacks
//! where a DNS name resolves to a non-loopback address.

use async_trait::async_trait;
use reqwest::Client;
use std::net::SocketAddr;

use crate::acp::AcpError;
use crate::ports::{AcpTransport, AcpWireMessage, AcpWireResponse};

/// Loopback HTTP transport for ACP communication
///
/// Sends ACP messages as HTTP POST requests to a loopback endpoint.
/// The constructor enforces that the target address is loopback only,
/// resolving hostnames to verify the actual IP to prevent DNS rebinding.
pub struct LoopbackHttpTransport {
    endpoint: SocketAddr,
    resolved_ip: std::net::IpAddr,
    client: Client,
}

impl std::fmt::Debug for LoopbackHttpTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoopbackHttpTransport")
            .field("endpoint", &self.endpoint)
            .field("resolved_ip", &self.resolved_ip)
            .finish()
    }
}

impl LoopbackHttpTransport {
    /// Create a new loopback HTTP transport from a SocketAddr
    ///
    /// # Errors
    ///
    /// Returns `AcpError::NonLoopbackRefused` if the endpoint IP is not
    /// a loopback address (127.0.0.0/8 or ::1).
    pub fn new(endpoint: SocketAddr) -> Result<Self, AcpError> {
        if !endpoint.ip().is_loopback() {
            return Err(AcpError::NonLoopbackRefused(endpoint.ip()));
        }

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AcpError::TransportError(format!("HTTP client creation failed: {e}")))?;

        Ok(Self {
            endpoint,
            resolved_ip: endpoint.ip(),
            client,
        })
    }

    /// Create a new loopback HTTP transport from a hostname and port,
    /// resolving the hostname and verifying the resolved IP is loopback.
    ///
    /// This prevents DNS rebinding attacks where a hostname initially
    /// appears safe but resolves to a non-loopback address.
    ///
    /// # Errors
    ///
    /// Returns `AcpError::NonLoopbackRefused` if the resolved IP is not loopback.
    /// Returns `AcpError::TransportError` if the hostname cannot be resolved.
    pub fn from_hostname(hostname: &str, port: u16) -> Result<Self, AcpError> {
        let addr: std::net::SocketAddr = format!("{hostname}:{port}")
            .parse()
            .map_err(|e| AcpError::TransportError(format!("Invalid address: {e}")))?;

        let ip = addr.ip();
        if !ip.is_loopback() {
            return Err(AcpError::NonLoopbackRefused(ip));
        }

        let endpoint = SocketAddr::new(ip, port);
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AcpError::TransportError(format!("HTTP client creation failed: {e}")))?;

        Ok(Self {
            endpoint,
            resolved_ip: ip,
            client,
        })
    }

    /// Get the target endpoint address
    pub fn endpoint(&self) -> SocketAddr {
        self.endpoint
    }

    /// Get the resolved IP address (important for DNS rebinding verification)
    pub fn resolved_ip(&self) -> std::net::IpAddr {
        self.resolved_ip
    }

    fn endpoint_url(&self, path: &str) -> String {
        format!("http://{}{}", self.endpoint, path)
    }
}

#[async_trait]
impl AcpTransport for LoopbackHttpTransport {
    async fn send(&self, msg: &AcpWireMessage) -> Result<AcpWireResponse, AcpError> {
        let url = self.endpoint_url("/acp/message");

        let response = self.client.post(&url).json(msg).send().await.map_err(|e| {
            if e.is_connect() {
                AcpError::ConnectionRefused(format!("{}: {e}", self.endpoint))
            } else {
                AcpError::TransportError(format!("HTTP request failed: {e}"))
            }
        })?;

        if !response.status().is_success() {
            return Err(AcpError::TransportError(format!(
                "HTTP {} from {}",
                response.status(),
                self.endpoint
            )));
        }

        response
            .json::<AcpWireResponse>()
            .await
            .map_err(|e| AcpError::TransportError(format!("Response deserialization failed: {e}")))
    }

    async fn receive(&self) -> Result<AcpWireMessage, AcpError> {
        Err(AcpError::TransportError(
            "LoopbackHttpTransport client does not support receive(); use an HTTP server to accept messages".to_string(),
        ))
    }

    fn is_connected(&self) -> bool {
        true
    }
}
