//! MCP server and tool management routes
//!
//! These endpoints list and invoke MCP tools. Servers must be started
//! (e.g., via the `API_SERVERS` table in `serve.rs`) for tools to be
//! discoverable and invokable. If no servers are running, the listing
//! endpoints return empty results and invocations will fail.
//!
//! # Service layer depth test
//!
//! McpService was considered but **rejected** as shallow: every handler is a
//! thin delegation to `McpRuntime`/`McpDispatcher` methods (`list_servers`,
//! `discover_tools`, `invoke`, `get_tool_info`) plus HTTP response mapping.
//! No CLI MCP commands share this logic (CLI `commands/mcp.rs` uses a separate
//! `create_mcp_dispatcher_with_servers` path). An McpService would just be
//! `self.mcp_runtime().discover_tools()` — a pure pass-through.
//!
//! Decision: Guideline — keep direct `service_context.mcp_runtime()`/`mcp_dispatcher()`
//! access. Revisit if MCP orchestration logic (e.g., server health monitoring,
//! tool result caching) grows beyond simple discovery/invocation.

use axum::extract::Extension;
use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ApiError;
use crate::ApiState;
use crate::middleware::auth::AuthContext;

/// Create MCP router
pub fn mcp_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(list_servers))
        .routes(routes!(list_tools))
        .routes(routes!(mcp_invoke))
}

/// List MCP servers
#[utoipa::path(
    get,
    path = "/api/mcp/servers",
    tag = "mcp",
    responses(
        (status = 200, description = "List of MCP servers", body = Vec<String>),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn list_servers(State(state): State<ApiState>) -> Json<Vec<String>> {
    let servers = state.agent_service.mcp_runtime().list_servers().await;
    Json(servers.iter().map(|s| s.id.clone()).collect())
}

/// List MCP tools
///
/// Returns all tools discovered from running MCP servers. If no servers
/// are started, the list will be empty.
#[utoipa::path(
    get,
    path = "/api/mcp/tools",
    tag = "mcp",
    responses(
        (status = 200, description = "List of MCP tool names", body = Vec<String>),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn list_tools(State(state): State<ApiState>) -> Json<Vec<String>> {
    let tools = state.agent_service.mcp_runtime().discover_tools().await;
    Json(tools)
}

/// MCP invoke request body
#[derive(Debug, Deserialize, ToSchema)]
pub struct McpInvokeRequest {
    /// Tool name to invoke (e.g., "inference_generate")
    pub tool: String,
    /// JSON input arguments (defaults to null)
    #[serde(default)]
    pub input: serde_json::Value,
}

/// MCP invoke response body
#[derive(Debug, Serialize, ToSchema)]
pub struct McpInvokeResponse {
    /// MCP server that provided the tool
    pub server: String,
    /// Tool name that was invoked
    pub tool: String,
    /// Result from the tool invocation
    pub result: serde_json::Value,
}

/// Invoke an MCP tool directly.
///
/// Requires authentication via Bearer token. Dispatches the tool call
/// through the MCP runtime with capability verification. The server
/// that owns the tool is resolved automatically from the tool name.
#[utoipa::path(
    post,
    path = "/api/mcp/invoke",
    tag = "mcp",
    request_body = McpInvokeRequest,
    responses(
        (status = 200, description = "Tool invocation result", body = McpInvokeResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Tool or server not found"),
        (status = 500, description = "Tool invocation error"),
    ),
)]
pub(crate) async fn mcp_invoke(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<McpInvokeRequest>,
) -> Result<Json<McpInvokeResponse>, ApiError> {
    use hkask_templates::McpPort;

    let input = if req.input.is_null() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        req.input
    };

    // Invoke via the MCP dispatcher with the authenticated capability token
    let result = state
        .agent_service
        .mcp_dispatcher()
        .invoke(&req.tool, input, &auth.token)
        .await
        .map_err(|e| ApiError::from(hkask_services::ServiceError::Template(e)))?;

    // Resolve server_id from the runtime's tool registry
    let server_id = state
        .agent_service
        .mcp_runtime()
        .get_tool_info(&req.tool)
        .await
        .map(|t| t.server_id)
        .unwrap_or_else(|| "unknown".to_string());

    Ok(Json(McpInvokeResponse {
        server: server_id,
        tool: req.tool,
        result,
    }))
}
