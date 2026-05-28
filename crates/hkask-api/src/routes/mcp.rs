//! MCP server and tool management routes

use axum::{Json, extract::State, routing::Router};

use crate::ApiState;

/// Create MCP router
pub fn mcp_router() -> Router<ApiState> {
    Router::new()
        .route("/api/mcp/servers", axum::routing::get(list_servers))
        .route("/api/mcp/tools", axum::routing::get(list_tools))
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
