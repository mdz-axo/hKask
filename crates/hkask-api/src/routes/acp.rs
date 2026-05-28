//! ACP registration routes

use axum::{Json, extract::State, http::StatusCode, routing::Router};

use crate::{AcpRegisterRequest, AcpRegisterResponse, ApiState};

/// ACP registration router
pub fn acp_router() -> Router<ApiState> {
    Router::new().route("/api/v1/acp/register", axum::routing::post(acp_register))
}

/// Register an agent with the ACP runtime
async fn acp_register(
    State(state): State<ApiState>,
    Json(req): Json<AcpRegisterRequest>,
) -> Result<Json<AcpRegisterResponse>, StatusCode> {
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

    Ok(Json(AcpRegisterResponse {
        token: token.id.clone(),
        registered_at: chrono::Utc::now().timestamp(),
        webid: req.webid,
    }))
}
