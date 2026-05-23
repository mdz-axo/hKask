//! ACP Transport — Wire protocol abstraction for ACP communication
//!
//! Defines the interface for sending/receiving ACP messages over
//! different transports (stdio, HTTP loopback).
//!
//! # Security
//!
//! All transports enforce security boundaries:
//! - `StdioTransport`: Process isolation, no network exposure
//! - `LoopbackHttpTransport`: Refuses non-loopback addresses

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::acp::{A2AMessage, AcpError};

/// ACP wire message — serialized format for transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpWireMessage {
    /// Message ID for correlation
    pub id: String,
    /// Message payload
    pub payload: A2AMessage,
    /// Timestamp (Unix epoch seconds)
    pub timestamp: i64,
}

impl AcpWireMessage {
    /// Create a new wire message wrapping an A2A message
    pub fn new(payload: A2AMessage) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            payload,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

/// ACP wire response — serialized response for transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpWireResponse {
    /// Correlation ID matching the request
    pub id: String,
    /// Success status
    pub success: bool,
    /// Result data (if successful)
    pub result: Option<serde_json::Value>,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl AcpWireResponse {
    /// Create a success response
    pub fn ok(id: String, result: serde_json::Value) -> Self {
        Self {
            id,
            success: true,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn err(id: String, error: String) -> Self {
        Self {
            id,
            success: false,
            result: None,
            error: Some(error),
        }
    }
}

/// ACP Transport — Send and receive ACP messages over a wire protocol
///
/// # Implementations
///
/// - `StdioTransport`: JSON-RPC over stdin/stdout (process isolation)
/// - `LoopbackHttpTransport`: HTTP on 127.0.0.1/::1 only (systemd pods)
///
/// # Security
///
/// Cross-machine ACP is explicitly excluded (see AGENTS.md Hallucinations).
/// Each transport enforces its own security boundary.
#[async_trait]
pub trait AcpTransport: Send + Sync {
    /// Send an ACP message and wait for a response
    async fn send(&self, msg: &AcpWireMessage) -> Result<AcpWireResponse, AcpError>;

    /// Receive an ACP message (blocking until available)
    async fn receive(&self) -> Result<AcpWireMessage, AcpError>;

    /// Check if the transport is connected and operational
    fn is_connected(&self) -> bool;
}
