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

use async_trait::async_trait;
use reqwest::Client;
use std::net::SocketAddr;

use crate::acp::AcpError;
use crate::ports::{AcpTransport, AcpWireMessage, AcpWireResponse};

/// Loopback HTTP transport for ACP communication
///
/// Sends ACP messages as HTTP POST requests to a loopback endpoint.
/// The constructor enforces that the target address is loopback only.
pub struct LoopbackHttpTransport {
    endpoint: SocketAddr,
    client: Client,
}

impl std::fmt::Debug for LoopbackHttpTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoopbackHttpTransport")
            .field("endpoint", &self.endpoint)
            .finish()
    }
}

impl LoopbackHttpTransport {
    /// Create a new loopback HTTP transport
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

        Ok(Self { endpoint, client })
    }

    /// Get the target endpoint address
    pub fn endpoint(&self) -> SocketAddr {
        self.endpoint
    }

    fn endpoint_url(&self, path: &str) -> String {
        format!("http://{}{}", self.endpoint, path)
    }
}

#[async_trait]
impl AcpTransport for LoopbackHttpTransport {
    async fn send(&self, msg: &AcpWireMessage) -> Result<AcpWireResponse, AcpError> {
        let url = self.endpoint_url("/acp/message");

        let response = self
            .client
            .post(&url)
            .json(msg)
            .send()
            .await
            .map_err(|e| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_rejects_non_loopback_ipv4() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8080);
        let result = LoopbackHttpTransport::new(addr);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AcpError::NonLoopbackRefused(_)));
    }

    #[test]
    fn test_rejects_public_ipv4() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 8080);
        let result = LoopbackHttpTransport::new(addr);
        assert!(result.is_err());
    }

    #[test]
    fn test_accepts_loopback_ipv4() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let result = LoopbackHttpTransport::new(addr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().endpoint(), addr);
    }

    #[test]
    fn test_accepts_loopback_ipv6() {
        let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 8080);
        let result = LoopbackHttpTransport::new(addr);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rejects_non_loopback_ipv6() {
        let addr = SocketAddr::new(
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
            8080,
        );
        let result = LoopbackHttpTransport::new(addr);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_connected() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9090);
        let transport = LoopbackHttpTransport::new(addr).unwrap();
        assert!(transport.is_connected());
    }

    #[tokio::test]
    async fn test_receive_returns_error() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9090);
        let transport = LoopbackHttpTransport::new(addr).unwrap();
        let result = transport.receive().await;
        assert!(result.is_err());
    }
}
