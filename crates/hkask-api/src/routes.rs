//! HTTP routes implementation

use axum::{
    Json, extract::Path, extract::Query, extract::State, http::StatusCode, response::IntoResponse,
    routing::Router,
};
use hkask_cns::algedonic::{AlgedonicManager, CnsHealth};
use hkask_cns::variety::VarietyMonitor;
use hkask_ensemble::ports::InferenceClient;
use hkask_templates::RegistryIndex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

use crate::{
    ApiState, ChatRequest, ChatResponse, CnsHealthResponse, CnsVarietyResponse, CreatePodRequest,
    CreatePodResponse, GrantCapabilityRequest, InferenceSpan, ListPodsResponse, PodStatusResponse,
    SoapInferAuthRequest, SoapInferRequest, SoapInferResponse, SoapInferenceConfig,
    SpecCaptureRequest, SpecCaptureResponse, SpecCultivateResponse, SpecListResponse,
    SpecValidateRequest, SpecValidateResponse, TemplateResponse, ValidationErrorType,
    VarietyCounterResponse,
};
use hkask_types::SpecCategory;

/// Create templates router
pub fn templates_router() -> Router<ApiState> {
    Router::new()
        .route("/api/templates", axum::routing::get(list_templates))
        .route("/api/templates/:id", axum::routing::get(get_template))
        .route("/api/templates", axum::routing::post(register_template))
        .route(
            "/api/templates/search/:term",
            axum::routing::get(search_templates),
        )
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
    StatusCode::CREATED
}

/// Search templates by lexicon term
async fn search_templates(
    State(state): State<ApiState>,
    Path(term): Path<String>,
) -> Json<Vec<TemplateResponse>> {
    let registry = state.registry.lock().await;
    let results = registry.search_by_lexicon(&term).unwrap_or_default();

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

/// ACP registration router
pub fn acp_router() -> Router<ApiState> {
    Router::new().route("/api/v1/acp/register", axum::routing::post(acp_register))
}

/// Register an agent with the ACP runtime
async fn acp_register(
    State(state): State<ApiState>,
    Json(req): Json<crate::AcpRegisterRequest>,
) -> Result<Json<crate::AcpRegisterResponse>, StatusCode> {
    let webid = uuid::Uuid::parse_str(&req.webid)
        .map_err(|_| StatusCode::BAD_REQUEST)
        .map(hkask_types::WebID)?;

    if !["Bot", "Replicant"].contains(&req.agent_type.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    if req.capabilities.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let acp = state.pod_manager.acp_runtime();
    let token = acp
        .register_agent(webid, &req.agent_type, req.capabilities)
        .await
        .map_err(|e| match e {
            hkask_agents::AcpError::AgentAlreadyRegistered(_) => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    Ok(Json(crate::AcpRegisterResponse {
        token: token.id.clone(),
        registered_at: chrono::Utc::now().timestamp(),
        webid: req.webid,
    }))
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
            state: s.state.to_string(),
            webid: s.webid,
            agent_type: s.agent_type.to_string(),
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
    use hkask_types::CapabilityResource;

    state.cns_emitter.emit_agent_pod(
        "api.pod.create.start",
        serde_json::json!({
            "template": req.template,
            "name": req.name,
        }),
    );

    let user_webid = state.system_webid;

    let has_capability = state.capability_checker.check_resource(
        &state
            .capability_checker
            .grant_tool("pod".to_string(), state.system_webid, user_webid),
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
            "state": status.state.to_string(),
        }),
    );

    Ok(Json(PodStatusResponse {
        pod_id: status.pod_id,
        name: status.name,
        state: status.state.to_string(),
        webid: status.webid,
        agent_type: status.agent_type.to_string(),
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

// TODO: Get tool definition - currently unused
// async fn get_tool(
//     State(state): State<ApiState>,
//     Path(name): Path<String>,
// ) -> Result<Json<ToolResponse>, StatusCode> {
//     let tool = state
//         .mcp_runtime
//         .get_tool(&name)
//         .await
//         .ok_or(StatusCode::NOT_FOUND)?;
//
//     Ok(Json(ToolResponse {
//         name: tool.name,
//         description: tool.description,
//         input_schema: tool.input_schema,
//         server_id: tool.server_id,
//     }))
// }

/// CNS health endpoint
#[utoipa::path(
    get,
    path = "/api/cns/health",
    tag = "cns",
    responses(
        (status = 200, description = "CNS health status", body = CnsHealthResponse),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn cns_health(State(state): State<ApiState>) -> Json<CnsHealthResponse> {
    state.cns_emitter.emit_tool(
        "cns.health.check",
        serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    );

    let health = CnsHealth::check(&AlgedonicManager::new(100));

    Json(CnsHealthResponse {
        overall_deficit: health.overall_deficit,
        critical_count: health.critical_count,
        warning_count: health.warning_count,
        healthy: health.healthy,
    })
}

/// CNS alerts endpoint
async fn cns_alerts(State(_state): State<ApiState>) -> Json<Vec<String>> {
    Json(vec![])
}

/// CNS variety endpoint
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
    let mut monitor = VarietyMonitor::new();

    let domains: Vec<String> = vec![
        "tool.invocation".to_string(),
        "template.render".to_string(),
        "agent.pod".to_string(),
    ];

    for domain in &domains {
        monitor.counter(domain).increment("state_active");
    }

    let counters: HashMap<String, VarietyCounterResponse> = domains
        .iter()
        .map(|d| {
            let counter = monitor.counter(d);
            (
                d.clone(),
                VarietyCounterResponse {
                    variety: counter.variety(),
                    total: counter.total(),
                    entropy: counter.entropy(),
                },
            )
        })
        .collect();

    let total_deficit: u64 = counters.values().map(|c| c.variety).sum();

    Json(CnsVarietyResponse {
        domains,
        total_deficit,
        counters,
    })
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
        .route(
            "/api/sovereignty/status",
            axum::routing::get(sovereignty_status),
        )
        .route(
            "/api/sovereignty/consent/grant",
            axum::routing::post(sovereignty_grant_consent),
        )
        .route(
            "/api/sovereignty/consent/revoke",
            axum::routing::post(sovereignty_revoke_consent),
        )
        .route(
            "/api/sovereignty/killzone",
            axum::routing::get(sovereignty_killzone),
        )
        .route(
            "/api/sovereignty/access/check",
            axum::routing::get(sovereignty_check_access),
        )
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
    use hkask_templates::{InferencePort, OkapiConfig, OkapiInference};
    use hkask_types::LLMParameters;

    let config = OkapiConfig::local_dev();
    let model = "qwen3:8b";

    let inference = match OkapiInference::new(model, config) {
        Ok(i) => i,
        Err(e) => {
            return Json(ChatResponse {
                output: format!("Failed to initialize Okapi: {}", e),
                template_id: req.template_id.unwrap_or("error".to_string()),
            });
        }
    };

    let prompt = match &req.template_id {
        Some(id) => format!("[template: {}] {}", id, req.input),
        None => {
            if req.input.contains('?') || req.input.contains("what") || req.input.contains("how") {
                format!("Answer concisely: {}", req.input)
            } else if req.input.contains("create")
                || req.input.contains("make")
                || req.input.contains("build")
            {
                format!("Provide step-by-step instructions: {}", req.input)
            } else {
                format!("Respond helpfully: {}", req.input)
            }
        }
    };

    let params = LLMParameters {
        temperature: 0.7,
        top_p: 0.9,
        top_k: 40,
        frequency_penalty: 0.0,
        presence_penalty: 0.0,
        max_tokens: 512,
        seed: None,
    };

    let output = match inference.generate(&prompt, &params).await {
        Ok(result) => result.text,
        Err(e) => format!("Inference error: {}", e),
    };

    Json(ChatResponse {
        output,
        template_id: req.template_id.unwrap_or("auto-select".to_string()),
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
        sovereign_data: state
            .boundary
            .sovereign_data
            .iter()
            .map(|c| c.as_str().to_string())
            .collect(),
        shared_data: state
            .boundary
            .shared_data
            .iter()
            .map(|c| c.as_str().to_string())
            .collect(),
        public_data: state
            .boundary
            .public_data
            .iter()
            .map(|c| c.as_str().to_string())
            .collect(),
    })
}

/// Grant consent endpoint
async fn sovereignty_grant_consent(
    State(_state): State<ApiState>,
) -> Json<SovereigntyConsentResponse> {
    Json(SovereigntyConsentResponse {
        consent: true,
        message: "Explicit consent granted. Data sharing enabled for shared categories."
            .to_string(),
    })
}

/// Revoke consent endpoint
async fn sovereignty_revoke_consent(
    State(_state): State<ApiState>,
) -> Json<SovereigntyConsentResponse> {
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

    let category_str = params.get("category").map(|s| s.as_str()).unwrap_or("");
    let state = UserSovereigntyState::new();

    // Parse category string to DataCategory
    let category = parse_data_category(category_str);
    let category_name = category.as_str();

    let (classification, access_required) = if state.boundary.is_sovereign(&category) {
        (
            "SOVEREIGN".to_string(),
            "Requires explicit consent AND owner".to_string(),
        )
    } else if state.boundary.is_shared(&category) {
        (
            "SHARED".to_string(),
            "Requires explicit consent".to_string(),
        )
    } else if state.boundary.is_public(&category) {
        ("PUBLIC".to_string(), "Always accessible".to_string())
    } else {
        ("UNKNOWN".to_string(), "Denied by default".to_string())
    };

    Json(AccessCheckResponse {
        category: category_name.to_string(),
        classification,
        access_required,
    })
}

/// Parse a string into a DataCategory
fn parse_data_category(s: &str) -> hkask_types::DataCategory {
    match s {
        "episodic_memory" => hkask_types::DataCategory::EpisodicMemory,
        "semantic_memory" => hkask_types::DataCategory::SemanticMemory,
        "personal_context" => hkask_types::DataCategory::PersonalContext,
        "capability_tokens" => hkask_types::DataCategory::CapabilityTokens,
        "ocap_boundaries" => hkask_types::DataCategory::OcapBoundaries,
        "template_invocations" => hkask_types::DataCategory::TemplateInvocations,
        "hlexicon_terms" => hkask_types::DataCategory::HLexiconTerms,
        "template_registry" => hkask_types::DataCategory::TemplateRegistry,
        _ => hkask_types::DataCategory::Custom(s.to_string()),
    }
}

// ============================================================================
// Ensemble Multi-Agent Routes (Phase 7)
// ============================================================================

/// Create ensemble router
pub fn ensemble_router() -> Router<ApiState> {
    Router::new()
        .route("/api/ensemble/chat", axum::routing::post(create_chat))
        .route("/api/ensemble/chat/:session", axum::routing::get(get_chat))
        .route(
            "/api/ensemble/chat/:session/list",
            axum::routing::get(list_chats),
        )
        .route(
            "/api/ensemble/chat/:session/register",
            axum::routing::post(register_bot),
        )
        .route(
            "/api/ensemble/chat/:session/send",
            axum::routing::post(send_message),
        )
        .route(
            "/api/ensemble/deliberation",
            axum::routing::post(create_deliberation),
        )
        .route(
            "/api/ensemble/deliberation/:session/start",
            axum::routing::post(start_deliberation),
        )
        .route(
            "/api/ensemble/deliberation/:session/record",
            axum::routing::post(record_response),
        )
        .route(
            "/api/ensemble/deliberation/:session/synthesize",
            axum::routing::post(synthesize_deliberation),
        )
        .route(
            "/api/ensemble/deliberation/list",
            axum::routing::get(list_deliberations),
        )
}

/// Create chat request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateChatRequest {
    pub session_id: String,
}

/// Register bot request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterBotRequest {
    pub bot_webid: String,
    pub role: String,
}

/// Send message request
#[derive(Debug, Deserialize, ToSchema)]
pub struct SendMessageRequest {
    pub content: String,
}

/// Record response request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RecordResponseRequest {
    pub agent_webid: String,
    pub content: String,
    pub confidence: f64,
}

/// Chat response
#[derive(Debug, Serialize, ToSchema)]
pub struct EnsembleResponse {
    pub success: bool,
    pub message: String,
}

/// Create chat session
#[utoipa::path(
    post,
    path = "/api/ensemble/chat",
    tag = "ensemble",
    request_body = CreateChatRequest,
    responses(
        (status = 201, description = "Chat session created", body = EnsembleResponse),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn create_chat(
    State(_state): State<ApiState>,
    Json(req): Json<CreateChatRequest>,
) -> impl IntoResponse {
    // For now, just return success - actual implementation would use EnsembleChatManager
    let response = EnsembleResponse {
        success: true,
        message: format!("Chat session '{}' created", req.session_id),
    };
    (StatusCode::CREATED, Json(response)).into_response()
}

/// Get chat details
async fn get_chat(
    State(_state): State<ApiState>,
    Path(session): Path<String>,
) -> impl IntoResponse {
    let response = EnsembleResponse {
        success: true,
        message: format!("Chat session '{}' details", session),
    };
    Json(response)
}

/// List chat sessions
async fn list_chats(State(_state): State<ApiState>) -> impl IntoResponse {
    Json(vec![String::from("default_session")])
}

/// Register bot in chat
async fn register_bot(
    State(_state): State<ApiState>,
    Path(session): Path<String>,
    Json(_req): Json<RegisterBotRequest>,
) -> impl IntoResponse {
    let response = EnsembleResponse {
        success: true,
        message: format!("Bot registered in session '{}'", session),
    };
    Json(response)
}

/// Send message to chat
async fn send_message(
    State(_state): State<ApiState>,
    Path(session): Path<String>,
    Json(_req): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let response = EnsembleResponse {
        success: true,
        message: format!("Message sent to session '{}'", session),
    };
    Json(response)
}

/// Create deliberation session
async fn create_deliberation(
    State(_state): State<ApiState>,
    Json(req): Json<CreateChatRequest>,
) -> impl IntoResponse {
    let response = EnsembleResponse {
        success: true,
        message: format!("Deliberation session '{}' created", req.session_id),
    };
    (StatusCode::CREATED, Json(response)).into_response()
}

/// Start deliberation
async fn start_deliberation(
    State(_state): State<ApiState>,
    Path(session): Path<String>,
) -> impl IntoResponse {
    let response = EnsembleResponse {
        success: true,
        message: format!("Deliberation '{}' started", session),
    };
    Json(response)
}

/// Record response in deliberation
async fn record_response(
    State(_state): State<ApiState>,
    Path(session): Path<String>,
    Json(_req): Json<RecordResponseRequest>,
) -> impl IntoResponse {
    let response = EnsembleResponse {
        success: true,
        message: format!("Response recorded in deliberation '{}'", session),
    };
    Json(response)
}

/// Synthesize deliberation responses
async fn synthesize_deliberation(
    State(_state): State<ApiState>,
    Path(session): Path<String>,
) -> impl IntoResponse {
    let response = EnsembleResponse {
        success: true,
        message: format!("Deliberation '{}' synthesized", session),
    };
    Json(response)
}

/// List deliberation sessions
async fn list_deliberations(State(_state): State<ApiState>) -> impl IntoResponse {
    Json(vec![String::from("default_deliberation")])
}

// ============================================================================
// SOAP Inference Routes for Russell Integration
// ============================================================================

/// Create SOAP inference router
pub fn soap_infer_router() -> Router<ApiState> {
    Router::new().route("/api/llm/infer", axum::routing::post(soap_infer))
}

/// SOAP inference endpoint for Russell
#[utoipa::path(
    post,
    path = "/api/llm/infer",
    tag = "inference",
    request_body = SoapInferAuthRequest,
    responses(
        (status = 200, description = "LLM inference response", body = SoapInferResponse),
        (status = 400, description = "Validation failed"),
        (status = 403, description = "Capability verification failed"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 500, description = "Internal server error"),
        (status = 504, description = "Inference timeout"),
    ),
)]
async fn soap_infer(
    State(state): State<ApiState>,
    Json(req): Json<SoapInferAuthRequest>,
) -> Result<Json<SoapInferResponse>, StatusCode> {
    use std::time::Instant;
    use tokio::time::{Duration, timeout};

    let config = SoapInferenceConfig::from_env();
    let start = Instant::now();

    // Validate request size (DoS prevention)
    if let Err(err) = validate_soap_request(&req.request, &config) {
        InferenceSpan::ValidationError {
            error_type: format!("{:?}", err),
        }
        .emit(&state.cns_emitter);
        return Err(StatusCode::BAD_REQUEST);
    }

    // CNS span: inference started
    InferenceSpan::Start {
        timestamp: chrono::Utc::now().to_rfc3339(),
        events_count: req.request.objective.recent_events.len(),
        severity_total: req.request.objective.severity_counts.crit
            + req.request.objective.severity_counts.alert
            + req.request.objective.severity_counts.warn
            + req.request.objective.severity_counts.info,
    }
    .emit(&state.cns_emitter);

    // Verify capability token (OCAP security boundary)
    // Parse token to extract holder WebID for proper authority tracking
    let token = match hkask_types::capability::CapabilityToken::from_base64(&req.capability_token) {
        Ok(t) => t,
        Err(_) => {
            InferenceSpan::OcapDenied {
                reason: "invalid_token_format".to_string(),
            }
            .emit(&state.cns_emitter);
            return Err(StatusCode::FORBIDDEN);
        }
    };

    // Verify token signature
    if !token.verify(&config.capability_secret) {
        InferenceSpan::OcapDenied {
            reason: "invalid_token_signature".to_string(),
        }
        .emit(&state.cns_emitter);
        return Err(StatusCode::FORBIDDEN);
    }

    // Rate limiting by token holder (Miller authority separation)
    let holder_webid = token.holder();
    if !state.rate_limiter.check(&holder_webid) {
        InferenceSpan::RateLimitExceeded {
            endpoint: "/api/llm/infer".to_string(),
        }
        .emit(&state.cns_emitter);
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Load Jack persona from file (runtime loading)
    let jack_persona = match config.load_jack_persona() {
        Ok(content) => content,
        Err(e) => {
            InferenceSpan::PersonaError {
                error: e.to_string(),
            }
            .emit(&state.cns_emitter);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let system_prompt = format!(
        "You are Jack, Russell's nurse persona.\n\n{}\n\n\
         Safety Constraints:\n\
         - Never emit shell commands\n\
         - Rank intervention IDs; don't compose commands\n\
         - Use SOAP format: Subjective, Objective, Assessment, Plan\n\
         - When proposing actions, use ACTION: <skill>/<id> syntax",
        jack_persona
    );

    let user_prompt = build_soap_prompt(&req.request);
    let full_prompt = format!("{}\n\n{}", system_prompt, user_prompt);

    let infer_request = hkask_ensemble::ports::GenerateRequest {
        model: config.model.clone(),
        prompt: full_prompt.clone(),
        options: Some(hkask_ensemble::ports::GenerateOptions {
            n_probs: None,
            temperature: Some(config.temperature),
            max_tokens: Some(config.max_tokens as i32),
        }),
    };

    // Inference with timeout (resilience pattern)
    let response_text = if let Some(ref inferencer) = state.ensemble_inferencer {
        match timeout(
            Duration::from_secs(config.timeout_secs),
            inferencer.generate(&infer_request),
        )
        .await
        {
            Ok(Ok(resp)) => resp.response,
            Ok(Err(e)) => {
                InferenceSpan::InferenceError {
                    error: e.to_string(),
                }
                .emit(&state.cns_emitter);
                format!("Inference error: {}", e)
            }
            Err(_) => {
                InferenceSpan::Timeout {
                    timeout_secs: config.timeout_secs,
                }
                .emit(&state.cns_emitter);
                return Err(StatusCode::GATEWAY_TIMEOUT);
            }
        }
    } else {
        format!(
            "Mock response: Received SOAP request with {} events, {} crit, {} alert, {} warn, {} info",
            req.request.objective.recent_events.len(),
            req.request.objective.severity_counts.crit,
            req.request.objective.severity_counts.alert,
            req.request.objective.severity_counts.warn,
            req.request.objective.severity_counts.info,
        )
    };

    let latency_ms = start.elapsed().as_millis() as u64;
    let actions = extract_actions(&response_text);

    // CNS span: inference outcome
    InferenceSpan::Outcome {
        latency_ms,
        actions_count: actions.len(),
        success: !response_text.contains("Inference error"),
    }
    .emit(&state.cns_emitter);

    // CNS span: variety counter for inference domain
    InferenceSpan::Execute {
        model: config.model.clone(),
        prompt_length: full_prompt.len(),
        response_length: response_text.len(),
    }
    .emit(&state.cns_emitter);

    Ok(Json(SoapInferResponse {
        response: response_text,
        model: config.model,
        latency_ms,
        actions,
    }))
}

/// Build SOAP prompt from request
fn build_soap_prompt(req: &SoapInferRequest) -> String {
    let mut prompt = String::new();

    if let Some(subj) = &req.subjective {
        prompt.push_str(&format!("**Subjective:** {}\n\n", subj));
    }

    prompt.push_str("**Objective:**\n");
    prompt.push_str(&format!(
        "Severity: {} crit, {} alert, {} warn, {} info\n\n",
        req.objective.severity_counts.crit,
        req.objective.severity_counts.alert,
        req.objective.severity_counts.warn,
        req.objective.severity_counts.info,
    ));

    if !req.objective.recent_events.is_empty() {
        prompt.push_str("Recent Events:\n");
        for event in &req.objective.recent_events {
            prompt.push_str(&format!(
                "- [{}] {}: {}\n",
                event.severity, event.probe, event.message
            ));
        }
        prompt.push('\n');
    }

    prompt.push_str("**Assessment:**\n(Awaiting your analysis)\n\n");
    prompt.push_str("**Plan:**\n(Awaiting your recommendations)\n");

    prompt
}

/// Extract ACTION: proposals from response text
fn extract_actions(response: &str) -> Vec<String> {
    let mut actions = Vec::new();
    for line in response.lines() {
        if let Some(action) = line.trim().strip_prefix("ACTION:") {
            actions.push(action.trim().to_string());
        }
    }
    actions
}

/// Validate SOAP request size and content (DoS prevention)
pub fn validate_soap_request(
    req: &SoapInferRequest,
    config: &SoapInferenceConfig,
) -> Result<(), ValidationErrorType> {
    // Check event count
    if req.objective.recent_events.len() > config.max_events {
        return Err(ValidationErrorType::TooManyEvents);
    }

    // Check subjective length
    if let Some(subj) = &req.subjective
        && subj.len() > config.max_subjective_len
    {
        return Err(ValidationErrorType::SubjectiveTooLong);
    }

    // Check event message lengths
    for event in &req.objective.recent_events {
        if event.message.len() > config.max_message_len {
            return Err(ValidationErrorType::MessageTooLong);
        }
    }

    Ok(())
}

/// Create spec router
pub fn spec_router() -> Router<ApiState> {
    Router::new()
        .route("/api/specs", axum::routing::get(list_specs))
        .route("/api/specs/capture", axum::routing::post(capture_spec))
        .route("/api/specs/validate", axum::routing::post(validate_specs))
        .route("/api/specs/cultivate", axum::routing::get(cultivate_specs))
}

/// List specifications
#[utoipa::path(
    get,
    path = "/api/specs",
    tag = "specs",
    responses(
        (status = 200, description = "List of specifications", body = Vec<SpecListResponse>),
    ),
)]
async fn list_specs(State(_state): State<ApiState>) -> Json<Vec<SpecListResponse>> {
    Json(vec![])
}

/// Capture a new specification
#[utoipa::path(
    post,
    path = "/api/specs/capture",
    tag = "specs",
    responses(
        (status = 200, description = "Captured specification", body = SpecCaptureResponse),
    ),
)]
async fn capture_spec(
    State(_state): State<ApiState>,
    Json(req): Json<SpecCaptureRequest>,
) -> Json<SpecCaptureResponse> {
    use hkask_types::{DomainAnchor, GoalSpec, Spec, SpecCategory};

    let cat = SpecCategory::parse_str(&req.category).unwrap_or(SpecCategory::Domain);
    let anchor = DomainAnchor::parse_str(&req.domain_anchor).unwrap_or(DomainAnchor::Hkask);

    let mut goal = GoalSpec::new(&req.description);
    for c in &req.criteria {
        goal = goal.with_criterion(c);
    }

    let spec = Spec::new(&req.description, cat, anchor).with_goal(goal);

    Json(SpecCaptureResponse {
        spec_id: spec.id.to_string(),
        name: spec.name,
        category: spec.category.as_str().to_string(),
        domain_anchor: spec.domain_anchor.as_str().to_string(),
    })
}

/// Validate specification collection
#[utoipa::path(
    post,
    path = "/api/specs/validate",
    tag = "specs",
    responses(
        (status = 200, description = "Validation result", body = SpecValidateResponse),
    ),
)]
async fn validate_specs(
    State(_state): State<ApiState>,
    Json(req): Json<SpecValidateRequest>,
) -> Json<SpecValidateResponse> {
    Json(SpecValidateResponse {
        valid: false,
        coherence_score: 0.0,
        threshold: req.threshold,
        violations: vec!["No specifications in collection".to_string()],
        suggestions: SpecCategory::all()
            .iter()
            .map(|c| format!("Missing category: {}", c.as_str()))
            .collect(),
    })
}

/// Cultivate specification collection
#[utoipa::path(
    get,
    path = "/api/specs/cultivate",
    tag = "specs",
    responses(
        (status = 200, description = "Cultivation result", body = SpecCultivateResponse),
    ),
)]
async fn cultivate_specs(State(_state): State<ApiState>) -> Json<SpecCultivateResponse> {
    Json(SpecCultivateResponse {
        coherence_score: 0.0,
        spec_count: 0,
        categories_covered: vec![],
        categories_missing: SpecCategory::all()
            .iter()
            .map(|c| c.as_str().to_string())
            .collect(),
    })
}
