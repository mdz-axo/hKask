//! MCP server and tool management routes

use axum::{Extension, Json, extract::State, http::StatusCode, routing::Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::middleware::auth::AuthContext;
use crate::{ApiState, ErrorResponse};

/// Create MCP router
pub fn mcp_router() -> Router<ApiState> {
    Router::new()
        .route("/api/mcp/servers", axum::routing::get(list_servers))
        .route("/api/mcp/tools", axum::routing::get(list_tools))
        .route("/api/mcp/invoke", axum::routing::post(mcp_invoke))
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
async fn list_servers(State(state): State<ApiState>) -> Json<Vec<String>> {
    let servers = state.mcp_runtime.list_servers().await;
    Json(servers.iter().map(|s| s.id.clone()).collect())
}

/// List MCP tools
async fn list_tools(State(state): State<ApiState>) -> Json<Vec<String>> {
    let tools = state.mcp_runtime.discover_tools().await;
    Json(tools)
}

/// MCP invoke request body
#[derive(Debug, Deserialize, ToSchema)]
pub struct McpInvokeRequest {
    /// MCP server name
    pub server: String,
    /// Tool name to invoke
    pub tool: String,
    /// JSON input arguments (defaults to null)
    #[serde(default)]
    pub input: serde_json::Value,
}

/// MCP invoke response body
#[derive(Debug, Serialize, ToSchema)]
pub struct McpInvokeResponse {
    /// MCP server name
    pub server: String,
    /// Tool name that was invoked
    pub tool: String,
    /// Result from the tool invocation
    pub result: serde_json::Value,
}

/// Invoke an MCP tool directly.
///
/// Requires authentication via Bearer token. Dispatches the tool call
/// through the MCP runtime with capability verification.
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
async fn mcp_invoke(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<McpInvokeRequest>,
) -> Result<Json<McpInvokeResponse>, (StatusCode, Json<ErrorResponse>)> {
    use hkask_templates::McpPort;

    let input = if req.input.is_null() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        req.input
    };

    // Invoke via the MCP dispatcher with the authenticated capability token
    let result = state
        .mcp_dispatcher
        .invoke(&req.tool, input, &auth.token)
        .await
        .map_err(|e| {
            let code = match &e {
                hkask_templates::TemplateError::Mcp(_) => StatusCode::INTERNAL_SERVER_ERROR,
                hkask_templates::TemplateError::CapabilityDenied(_) => StatusCode::UNAUTHORIZED,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (
                code,
                Json(ErrorResponse {
                    error: e.to_string(),
                    code: code.as_u16().to_string(),
                    details: None,
                }),
            )
        })?;

    Ok(Json(McpInvokeResponse {
        server: req.server,
        tool: req.tool,
        result,
    }))
}
