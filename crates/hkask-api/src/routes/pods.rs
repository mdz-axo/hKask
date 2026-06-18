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

/// Create pod request — Pattern D agent creation.
///
/// `template` is a FlowDef template ID. `persona_yaml` is the agent persona
/// definition in YAML format (maps to a WordAct).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePodRequest {
    /// FlowDef template ID defining the agent's operational pattern
    pub template: String,
    /// Agent persona definition in YAML (WordAct)
    pub persona_yaml: String,
    /// Optional human-readable pod name
    pub name: Option<String>,
}

/// Create pod response — returns the new pod's ID.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePodResponse {
    /// Unique pod identifier
    pub pod_id: String,
}

/// Pod status response — current state of an agent pod (Pattern D).
///
/// `state` is one of: "active", "inactive", "error".
/// `agent_type` is one of: "Bot", "Replicant" (P10).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PodStatusResponse {
    /// Unique pod identifier
    pub pod_id: String,
    /// Human-readable pod name
    pub name: Option<String>,
    /// Pod state: "active", "inactive", or "error"
    pub state: String,
    /// Agent WebID (P12 — accountable identity)
    pub webid: String,
    /// Agent type: "Bot" or "Replicant" (P10)
    pub agent_type: String,
    /// FlowDef template ID
    pub template: String,
    /// Unix epoch seconds of pod creation
    pub created_at: i64,
}

/// List pods response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListPodsResponse {
    /// All active pods
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

async fn list_pods(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
) -> Json<ListPodsResponse> {
    // REQ: P9-CNS-SURF-030 pre: valid request post: cns.api span emitted
    // P9: CNS span
    tracing::info!(target: "cns.api", operation = "pods_list", "CNS");
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
    // REQ: P9-CNS-SURF-031 pre: valid request post: cns.api span emitted
    // P9: CNS span
    tracing::info!(target: "cns.api", operation = "pods_create", "CNS");
    let has = state.agent_service.capability_checker().check_resource(
        &auth.token,
        &auth.webid,
        DelegationResource::Tool,
    );
    if !has {
        return Err(
            ServiceError::A2A { message: hkask_agents::a2a::A2AError::CapabilityDenied(
                auth.webid,
                "Insufficient capability to create pods".into(),
            ).to_string() }
            .into(),
        );
    }
    let persona = AgentPersona::from_yaml(&req.persona_yaml).map_err(|e| ServiceError::Pod { message: e.to_string() })?;
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
    // REQ: P9-CNS-SURF-032 pre: valid request post: cns.api span emitted
    // P9: CNS span
    tracing::info!(target: "cns.api", operation = "pods_activate", pod_id = %id, "CNS");
    let pid = parse_pod_id(&id)?;
    state.agent_service.pod_manager().activate_pod(&pid).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn deactivate_pod(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<StatusCode, ServiceErrorResponse> {
    // REQ: P9-CNS-SURF-033 pre: valid request post: cns.api span emitted
    // P9: CNS span
    tracing::info!(target: "cns.api", operation = "pods_deactivate", pod_id = %id, "CNS");
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
    // REQ: P9-CNS-SURF-034 pre: valid request post: cns.api span emitted
    // P9: CNS span
    tracing::info!(target: "cns.api", operation = "pods_status", pod_id = %id, "CNS");
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
