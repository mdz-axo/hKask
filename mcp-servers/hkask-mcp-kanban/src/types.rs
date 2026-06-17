//! Request/response types for the kanban MCP server tools.
//!
//! Each tool has a request struct and response struct serializable
//! for MCP JSON-RPC transport.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── Board tools ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BoardCreateRequest {
    pub name: String,
    pub columns: Option<Vec<ColumnDefInput>>,
    /// OCAP capability token.
    pub capability_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ColumnDefInput {
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BoardCreateResponse {
    pub board_id: String,
    pub name: String,
    pub columns: Vec<ColumnInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ColumnInfo {
    pub id: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BoardListRequest {
    pub capability_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BoardListResponse {
    pub boards: Vec<BoardInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BoardInfo {
    pub board_id: String,
    pub name: String,
    pub column_count: usize,
}

// ── Task tools ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskCreateRequest {
    pub board_id: String,
    pub title: String,
    pub description: Option<String>,
    pub criteria: Option<Vec<String>>,
    pub assignee_webid: Option<String>,
    pub capability_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskCreateResponse {
    pub task_id: String,
    pub board_id: String,
    pub title: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskListRequest {
    pub board_id: String,
    pub status: Option<String>,
    pub capability_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskListResponse {
    pub tasks: Vec<TaskInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskInfo {
    pub task_id: String,
    pub title: String,
    pub status: String,
    pub assignee: Option<String>,
    pub criteria_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskMoveRequest {
    pub task_id: String,
    pub target_status: String,
    pub capability_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskMoveResponse {
    pub task_id: String,
    pub previous_status: String,
    pub new_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskAssignRequest {
    pub task_id: String,
    pub agent_webid: String,
    pub consent_proof_agent_webid: String,
    pub capability_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskAssignResponse {
    pub task_id: String,
    pub assignee: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskVerifyRequest {
    pub task_id: String,
    pub evidence: String,
    pub capability_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskVerifyResponse {
    pub task_id: String,
    pub passed: bool,
    pub reasoning: String,
    pub new_status: String,
}
