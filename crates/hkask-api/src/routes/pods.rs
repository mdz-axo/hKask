//! Pod lifecycle management routes

use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
    routing::Router,
};

use crate::ApiState;
use crate::middleware::auth::AuthContext;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Create pod request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePodRequest {
    pub template: String,
    pub persona_yaml: String,
    pub name: Option<String>,
}

/// Create pod response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePodResponse {
    pub pod_id: String,
}

/// Pod status response
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

/// List pods response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListPodsResponse {
    pub pods: Vec<PodStatusResponse>,
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

    Json(ListPodsResponse { pods })
}

/// Create a new pod
async fn create_pod(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreatePodRequest>,
) -> Result<Json<CreatePodResponse>, StatusCode> {
    use hkask_agents::pod::AgentPersona;
    use hkask_types::DelegationResource;

    let has_capability =
        state
            .capability_checker
            .check_resource(&auth.token, &auth.webid, DelegationResource::Tool);

    if !has_capability {
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(CreatePodResponse {
        pod_id: pod_id.to_string(),
    }))
}

/// Activate a pod
async fn activate_pod(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let _ = _auth; // Auth verified by middleware; handler does not use token
    use hkask_agents::pod::PodID;
    use uuid::Uuid;

    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };
    let pod_id = PodID::from_uuid(uuid);

    match state.pod_manager.activate_pod(&pod_id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Deactivate a pod
async fn deactivate_pod(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let _ = _auth; // Auth verified by middleware; handler does not use token
    use hkask_agents::pod::PodID;
    use uuid::Uuid;

    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };
    let pod_id = PodID::from_uuid(uuid);

    match state.pod_manager.deactivate_pod(&pod_id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Get pod status
async fn pod_status(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<PodStatusResponse>, StatusCode> {
    let _ = _auth; // Auth verified by middleware; handler does not use token
    use hkask_agents::pod::PodID;
    use uuid::Uuid;

    let uuid = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let pod_id = PodID::from_uuid(uuid);

    let status = state
        .pod_manager
        .get_pod_status(&pod_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

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
