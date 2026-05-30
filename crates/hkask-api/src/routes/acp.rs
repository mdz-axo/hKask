//! ACP registration and listing routes

use axum::extract::{Extension, Path};
use axum::{Json, extract::State, http::StatusCode, routing::Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use hkask_types::{Phase, Span};

use crate::middleware::AuthContext;
use crate::{AcpRegisterRequest, AcpRegisterResponse, ApiState, ErrorResponse};

/// ACP agent response
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AcpAgentResponse {
    pub webid: String,
    pub agent_type: String,
    pub capabilities: Vec<String>,
    pub registered_at: i64,
    pub active: bool,
}

/// Agent list response
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AgentListResponse {
    pub agents: Vec<AcpAgentResponse>,
}

/// Create ACP router
pub fn acp_router() -> Router<ApiState> {
    Router::new()
        .route("/api/v1/acp/register", axum::routing::post(acp_register))
        .route("/api/v1/acp/agents", axum::routing::get(acp_list_agents))
        .route(
            "/api/v1/acp/agents/:agent_id",
            axum::routing::delete(acp_unregister_agent),
        )
}

/// Register an agent with the ACP runtime
async fn acp_register(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Json(req): Json<AcpRegisterRequest>,
) -> Result<Json<AcpRegisterResponse>, (StatusCode, Json<ErrorResponse>)> {
    let webid = uuid::Uuid::parse_str(&req.webid)
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_webid".to_string(),
                    code: "ACP_BAD_REQUEST".to_string(),
                    details: Some(serde_json::json!({
                        "message": format!("Invalid WebID format: {}", req.webid)
                    })),
                }),
            )
        })
        .map(hkask_types::WebID)?;

    if !["Bot", "Replicant"].contains(&req.agent_type.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid_agent_type".to_string(),
                code: "ACP_BAD_REQUEST".to_string(),
                details: Some(serde_json::json!({
                    "message": format!("Agent type must be 'Bot' or 'Replicant', got: {}", req.agent_type)
                })),
            }),
        ));
    }

    if req.capabilities.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "empty_capabilities".to_string(),
                code: "ACP_BAD_REQUEST".to_string(),
                details: Some(serde_json::json!({
                    "message": "At least one capability is required"
                })),
            }),
        ));
    }

    let acp = state.pod_manager.acp_runtime();
    let token = acp
        .register_agent(webid, &req.agent_type, req.capabilities)
        .await
        .map_err(|e| match e {
            hkask_agents::AcpError::AgentAlreadyRegistered(_) => (
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: "agent_already_registered".to_string(),
                    code: "ACP_CONFLICT".to_string(),
                    details: Some(serde_json::json!({
                        "message": e.to_string()
                    })),
                }),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "registration_failed".to_string(),
                    code: "ACP_ERROR".to_string(),
                    details: Some(serde_json::json!({
                        "message": e.to_string()
                    })),
                }),
            ),
        })?;

    Ok(Json(AcpRegisterResponse {
        token: token.id.clone(),
        registered_at: chrono::Utc::now().timestamp(),
        webid: req.webid,
    }))
}

/// List all registered ACP agents
#[utoipa::path(
    get,
    path = "/api/v1/acp/agents",
    tag = "acp",
    responses(
        (status = 200, description = "List of registered agents", body = AgentListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn acp_list_agents(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
) -> Result<Json<AgentListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let acp = state.pod_manager.acp_runtime();
    let agents = acp.list_agents().await;

    let agent_responses: Vec<AcpAgentResponse> = agents
        .into_iter()
        .map(|a| AcpAgentResponse {
            webid: a.webid.to_string(),
            agent_type: a.agent_type,
            capabilities: a.capabilities,
            registered_at: a.registered_at,
            active: a.active,
        })
        .collect();

    Ok(Json(AgentListResponse {
        agents: agent_responses,
    }))
}

/// Unregister an ACP agent
#[utoipa::path(
    delete,
    path = "/api/v1/acp/agents/{agent_id}",
    tag = "acp",
    responses(
        (status = 200, description = "Agent unregistered"),
        (status = 400, description = "Invalid agent ID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Agent not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn acp_unregister_agent(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(agent_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.acp.unregister.start"),
        Phase::Observe,
        serde_json::json!({
            "agent_id": agent_id,
        }),
    );

    let webid = uuid::Uuid::parse_str(&agent_id)
        .map_err(|_| {
            state.cns_emitter.emit_with_phase(
                Span::agent_pod("api.acp.unregister.error"),
                Phase::Observe,
                serde_json::json!({ "reason": "invalid_webid" }),
            );
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_webid".to_string(),
                    code: "ACP_BAD_REQUEST".to_string(),
                    details: Some(serde_json::json!({
                        "message": format!("Invalid WebID format: {}", agent_id)
                    })),
                }),
            )
        })
        .map(hkask_types::WebID)?;

    let acp = state.pod_manager.acp_runtime();
    acp.unregister_agent(&webid).await.map_err(|e| match e {
        hkask_agents::AcpError::AgentNotFound(_) => {
            state.cns_emitter.emit_with_phase(
                Span::agent_pod("api.acp.unregister.not_found"),
                Phase::Observe,
                serde_json::json!({ "agent_id": agent_id }),
            );
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "agent_not_found".to_string(),
                    code: "ACP_NOT_FOUND".to_string(),
                    details: Some(serde_json::json!({
                        "message": format!("Agent '{}' not found", agent_id)
                    })),
                }),
            )
        }
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "unregister_failed".to_string(),
                code: "ACP_ERROR".to_string(),
                details: Some(serde_json::json!({
                    "message": e.to_string()
                })),
            }),
        ),
    })?;

    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.acp.unregister.success"),
        Phase::Observe,
        serde_json::json!({ "agent_id": agent_id }),
    );

    Ok(StatusCode::NO_CONTENT)
}
