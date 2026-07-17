//! Pod lifecycle management routes — call PodManager directly.

use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use hkask_capability::DelegationResource;
use hkask_services_core::{DomainKind, ErrorKind, ServiceError};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
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

/// expect: "API endpoints enforce OCAP boundaries"
/// pre:  none
/// post: returns OpenApi`Router<ApiState>` with pod routes registered
pub fn pods_router() -> OpenApiRouter<ApiState> {
    use utoipa_axum::router::OpenApiRouter;
    OpenApiRouter::new()
        .routes(routes!(list_pods))
        .routes(routes!(create_pod))
        .routes(routes!(activate_pod))
        .routes(routes!(deactivate_pod))
        .routes(routes!(pod_status))
}

fn parse_pod_id(id: &str) -> Result<hkask_agents::pod::PodID, ServiceError> {
    use hkask_agents::pod::PodID;
    Uuid::parse_str(id).map(PodID::from_uuid).map_err(|e| {
        let msg = format!("Invalid pod ID: {e}");
        ServiceError::Domain {
            kind: ErrorKind::BadRequest,
            domain: DomainKind::User,
            source: Some(Box::new(e)),
            message: msg,
        }
    })
}

#[utoipa::path(
    get,
    path = "/api/pods",
    tag = "pods",
    responses(
        (status = 200, description = "List all active pods", body = ListPodsResponse),
    ),
)]
async fn list_pods(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
) -> Json<ListPodsResponse> {
    // P9: CNS span
    tracing::info!(target: "hkask.api", operation = "pods_list", "CNS");
    let pod_statuses = state
        .agent_service
        .infra()
        .pods
        .clone()
        .list_pods()
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

#[utoipa::path(
    post,
    path = "/api/pods",
    tag = "pods",
    request_body = CreatePodRequest,
    responses(
        (status = 200, description = "Pod created", body = CreatePodResponse),
        (status = 400, description = "Invalid request"),
        (status = 403, description = "Insufficient capability"),
    ),
)]
async fn create_pod(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreatePodRequest>,
) -> Result<Json<CreatePodResponse>, ServiceErrorResponse> {
    tracing::info!(target: "hkask.api", operation = "pods_create", "CNS");

    let token = auth.token.as_ref().ok_or_else(|| ServiceError::Domain {
        kind: ErrorKind::BadRequest,
        domain: DomainKind::Agent,
        source: None,
        message: "Session auth not supported for pod creation".to_string(),
    })?;
    let has = state.agent_service.governance().checker.check_resource(
        token,
        &auth.webid,
        DelegationResource::Tool,
    );
    if !has {
        return Err(ServiceError::Domain {
            kind: ErrorKind::BadRequest,
            domain: DomainKind::Agent,
            source: None,
            message: hkask_agents::a2a::A2AError::CapabilityDenied(
                auth.webid,
                "Insufficient capability to create pods".into(),
            )
            .to_string(),
        }
        .into());
    }

    let persona = hkask_agents::pod::AgentPersona::from_yaml(&req.persona_yaml).map_err(|e| {
        let msg = format!("Invalid persona YAML: {e}");
        ServiceError::Domain {
            kind: ErrorKind::BadRequest,
            domain: DomainKind::User,
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;
    let pod_id = state
        .agent_service
        .infra()
        .pods
        .clone()
        .create_pod(
            &req.template,
            &persona,
            req.name,
            hkask_agents::pod::PodKind::Replicant,
        )
        .await?;

    Ok(Json(CreatePodResponse {
        pod_id: pod_id.to_string(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/pods/{id}/activate",
    tag = "pods",
    params(
        ("id" = String, Path, description = "Pod ID"),
    ),
    responses(
        (status = 204, description = "Pod activated"),
        (status = 400, description = "Invalid pod ID"),
    ),
)]
async fn activate_pod(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<StatusCode, ServiceErrorResponse> {
    // P9: CNS span
    tracing::info!(target: "hkask.api", operation = "pods_activate", pod_id = %id, "CNS");
    let pid = parse_pod_id(&id)?;
    state
        .agent_service
        .infra()
        .pods
        .clone()
        .activate_pod(&pid)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/pods/{id}/deactivate",
    tag = "pods",
    params(
        ("id" = String, Path, description = "Pod ID"),
    ),
    responses(
        (status = 204, description = "Pod deactivated"),
        (status = 400, description = "Invalid pod ID"),
    ),
)]
async fn deactivate_pod(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<StatusCode, ServiceErrorResponse> {
    // P9: CNS span
    tracing::info!(target: "hkask.api", operation = "pods_deactivate", pod_id = %id, "CNS");
    let pid = parse_pod_id(&id)?;
    state
        .agent_service
        .infra()
        .pods
        .clone()
        .deactivate_pod(&pid)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/pods/{id}/status",
    tag = "pods",
    params(
        ("id" = String, Path, description = "Pod ID"),
    ),
    responses(
        (status = 200, description = "Pod status", body = PodStatusResponse),
        (status = 400, description = "Invalid pod ID"),
    ),
)]
async fn pod_status(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
) -> Result<Json<PodStatusResponse>, ServiceErrorResponse> {
    // P9: CNS span
    tracing::info!(target: "hkask.api", operation = "pods_status", pod_id = %id, "CNS");
    let pid = parse_pod_id(&id)?;
    let status = state
        .agent_service
        .infra()
        .pods
        .clone()
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
