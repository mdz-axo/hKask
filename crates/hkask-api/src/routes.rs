//! HTTP routes implementation

use axum::{
    extract::Path, extract::State, http::StatusCode, response::IntoResponse, routing::Router, Json,
};
use hkask_templates::RegistryIndex;
use serde_json::Value;
use std::collections::HashMap;

use crate::{
    ApiState, ChatRequest, ChatResponse, CnsHealthResponse, CnsVarietyResponse, CreatePodRequest,
    CreatePodResponse, GrantCapabilityRequest, ListPodsResponse, PodStatusResponse,
    TemplateResponse, ToolResponse,
};

/// Create templates router
pub fn templates_router() -> Router<ApiState> {
    Router::new()
        .route("/api/templates", axum::routing::get(list_templates))
        .route("/api/templates/:id", axum::routing::get(get_template))
        .route("/api/templates", axum::routing::post(register_template))
}

/// List templates
#[utoipa::path(
    get,
    path = "/api/templates",
    tag = "templates",
    responses(
        (status = 200, description = "List of templates", body = Vec<TemplateResponse>),
        (status = 500, description = "Internal server error"),
    ),
)]
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
#[utoipa::path(
    get,
    path = "/api/templates/{id}",
    tag = "templates",
    params(
        ("id" = String, Path, description = "Template ID"),
    ),
    responses(
        (status = 200, description = "Template details", body = TemplateResponse),
        (status = 404, description = "Template not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn get_template(State(state): State<ApiState>, Path(id): Path<String>) -> impl IntoResponse {
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

/// Search templates by lexicon term
async fn search_templates(
    State(state): State<ApiState>,
    Path(term): Path<String>,
) -> Json<Vec<TemplateResponse>> {
    let registry = state.registry.lock().await;
    let results = registry.search_by_lexicon(&term);

    let templates = results
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
#[utoipa::path(
    get,
    path = "/api/bots/{id}/capabilities",
    tag = "bots",
    params(
        ("id" = String, Path, description = "Bot WebID"),
    ),
    responses(
        (status = 200, description = "List of capabilities", body = Vec<String>),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn list_capabilities(
    State(_state): State<ApiState>,
    Path(_id): Path<String>,
) -> Json<Vec<String>> {
    // TODO: Implement capability listing
    Json(vec![])
}

/// Grant capability to bot
#[utoipa::path(
    post,
    path = "/api/bots/{id}/grant",
    tag = "bots",
    params(
        ("id" = String, Path, description = "Bot WebID"),
    ),
    request_body = GrantCapabilityRequest,
    responses(
        (status = 200, description = "Capability granted"),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn grant_capability(
    State(_state): State<ApiState>,
    Path(_id): Path<String>,
    Json(_req): Json<GrantCapabilityRequest>,
) -> StatusCode {
    // TODO: Implement capability granting
    StatusCode::OK
}

/// Create pods router
pub fn pods_router() -> Router<ApiState> {
    Router::new()
        .route("/api/pods", axum::routing::get(list_pods))
        .route("/api/pods", axum::routing::post(create_pod))
        .route("/api/pods/:id/activate", axum::routing::post(activate_pod))
        .route(
            "/api/pods/:id/deactivate",
            axum::routing::post(deactivate_pod),
        )
        .route("/api/pods/:id/status", axum::routing::get(pod_status))
}

/// List all pods
async fn list_pods(State(state): State<ApiState>) -> Json<ListPodsResponse> {
    state.cns_emitter.emit_agent_pod(
        "api.pod.list.start",
        serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    );

    let pod_statuses = state.pod_manager.list_pods().await.unwrap_or_default();

    let pods: Vec<PodStatusResponse> = pod_statuses
        .into_iter()
        .map(|s| PodStatusResponse {
            pod_id: s.pod_id,
            name: s.name,
            state: s.state,
            webid: s.webid,
            agent_type: s.agent_type,
            template: s.template,
            created_at: s.created_at,
        })
        .collect();

    state.cns_emitter.emit_agent_pod(
        "api.pod.list.outcome",
        serde_json::json!({
            "count": pods.len(),
        }),
    );

    Json(ListPodsResponse { pods })
}

/// Create a new pod
async fn create_pod(
    State(state): State<ApiState>,
    Json(req): Json<CreatePodRequest>,
) -> Result<Json<CreatePodResponse>, StatusCode> {
    use hkask_agents::pod::AgentPersona;
    use hkask_types::{CapabilityAction, CapabilityResource};

    state.cns_emitter.emit_agent_pod(
        "api.pod.create.start",
        serde_json::json!({
            "template": req.template,
            "name": req.name,
        }),
    );

    let user_webid = state.system_webid.clone();

    let has_capability = state.capability_checker.check_resource(
        &hkask_types::CapabilityToken::new(
            CapabilityResource::Tool,
            "pod".to_string(),
            CapabilityAction::Execute,
            state.system_webid.clone(),
            user_webid.clone(),
            b"temp-secret",
        ),
        &user_webid,
        CapabilityResource::Tool,
    );

    if !has_capability {
        state.cns_emitter.emit_agent_pod(
            "api.pod.create.denied",
            serde_json::json!({
                "reason": "capability_check_failed",
            }),
        );
        return Err(StatusCode::FORBIDDEN);
    }

    let persona = AgentPersona::from_yaml(&req.persona_yaml).map_err(|e| {
        tracing::warn!("Invalid persona YAML: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    let pod_id = state
        .pod_manager
        .create_pod(&req.template, &persona, req.name)
        .await
        .map_err(|e| {
            state.cns_emitter.emit_agent_pod(
                "api.pod.create.error",
                serde_json::json!({
                    "error": e.to_string(),
                }),
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    state.cns_emitter.emit_agent_pod(
        "api.pod.create.success",
        serde_json::json!({
            "pod_id": pod_id.to_string(),
        }),
    );

    Ok(Json(CreatePodResponse {
        pod_id: pod_id.to_string(),
    }))
}

/// Activate a pod
async fn activate_pod(State(state): State<ApiState>, Path(id): Path<String>) -> StatusCode {
    use hkask_agents::pod::PodID;
    use uuid::Uuid;

    state.cns_emitter.emit_agent_pod(
        "api.pod.activate.start",
        serde_json::json!({
            "pod_id": id,
        }),
    );

    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            state.cns_emitter.emit_agent_pod(
                "api.pod.activate.error",
                serde_json::json!({
                    "reason": "invalid_pod_id",
                }),
            );
            return StatusCode::BAD_REQUEST;
        }
    };
    let pod_id = PodID(uuid);

    match state.pod_manager.activate_pod(&pod_id).await {
        Ok(_) => {
            state.cns_emitter.emit_agent_pod(
                "api.pod.activate.success",
                serde_json::json!({
                    "pod_id": id,
                }),
            );
            StatusCode::NO_CONTENT
        }
        Err(e) => {
            state.cns_emitter.emit_agent_pod(
                "api.pod.activate.error",
                serde_json::json!({
                    "reason": e.to_string(),
                }),
            );
            StatusCode::NOT_FOUND
        }
    }
}

/// Deactivate a pod
async fn deactivate_pod(State(state): State<ApiState>, Path(id): Path<String>) -> StatusCode {
    use hkask_agents::pod::PodID;
    use uuid::Uuid;

    state.cns_emitter.emit_agent_pod(
        "api.pod.deactivate.start",
        serde_json::json!({
            "pod_id": id,
        }),
    );

    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            state.cns_emitter.emit_agent_pod(
                "api.pod.deactivate.error",
                serde_json::json!({
                    "reason": "invalid_pod_id",
                }),
            );
            return StatusCode::BAD_REQUEST;
        }
    };
    let pod_id = PodID(uuid);

    match state.pod_manager.deactivate_pod(&pod_id).await {
        Ok(_) => {
            state.cns_emitter.emit_agent_pod(
                "api.pod.deactivate.success",
                serde_json::json!({
                    "pod_id": id,
                }),
            );
            StatusCode::NO_CONTENT
        }
        Err(e) => {
            state.cns_emitter.emit_agent_pod(
                "api.pod.deactivate.error",
                serde_json::json!({
                    "reason": e.to_string(),
                }),
            );
            StatusCode::NOT_FOUND
        }
    }
}

/// Get pod status
async fn pod_status(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<PodStatusResponse>, StatusCode> {
    use hkask_agents::pod::PodID;
    use uuid::Uuid;

    state.cns_emitter.emit_agent_pod(
        "api.pod.status.start",
        serde_json::json!({
            "pod_id": id,
        }),
    );

    let uuid = Uuid::parse_str(&id).map_err(|_| {
        state.cns_emitter.emit_agent_pod(
            "api.pod.status.error",
            serde_json::json!({
                "reason": "invalid_pod_id",
            }),
        );
        StatusCode::BAD_REQUEST
    })?;
    let pod_id = PodID(uuid);

    let status = state
        .pod_manager
        .get_pod_status(&pod_id)
        .await
        .map_err(|e| {
            state.cns_emitter.emit_agent_pod(
                "api.pod.status.error",
                serde_json::json!({
                    "reason": e.to_string(),
                }),
            );
            StatusCode::NOT_FOUND
        })?;

    state.cns_emitter.emit_agent_pod(
        "api.pod.status.success",
        serde_json::json!({
            "pod_id": id,
            "state": status.state,
        }),
    );

    Ok(Json(PodStatusResponse {
        pod_id: status.pod_id,
        name: status.name,
        state: status.state,
        webid: status.webid,
        agent_type: status.agent_type,
        template: status.template,
        created_at: status.created_at,
    }))
}

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

/// Get tool definition
async fn get_tool(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> Result<Json<ToolResponse>, StatusCode> {
    let tool = state
        .mcp_runtime
        .get_tool(&name)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ToolResponse {
        name: tool.name,
        description: tool.description,
        input_schema: tool.input_schema,
        server_id: tool.server_id,
    }))
}

/// Create CNS router
pub fn cns_router() -> Router<ApiState> {
    Router::new()
        .route("/api/cns/health", axum::routing::get(cns_health))
        .route("/api/cns/alerts", axum::routing::get(cns_alerts))
        .route("/api/cns/variety", axum::routing::get(cns_variety))
}

/// CNS health status
#[utoipa::path(
    get,
    path = "/api/cns/health",
    tag = "cns",
    responses(
        (status = 200, description = "CNS health status", body = CnsHealthResponse),
        (status = 500, description = "Internal server error"),
    ),
)]
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

/// CNS variety counters
#[utoipa::path(
    get,
    path = "/api/cns/variety",
    tag = "cns",
    responses(
        (status = 200, description = "CNS variety counters", body = CnsVarietyResponse),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn cns_variety(State(_state): State<ApiState>) -> Json<CnsVarietyResponse> {
    // TODO: Implement variety counter retrieval from CNS
    // Placeholder response for now
    Json(CnsVarietyResponse {
        domains: vec![],
        total_deficit: 0,
        counters: HashMap::new(),
    })
}

/// Create chat router
pub fn chat_router() -> Router<ApiState> {
    Router::new().route("/api/chat", axum::routing::post(chat))
}

/// Curator chat endpoint
#[utoipa::path(
    post,
    path = "/api/chat",
    tag = "chat",
    request_body = ChatRequest,
    responses(
        (status = 200, description = "Chat response", body = ChatResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn chat(State(_state): State<ApiState>, Json(req): Json<ChatRequest>) -> Json<ChatResponse> {
    // TODO: Implement actual chat processing
    Json(ChatResponse {
        output: format!("Received: {}", req.input),
        template_id: req.template_id.unwrap_or("default".to_string()),
    })
}
