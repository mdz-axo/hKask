//! Pod lifecycle management routes
//!
//! Delegates pod lifecycle operations to `PodService` in `hkask-services`.
//! Surface concerns (auth/capability checks, request/response DTOs) stay
//! here. Business logic (UUID parsing, error normalization) moves to the
//! service layer.

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::{Json, routing::Router};
use hkask_agents::pod::AgentPersona;
use hkask_services::{PodContext, PodService};
use hkask_types::DelegationResource;

use crate::ApiError;
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
    let ctx = PodContext::from_parts(state.service_context.pod_manager.clone());
    let pod_statuses = PodService::list_pods(&ctx).await.unwrap_or_default();

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
///
/// Auth/capability check is a surface concern — the service layer does not
/// enforce who can create pods.
async fn create_pod(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreatePodRequest>,
) -> Result<Json<CreatePodResponse>, ApiError> {
    // Surface concern: capability check stays in the API
    let has_capability = state.service_context.capability_checker.check_resource(
        &auth.token,
        &auth.webid,
        DelegationResource::Tool,
    );

    if !has_capability {
        return Err(ApiError::Forbidden {
            reason: "Insufficient capability to create pods".to_string(),
        });
    }

    let persona = AgentPersona::from_yaml(&req.persona_yaml).map_err(|e| {
        tracing::warn!("Invalid persona YAML: {}", e);
        ApiError::BadRequest {
            message: format!("Invalid persona YAML: {}", e),
        }
    })?;

    let ctx = PodContext::from_parts(state.service_context.pod_manager.clone());
    let pod_id = PodService::create_pod(&ctx, &req.template, &persona, req.name)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(CreatePodResponse { pod_id }))
}

/// Activate a pod
async fn activate_pod(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let ctx = PodContext::from_parts(state.service_context.pod_manager.clone());
    PodService::activate_pod(&ctx, &id)
        .await
        .map_err(ApiError::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Deactivate a pod
async fn deactivate_pod(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let ctx = PodContext::from_parts(state.service_context.pod_manager.clone());
    PodService::deactivate_pod(&ctx, &id)
        .await
        .map_err(ApiError::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get pod status
async fn pod_status(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<PodStatusResponse>, ApiError> {
    let ctx = PodContext::from_parts(state.service_context.pod_manager.clone());
    let status = PodService::get_pod_status(&ctx, &id)
        .await
        .map_err(ApiError::from)?;

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
