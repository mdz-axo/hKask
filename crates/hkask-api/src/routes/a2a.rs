//! ACP registration and listing routes
//!
//! # Service layer depth test
//!
//! A2AService was considered but **rejected** as shallow: every handler is a thin
//! delegation to `A2ARuntime` methods (`register_agent`, `list_agents`,
//! `unregister_agent`) plus HTTP response mapping. No CLI ACP commands exist.
//! An A2AService would just be `self.a2a_runtime().register_agent()` — a pure
//! pass-through that increases interface cost without adding behavior.
//!
//! Decision: Guideline — keep direct `service_context.pod_manager().a2a_runtime()` access.
//! Revisit if ACP policy logic (e.g., capability scoping, agent tier enforcement)
//! grows beyond simple registration/delegation.

use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use axum::{Json, extract::State};
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ApiState;
use crate::error::ServiceErrorResponse;
use crate::middleware::AuthContext;

use hkask_services::ServiceError;

/// Parse a WebID from a string, returning a structured error on failure.
fn parse_webid(raw: &str) -> Result<WebID, ServiceError> {
    uuid::Uuid::parse_str(raw)
        .map(WebID::from_uuid)
        .map_err(|_| ServiceError::ValidationError {
            source: None,
            message: format!("Invalid WebID format: {}", raw),
        })
}

/// ACP registration request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct A2ARegisterRequest {
    /// Agent WebID (UUID string)
    pub webid: String,
    /// Agent type: "Bot" or "Replicant"
    pub agent_type: String,
    /// Capabilities to grant (e.g., ["tool:execute", "template:render"])
    pub capabilities: Vec<String>,
}

/// ACP registration response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct A2ARegisterResponse {
    /// Granted capability token (HMAC-signed)
    pub token: String,
    /// Registration timestamp (Unix epoch seconds)
    pub registered_at: i64,
    /// Agent WebID
    pub webid: String,
}

/// ACP agent response — registered agent in the ACP capability delegation system (P4 OCAP).
///
/// `agent_type` is "Bot" or "Replicant" (P10).
/// `capabilities` are the granted capability verbs this agent holds.
/// `active` indicates whether the agent is currently allowed to exercise its capabilities.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct A2AAgentResponse {
    pub webid: String,
    pub agent_type: String,
    pub capabilities: Vec<String>,
    pub registered_at: i64,
    pub active: bool,
}

/// Agent list response — all ACP-registered agents.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AgentListResponse {
    pub agents: Vec<A2AAgentResponse>,
}

/// Create ACP router
///
/// REQ: API-014
/// pre:  none
/// post: returns OpenApiRouter<ApiState> with ACP routes registered
pub fn a2a_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .route("/api/v1/acp/register", axum::routing::post(acp_register))
        .routes(routes!(acp_list_agents))
        .routes(routes!(acp_unregister_agent))
}

/// Register an agent with the A2A runtime
async fn acp_register(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Json(req): Json<A2ARegisterRequest>,
) -> Result<Json<A2ARegisterResponse>, ServiceErrorResponse> {
    let webid = parse_webid(&req.webid)?;

    let agent_kind = hkask_types::AgentKind::parse(&req.agent_type).ok_or_else(|| {
        ServiceError::InvalidAgentType {
            source: None,
            message: format!(
                "Agent type must be 'Bot' or 'Replicant', got: {}",
                req.agent_type
            ),
        }
    })?;

    if req.capabilities.is_empty() {
        return Err(ServiceError::ValidationError {
            source: None,
            message: "At least one capability is required".to_string(),
        }
        .into());
    }

    let acp = state.agent_service.pod_manager().a2a_runtime();
    let token = acp
        .register_agent(webid, agent_kind, req.capabilities)
        .await?;

    Ok(Json(A2ARegisterResponse {
        token: token.id.clone(),
        registered_at: chrono::Utc::now().timestamp(),
        webid: req.webid,
    }))
}

/// List all registered ACP agents
#[utoipa::path(
    get,
    path = "/api/v1/acp/agents",
    tag = "a2a",
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
    let acp = state.agent_service.pod_manager().a2a_runtime();
    let agents = acp.list_agents().await;

    let agent_responses: Vec<A2AAgentResponse> = agents
        .into_iter()
        .map(|a| A2AAgentResponse {
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
    tag = "a2a",
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

    let acp = state.agent_service.pod_manager().a2a_runtime();
    acp.unregister_agent(&webid).await?;

    Ok(StatusCode::NO_CONTENT)
}
