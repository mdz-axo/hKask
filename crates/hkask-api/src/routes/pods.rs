//! Pod lifecycle management routes — call PodManager directly.

use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use hkask_agents::pod::AgentPersona;
use hkask_services::ServiceError;
use hkask_types::DelegationResource;
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;

use crate::ApiState;
use crate::error::ServiceErrorResponse;
use crate::middleware::auth::AuthContext;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePodRequest {
    pub template: String,
    pub persona_yaml: String,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePodResponse {
    pub pod_id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PodStatusResponse {
    pub pod_id: String,
    pub name: Option<String>,
    pub state: String,
    pub webid: String,
    pub agent_type: String,
    pub template: String,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListPodsResponse {
    pub pods: Vec<PodStatusResponse>,
}

/// REQ: API-004
/// pre:  none
/// post: returns OpenApiRouter<ApiState> with pod routes registered
pub fn pods_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .route("/api/pods", axum::routing::get(list_pods))
        .route("/api/pods", axum::routing::post(create_pod))
        .route("/api/pods/{id}/activate", axum::routing::post(activate_pod))
        .route(
            "/api/pods/{id}/deactivate",
            axum::routing::post(deactivate_pod),
        )
        .route("/api/pods/{id}/status", axum::routing::get(pod_status))
}

fn parse_pod_id(id: &str) -> Result<hkask_agents::pod::PodID, ServiceError> {
    use hkask_agents::pod::PodID;
    Uuid::parse_str(id).map(PodID::from_uuid).map_err(|e| {
        let msg = format!("Invalid pod ID: {e}");
        ServiceError::ValidationError {
            source: Some(Box::new(e)),
            message: msg,
        }
    })
}

async fn list_pods(State(state): State<ApiState>) -> Json<ListPodsResponse> {
    let pod_statuses = hkask_services::PodService::list_pods(&state.agent_service)
        .await
        .unwrap_or_default();
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
    Json(ListPodsResponse { pods })
}

async fn create_pod(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreatePodRequest>,
) -> Result<Json<CreatePodResponse>, ServiceErrorResponse> {
    let has = state.agent_service.capability_checker().check_resource(
        &auth.token,
        &auth.webid,
        DelegationResource::Tool,
    );
    if !has {
        return Err(
            ServiceError::Acp(hkask_agents::acp::AcpError::CapabilityDenied(
                auth.webid,
                "Insufficient capability to create pods".into(),
            ))
            .into(),
        );
    }
    let persona = AgentPersona::from_yaml(&req.persona_yaml).map_err(ServiceError::Pod)?;
    let pm = state.agent_service.pod_manager();
    let pod_id = pm.create_pod(&req.template, &persona, req.name).await?;
    Ok(Json(CreatePodResponse {
        pod_id: pod_id.to_string(),
    }))
}

async fn activate_pod(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<StatusCode, ServiceErrorResponse> {
    let pid = parse_pod_id(&id)?;
    state.agent_service.pod_manager().activate_pod(&pid).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn deactivate_pod(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<StatusCode, ServiceErrorResponse> {
    let pid = parse_pod_id(&id)?;
    state
        .agent_service
        .pod_manager()
        .deactivate_pod(&pid)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn pod_status(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<PodStatusResponse>, ServiceErrorResponse> {
    let pid = parse_pod_id(&id)?;
    let status = state
        .agent_service
        .pod_manager()
        .get_pod_status(&pid)
        .await?;
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
