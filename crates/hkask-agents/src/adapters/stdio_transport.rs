//! Stdio Transport — Newline-delimited JSON over stdin/stdout
//!
//! Provides process-isolated ACP communication with no network exposure.
//! Used for parent-child process communication (e.g., Russell pods launched
//! by hKask).
//!
//! # Protocol
//!
//! Each message is a single line of JSON (newline-delimited). The sender
//! writes a `AcpWireMessage` as JSON followed by `\n`. The receiver reads
//! lines and deserializes them.
//!
//! # Security
//!
//! No network exposure. Communication is limited to parent-child processes
//! sharing stdin/stdout file descriptors.

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

use crate::acp::AcpError;
use crate::ports::{AcpTransport, AcpWireMessage, AcpWireResponse};

/// Stdio transport for ACP communication
///
/// Reads newline-delimited JSON from stdin, writes to stdout.
/// Thread-safe via internal mutexes on reader and writer.
pub struct StdioTransport {
    reader: Mutex<BufReader<tokio::io::Stdin>>,
    writer: Mutex<tokio::io::Stdout>,
}

impl StdioTransport {
    /// Create a new stdio transport bound to the current process's stdin/stdout
    pub fn new() -> Self {
        Self {
            reader: Mutex::new(BufReader::new(tokio::io::stdin())),
            writer: Mutex::new(tokio::io::stdout()),
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
    async fn send(&self, msg: &AcpWireMessage) -> Result<AcpWireResponse, AcpError> {
        let json = serde_json::to_string(msg)
            .map_err(|e| AcpError::TransportError(format!("Serialization failed: {e}")))?;

        let mut writer = self.writer.lock().await;
        writer
            .write_all(json.as_bytes())
            .await
            .map_err(|e| AcpError::TransportError(format!("Write failed: {e}")))?;
        writer
            .write_all(b"\n")
            .await
            .map_err(|e| AcpError::TransportError(format!("Write failed: {e}")))?;
        writer
            .flush()
            .await
            .map_err(|e| AcpError::TransportError(format!("Flush failed: {e}")))?;

        Ok(AcpWireResponse::ok(
            msg.id.clone(),
            serde_json::json!({"sent": true}),
        ))
    }

    async fn receive(&self) -> Result<AcpWireMessage, AcpError> {
        let mut reader = self.reader.lock().await;
        let mut line = String::new();

        let bytes_read = reader
            .read_line(&mut line)
            .await
            .map_err(|e| AcpError::TransportError(format!("Read failed: {e}")))?;

        if bytes_read == 0 {
            return Err(AcpError::Disconnected);
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Err(AcpError::TransportError("Empty message".to_string()));
        }

        serde_json::from_str(trimmed)
            .map_err(|e| AcpError::TransportError(format!("Deserialization failed: {e}")))
    }

    fn is_connected(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::A2AMessage;
    use hkask_types::WebID;

    #[test]
    fn test_wire_message_serialization_roundtrip() {
        let msg = AcpWireMessage::new(A2AMessage::MemoryArtifact {
            producer: WebID::new(),
            artifact_type: "episodic_triple".to_string(),
            artifact_id: "art-001".to_string(),
            visibility: "private".to_string(),
        });

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: AcpWireMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(msg.id, deserialized.id);
        assert_eq!(msg.timestamp, deserialized.timestamp);
    }

    #[test]
    fn test_wire_response_ok() {
        let resp = AcpWireResponse::ok("test-id".to_string(), serde_json::json!({"status": "ok"}));
        assert!(resp.success);
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_wire_response_err() {
        let resp = AcpWireResponse::err("test-id".to_string(), "something failed".to_string());
        assert!(!resp.success);
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
    }

    #[test]
    fn test_stdio_transport_is_connected() {
        let transport = StdioTransport::new();
        assert!(transport.is_connected());
    }
}
