//! ACP registration and listing routes

use axum::extract::Extension;
use axum::{Json, extract::State, http::StatusCode, routing::Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
