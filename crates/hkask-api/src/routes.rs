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

    let pod_statuses: Vec<_> = state.pod_manager.list_pods().await.unwrap_or_default();

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

/// Create sovereignty router
pub fn sovereignty_router() -> Router<ApiState> {
    Router::new()
        .route("/api/sovereignty/status", axum::routing::get(sovereignty_status))
        .route("/api/sovereignty/consent/grant", axum::routing::post(sovereignty_grant_consent))
        .route("/api/sovereignty/consent/revoke", axum::routing::post(sovereignty_revoke_consent))
        .route("/api/sovereignty/killzone", axum::routing::get(sovereignty_killzone))
        .route("/api/sovereignty/access/check", axum::routing::get(sovereignty_check_access))
}

/// Sovereignty status response
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SovereigntyStatusResponse {
    pub explicit_consent: bool,
    pub sovereignty_compromised: bool,
    pub kill_zone_active: bool,
    pub vc_investment: f32,
    pub threshold: f32,
    pub acquisition_resistance: String,
    pub sovereign_data: Vec<String>,
    pub shared_data: Vec<String>,
    pub public_data: Vec<String>,
}

/// Sovereignty consent response
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SovereigntyConsentResponse {
    pub consent: bool,
    pub message: String,
}

/// Kill zone status response
#[derive(Serialize, Deserialize, ToSchema)]
pub struct KillZoneResponse {
    pub active: bool,
    pub acquisition_attempt: bool,
    pub vc_investment: f32,
    pub threshold: f32,
}

/// Access check response
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccessCheckResponse {
    pub category: String,
    pub classification: String,
    pub access_required: String,
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

/// Sovereignty status endpoint
#[utoipa::path(
    get,
    path = "/api/sovereignty/status",
    tag = "sovereignty",
    responses(
        (status = 200, description = "Sovereignty status", body = SovereigntyStatusResponse),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn sovereignty_status(State(_state): State<ApiState>) -> Json<SovereigntyStatusResponse> {
    use hkask_types::UserSovereigntyState;
    let state = UserSovereigntyState::new();
    
    Json(SovereigntyStatusResponse {
        explicit_consent: state.explicit_consent,
        sovereignty_compromised: state.is_compromised(),
        kill_zone_active: state.detector.kill_zone_active,
        vc_investment: state.detector.vc_investment,
        threshold: state.detector.threshold,
        acquisition_resistance: format!("{:?}", state.boundary.resistance),
        sovereign_data: state.boundary.sovereign_data.clone(),
        shared_data: state.boundary.shared_data.clone(),
        public_data: state.boundary.public_data.clone(),
    })
}

/// Grant consent endpoint
async fn sovereignty_grant_consent(State(_state): State<ApiState>) -> Json<SovereigntyConsentResponse> {
    Json(SovereigntyConsentResponse {
        consent: true,
        message: "Explicit consent granted. Data sharing enabled for shared categories.".to_string(),
    })
}

/// Revoke consent endpoint
async fn sovereignty_revoke_consent(State(_state): State<ApiState>) -> Json<SovereigntyConsentResponse> {
    Json(SovereigntyConsentResponse {
        consent: false,
        message: "Explicit consent revoked. Only public data accessible.".to_string(),
    })
}

/// Kill zone status endpoint
#[utoipa::path(
    get,
    path = "/api/sovereignty/killzone",
    tag = "sovereignty",
    responses(
        (status = 200, description = "Kill zone status", body = KillZoneResponse),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn sovereignty_killzone(State(_state): State<ApiState>) -> Json<KillZoneResponse> {
    use hkask_types::UserSovereigntyState;
    let state = UserSovereigntyState::new();
    
    Json(KillZoneResponse {
        active: state.detector.kill_zone_active,
        acquisition_attempt: state.detector.acquisition_attempt,
        vc_investment: state.detector.vc_investment,
        threshold: state.detector.threshold,
    })
}

/// Check access endpoint
#[utoipa::path(
    get,
    path = "/api/sovereignty/access/check",
    tag = "sovereignty",
    params(
        ("category" = String, Query, description = "Data category to check"),
    ),
    responses(
        (status = 200, description = "Access check result", body = AccessCheckResponse),
        (status = 400, description = "Missing category parameter"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn sovereignty_check_access(
    State(_state): State<ApiState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<AccessCheckResponse> {
    use hkask_types::UserSovereigntyState;
    
    let category = params.get("category").map(|s| s.as_str()).unwrap_or("");
    let state = UserSovereigntyState::new();
    
    let (classification, access_required) = if state.boundary.is_sovereign(category) {
        ("SOVEREIGN".to_string(), "Requires explicit consent AND owner".to_string())
    } else if state.boundary.shared_data.contains(&category.to_string()) {
        ("SHARED".to_string(), "Requires explicit consent".to_string())
    } else if state.boundary.public_data.contains(&category.to_string()) {
        ("PUBLIC".to_string(), "Always accessible".to_string())
    } else {
        ("UNKNOWN".to_string(), "Denied by default".to_string())
    };
    
    Json(AccessCheckResponse {
        category: category.to_string(),
        classification,
        access_required,
    })
}
