//! ACP Transport Layer
//!
//! Defines transport abstractions for ACP communication protocols.
//! Supports stdio (local process) and HTTP (loopback network) transports.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Receive failed: {0}")]
    ReceiveFailed(String),

    #[error("Transport closed")]
    Closed,

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub payload: serde_json::Value,
    pub timestamp: i64,
}

#[async_trait]
pub trait AcpTransport: Send + Sync {
    async fn connect(&self) -> Result<(), TransportError>;
    async fn send(&self, message: AcpMessage) -> Result<(), TransportError>;
    async fn receive(&self) -> Result<AcpMessage, TransportError>;
    async fn close(&self) -> Result<(), TransportError>;
    fn is_connected(&self) -> bool;
}

pub struct StdioTransport {
    connected: std::sync::atomic::AtomicBool,
}

impl StdioTransport {
    pub fn new() -> Self {
        Self {
            connected: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AcpTransport for StdioTransport {
    async fn connect(&self) -> Result<(), TransportError> {
        self.connected
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, message: AcpMessage) -> Result<(), TransportError> {
        if !self.is_connected() {
            return Err(TransportError::Closed);
        }

        let json = serde_json::to_string(&message)
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;

        println!("{}", json);
        Ok(())
    }

    async fn receive(&self) -> Result<AcpMessage, TransportError> {
        if !self.is_connected() {
            return Err(TransportError::Closed);
        }

        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .map_err(|e| TransportError::ReceiveFailed(e.to_string()))?;

        let message: AcpMessage = serde_json::from_str(&line)
            .map_err(|e| TransportError::ReceiveFailed(e.to_string()))?;

        Ok(message)
    }

    async fn close(&self) -> Result<(), TransportError> {
        self.connected
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::SeqCst)
    }
}

pub struct LoopbackHttpTransport {
    address: String,
    port: u16,
    connected: std::sync::atomic::AtomicBool,
}

impl LoopbackHttpTransport {
    pub fn new(address: &str, port: u16) -> Result<Self, TransportError> {
        if !Self::is_loopback_address(address) {
            return Err(TransportError::SecurityViolation(format!(
                "Address {} is not a loopback address (127.0.0.1, ::1, or localhost)",
                address
            )));
        }

        Ok(Self {
            address: address.to_string(),
            port,
            connected: std::sync::atomic::AtomicBool::new(false),
        })
    }

    fn is_loopback_address(addr: &str) -> bool {
        matches!(addr, "127.0.0.1" | "::1" | "localhost")
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

#[async_trait]
impl AcpTransport for LoopbackHttpTransport {
    async fn connect(&self) -> Result<(), TransportError> {
        self.connected
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, _message: AcpMessage) -> Result<(), TransportError> {
        if !self.is_connected() {
            return Err(TransportError::Closed);
        }

        todo!("HTTP POST to http://{}:{}/acp/message", self.address, self.port)
    }

    async fn receive(&self) -> Result<AcpMessage, TransportError> {
        if !self.is_connected() {
            return Err(TransportError::Closed);
        }

        todo!("HTTP server receive")
    }

    async fn close(&self) -> Result<(), TransportError> {
        self.connected
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_transport_rejects_non_loopback() {
        let result = LoopbackHttpTransport::new("192.168.1.1", 8080);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TransportError::SecurityViolation(_)
        ));
    }

    #[test]
    fn loopback_transport_accepts_localhost() {
        let result = LoopbackHttpTransport::new("127.0.0.1", 8080);
        assert!(result.is_ok());
    }

    #[test]
    fn loopback_transport_accepts_ipv6_loopback() {
        let result = LoopbackHttpTransport::new("::1", 8080);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn stdio_transport_lifecycle() {
        let transport = StdioTransport::new();
        assert!(!transport.is_connected());

        transport.connect().await.unwrap();
        assert!(transport.is_connected());

        transport.close().await.unwrap();
        assert!(!transport.is_connected());
    }
}
