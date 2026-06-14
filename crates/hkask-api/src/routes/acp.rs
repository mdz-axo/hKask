//! ACP registration and listing routes
//!
//! # Service layer depth test
//!
//! AcpService was considered but **rejected** as shallow: every handler is a thin
//! delegation to `AcpRuntime` methods (`register_agent`, `list_agents`,
//! `unregister_agent`) plus HTTP response mapping. No CLI ACP commands exist.
//! An AcpService would just be `self.acp_runtime().register_agent()` — a pure
//! pass-through that increases interface cost without adding behavior.
//!
//! Decision: Guideline — keep direct `service_context.pod_manager().acp_runtime()` access.
//! Revisit if ACP policy logic (e.g., capability scoping, agent tier enforcement)
//! grows beyond simple registration/delegation.

use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use axum::{Json, extract::State};
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ApiError;
use crate::ApiState;
use crate::middleware::AuthContext;

/// Parse a WebID from a string, returning a structured error on failure.
fn parse_webid(raw: &str) -> Result<WebID, ServiceErrorResponse> {
    uuid::Uuid::parse_str(raw)
        .map(WebID::from_uuid)
        .map_err(|_| ApiError::BadRequest {
            message: format!("Invalid WebID format: {}", raw),
        })
}

/// ACP registration request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AcpRegisterRequest {
    /// Agent WebID (UUID string)
    pub webid: String,
    /// Agent type: "Bot" or "Replicant"
    pub agent_type: String,
    /// Capabilities to grant (e.g., ["tool:execute", "template:render"])
    pub capabilities: Vec<String>,
}

/// ACP registration response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AcpRegisterResponse {
    /// Granted capability token (HMAC-signed)
    pub token: String,
    /// Registration timestamp (Unix epoch seconds)
    pub registered_at: i64,
    /// Agent WebID
    pub webid: String,
}

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
pub fn acp_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .route("/api/v1/acp/register", axum::routing::post(acp_register))
        .routes(routes!(acp_list_agents))
        .routes(routes!(acp_unregister_agent))
}

/// Register an agent with the ACP runtime
async fn acp_register(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Json(req): Json<AcpRegisterRequest>,
) -> Result<Json<AcpRegisterResponse>, ServiceErrorResponse> {
    let webid = parse_webid(&req.webid)?;

    let agent_kind =
        hkask_types::AgentKind::parse(&req.agent_type).ok_or_else(|| ApiError::BadRequest {
            message: format!(
                "Agent type must be 'Bot' or 'Replicant', got: {}",
                req.agent_type
            ),
        })?;

    if req.capabilities.is_empty() {
        return Err(ApiError::BadRequest {
            message: "At least one capability is required".to_string(),
        });
    }

    let acp = state.agent_service.pod_manager().acp_runtime();
    let token = acp
        .register_agent(webid, agent_kind, req.capabilities)
        .await
        .map_err(|e| ApiError::from(hkask_services::ServiceError::Acp(e)))?;

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
pub(crate) async fn acp_list_agents(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
) -> Result<Json<AgentListResponse>, ServiceErrorResponse> {
    let acp = state.agent_service.pod_manager().acp_runtime();
    let agents = acp.list_agents().await;

    let agent_responses: Vec<AcpAgentResponse> = agents
        .into_iter()
        .map(|a| AcpAgentResponse {
            webid: a.webid.to_string(),
            agent_type: a.agent_type.to_string(),
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
    params(
        ("agent_id" = String, Path, description = "ACP agent ID to unregister"),
    ),
    responses(
        (status = 200, description = "Agent unregistered"),
        (status = 400, description = "Invalid agent ID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Agent not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn acp_unregister_agent(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(agent_id): Path<String>,
) -> Result<StatusCode, ServiceErrorResponse> {
    use axum::http::StatusCode;

    let webid = parse_webid(&agent_id)?;

    let acp = state.agent_service.pod_manager().acp_runtime();
    acp.unregister_agent(&webid)
        .await
        .map_err(|e| ApiError::from(hkask_services::ServiceError::Acp(e)))?;

    Ok(StatusCode::NO_CONTENT)
}
