//! Pod lifecycle management routes — call PodManager directly.

use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use hkask_agents::pod::AgentPersona;
use hkask_types::DelegationResource;
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;
use crate::ApiState;
use crate::ApiError;
use crate::middleware::auth::AuthContext;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePodRequest {
    pub template: String,
    pub persona_yaml: String,
    pub name: Option<String>,
}
pub struct CreatePodResponse {
    pub pod_id: String,
pub struct PodStatusResponse {
    pub state: String,
    pub webid: String,
    pub agent_type: String,
    pub created_at: i64,
pub struct ListPodsResponse {
    pub pods: Vec<PodStatusResponse>,
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
fn parse_pod_id(id: &str) -> Result<hkask_agents::pod::PodID, ApiError> {
    use hkask_agents::pod::PodID;
    Uuid::parse_str(id)
        .map(PodID::from_uuid)
        .map_err(|e| ApiError::BadRequest {
            message: format!("Invalid pod ID: {e}"),
        })
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
        .collect();
    Json(ListPodsResponse { pods })
async fn create_pod(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreatePodRequest>,
) -> Result<Json<CreatePodResponse>, ApiError> {
    let has = state.agent_service.capability_checker().check_resource(
        &auth.token,
        &auth.webid,
        DelegationResource::Tool,
    );
    if !has {
        return Err(ApiError::Forbidden {
            reason: "Insufficient capability to create pods".into(),
        });
    }
    let persona = AgentPersona::from_yaml(&req.persona_yaml).map_err(|e| ApiError::BadRequest {
        message: format!("Invalid persona YAML: {e}"),
    })?;
    let pm = state.agent_service.pod_manager();
    let pod_id = pm
        .create_pod(&req.template, &persona, req.name)
        .map_err(|e| ApiError::from(hkask_services::ServiceError::Pod(e)))?;
    Ok(Json(CreatePodResponse {
        pod_id: pod_id.to_string(),
    }))
async fn activate_pod(
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let pid = parse_pod_id(&id)?;
    state
        .agent_service
        .pod_manager()
        .activate_pod(&pid)
    Ok(StatusCode::NO_CONTENT)
async fn deactivate_pod(
        .deactivate_pod(&pid)
async fn pod_status(
) -> Result<Json<PodStatusResponse>, ApiError> {
    let status = state
        .get_pod_status(&pid)
    Ok(Json(PodStatusResponse {
        pod_id: status.pod_id,
        name: status.name,
        state: status.state.to_string(),
        webid: status.webid,
        agent_type: status.agent_type.to_string(),
        template: status.template,
        created_at: status.created_at,
