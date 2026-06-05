//! MCP tool request types for the replicant server
//!
//! These types are used by the MCP tool handlers to deserialize
//! incoming tool calls from the MCP protocol.

use schemars::JsonSchema;
use serde::Deserialize;

/// Request payload for `replicant:chat`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChatRequest {
    /// The message to send to the replicant
    pub message: String,
    /// Model override (optional — uses the server default if empty)
    #[serde(default)]
    pub model: String,
}

/// Request payload for `replicant:status`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct StatusRequest {
    /// Replicant persona name (optional — uses the server default if empty)
    #[serde(default)]
    pub persona: String,
}

/// Request payload for `replicant:history`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct HistoryRequest {
    /// Maximum number of turns to return (default: all)
    #[serde(default)]
    pub limit: Option<usize>,
}
