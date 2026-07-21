//! Protocol types for daemon socket communication.
//!
//! This module defines the request/response types exchanged between
//! MCP server binaries and the hKask daemon over the Unix domain socket.
//! The protocol is newline-delimited JSON with a `type` tag discriminator.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DaemonRequest {
    #[serde(rename = "auth_query")]
    AuthQuery { userpod: String },
    #[serde(rename = "assignment_query")]
    AssignmentQuery { userpod: String, role: String },
    #[serde(rename = "capability_query")]
    CapabilityQuery { userpod: String, tool: String },
    /// Store an experience in both episodic (first-person) and semantic (third-person) memory.
    /// Each experience generates two h_mems from the same event:
    /// - Episodic: specific, time-bound, perspective-scoped, private
    /// - Semantic: generalizable, timeless, no perspective, public
    #[serde(rename = "store_experience")]
    StoreExperience {
        userpod: String,
        entity: String,
        attribute: String,
        value: serde_json::Value,
        confidence: Option<f64>,
    },
    /// Dispatch a tool call through the daemon to an MCP server.
    #[serde(rename = "tool_dispatch")]
    ToolDispatch {
        userpod: String,
        tool: String,
        input: serde_json::Value,
    },
    /// Query curator system health — metacognition snapshot.
    #[serde(rename = "curator_health_query")]
    CuratorHealthQuery { userpod: String },
    /// Query live CNS status — variety per domain.
    #[serde(rename = "cns_status_query")]
    CnsStatusQuery {
        userpod: String,
        domain: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DaemonResponse {
    #[serde(rename = "auth_response")]
    AuthResponse {
        authenticated: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        webid: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<String>,
    },
    #[serde(rename = "assignment_response")]
    AssignmentResponse { assigned: bool },
    #[serde(rename = "capability_response")]
    CapabilityResponse { granted: bool },
    #[serde(rename = "error")]
    ErrorResponse { message: String },
    #[serde(rename = "store_response")]
    StoreResponse {
        stored: bool,
        episodic_id: Option<String>,
        semantic_id: Option<String>,
    },
    #[serde(rename = "tool_dispatch_response")]
    ToolDispatchResponse {
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// Curator health snapshot response.
    #[serde(rename = "curator_health_response")]
    CuratorHealthResponse { health: serde_json::Value },
    /// CNS status response.
    #[serde(rename = "cns_status_response")]
    CnsStatusResponse { status: serde_json::Value },
}
