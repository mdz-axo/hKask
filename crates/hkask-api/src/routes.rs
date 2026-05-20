//! HTTP routes implementation

use axum::{
    extract::Path,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::Router,
    Json,
};
use hkask_templates::RegistryIndex;
use serde_json::Value;

use crate::{
    ApiState, ChatRequest, ChatResponse, CnsHealthResponse, GrantCapabilityRequest,
    TemplateResponse,
};

/// Create templates router
pub fn templates_router() -> Router<ApiState> {
    Router::new()
        .route("/api/templates", axum::routing::get(list_templates))
        .route("/api/templates/:id", axum::routing::get(get_template))
        .route("/api/templates", axum::routing::post(register_template))
}

/// List templates
async fn list_templates(State(state): State<ApiState>) -> Json<Vec<TemplateResponse>> {
    let registry = state.registry.lock().await;
    let entries = registry.list(None);

    let templates = entries
        .iter()
        .map(|e| TemplateResponse {
            id: e.id.clone(),
            template_type: e.template_type.as_str().to_string(),
            description: e.description.clone(),
            source_path: e.source_path.clone(),
            lexicon_terms: e.lexicon_terms.clone(),
        })
        .collect();

    Json(templates)
}

/// Get template by ID
async fn get_template(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let registry = state.registry.lock().await;

    match registry.get(&id) {
        Ok(entry) => {
            let response = TemplateResponse {
                id: entry.id.clone(),
                template_type: entry.template_type.as_str().to_string(),
                description: entry.description.clone(),
                source_path: entry.source_path.clone(),
                lexicon_terms: entry.lexicon_terms.clone(),
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Register template
async fn register_template(
    State(state): State<ApiState>,
    Json(_req): Json<TemplateResponse>,
) -> StatusCode {
    let _registry = state.registry.lock().await;
    // TODO: Actual registration logic
    StatusCode::CREATED
}

/// Create bots router
pub fn bots_router() -> Router<ApiState> {
    Router::new()
        .route(
            "/api/bots/:id/capabilities",
            axum::routing::get(list_capabilities),
        )
        .route("/api/bots/:id/grant", axum::routing::post(grant_capability))
}

/// List bot capabilities
async fn list_capabilities(
    State(_state): State<ApiState>,
    Path(_id): Path<String>,
) -> Json<Vec<String>> {
    // TODO: Implement capability listing
    Json(vec![])
}

/// Grant capability to bot
async fn grant_capability(
    State(_state): State<ApiState>,
    Path(_id): Path<String>,
    Json(_req): Json<GrantCapabilityRequest>,
) -> StatusCode {
    // TODO: Implement capability granting
    StatusCode::OK
}

/// Create MCP router
pub fn mcp_router() -> Router<ApiState> {
    Router::new()
        .route("/api/mcp/servers", axum::routing::get(list_servers))
        .route("/api/mcp/tools", axum::routing::get(list_tools))
}

/// List MCP servers
async fn list_servers(State(state): State<ApiState>) -> Json<Vec<String>> {
    let servers = state.mcp_runtime.list_servers().await;
    Json(servers.iter().map(|s| s.id.clone()).collect())
}

/// List MCP tools
async fn list_tools(State(state): State<ApiState>) -> Json<Vec<String>> {
    let tools = state.mcp_runtime.discover_tools().await;
    Json(tools)
}

/// Create CNS router
pub fn cns_router() -> Router<ApiState> {
    Router::new()
        .route("/api/cns/health", axum::routing::get(cns_health))
        .route("/api/cns/alerts", axum::routing::get(cns_alerts))
}

/// CNS health status
async fn cns_health(State(_state): State<ApiState>) -> Json<CnsHealthResponse> {
    // TODO: Implement CNS health check
    Json(CnsHealthResponse {
        overall_deficit: 0,
        critical_count: 0,
        warning_count: 0,
        healthy: true,
    })
}

/// CNS algedonic alerts
async fn cns_alerts(State(_state): State<ApiState>) -> Json<Value> {
    // TODO: Implement alerts retrieval
    Json(Value::Array(vec![]))
}

/// Create chat router
pub fn chat_router() -> Router<ApiState> {
    Router::new().route("/api/chat", axum::routing::post(chat))
}

/// Curator chat endpoint
async fn chat(
    State(_state): State<ApiState>,
    Json(req): Json<ChatRequest>,
) -> Json<ChatResponse> {
    // TODO: Implement actual chat processing
    Json(ChatResponse {
        output: format!("Received: {}", req.input),
        template_id: req.template_id.unwrap_or("default".to_string()),
    })
}