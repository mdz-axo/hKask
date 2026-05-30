//! Pod lifecycle management routes

use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
    routing::Router,
};

use hkask_types::{Phase, Span};

use crate::middleware::auth::AuthContext;
use crate::{ApiState, CreatePodRequest, CreatePodResponse, ListPodsResponse, PodStatusResponse};

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
    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.pod.list.start"),
        Phase::Observe,
        serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    );

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

    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.pod.list.outcome"),
        Phase::Observe,
        serde_json::json!({
            "count": pods.len(),
        }),
    );

    Json(ListPodsResponse { pods })
}

/// Create a new pod
async fn create_pod(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreatePodRequest>,
) -> Result<Json<CreatePodResponse>, StatusCode> {
    use hkask_agents::pod::AgentPersona;
    use hkask_types::CapabilityResource;

    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.pod.create.start"),
        Phase::Observe,
        serde_json::json!({
            "template": req.template,
            "name": req.name,
        }),
    );

    let has_capability =
        state
            .capability_checker
            .check_resource(&auth.token, &auth.webid, CapabilityResource::Tool);

    if !has_capability {
        state.cns_emitter.emit_with_phase(
            Span::agent_pod("api.pod.create.denied"),
            Phase::Observe,
            serde_json::json!({
                "reason": "capability_check_failed",
            }),
        );
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
        .map_err(|e| {
            state.cns_emitter.emit_with_phase(
                Span::agent_pod("api.pod.create.error"),
                Phase::Observe,
                serde_json::json!({
                    "error": e.to_string(),
                }),
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.pod.create.success"),
        Phase::Observe,
        serde_json::json!({
            "pod_id": pod_id.to_string(),
        }),
    );

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

    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.pod.activate.start"),
        Phase::Observe,
        serde_json::json!({
            "pod_id": id,
        }),
    );

    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            state.cns_emitter.emit_with_phase(
                Span::agent_pod("api.pod.activate.error"),
                Phase::Observe,
                serde_json::json!({
                    "reason": "invalid_pod_id",
                }),
            );
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    let pod_id = PodID(uuid);

    match state.pod_manager.activate_pod(&pod_id).await {
        Ok(_) => {
            state.cns_emitter.emit_with_phase(
                Span::agent_pod("api.pod.activate.success"),
                Phase::Observe,
                serde_json::json!({
                    "pod_id": id,
                }),
            );
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            state.cns_emitter.emit_with_phase(
                Span::agent_pod("api.pod.activate.error"),
                Phase::Observe,
                serde_json::json!({
                    "reason": e.to_string(),
                }),
            );
            Err(StatusCode::NOT_FOUND)
        }
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

    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.pod.deactivate.start"),
        Phase::Observe,
        serde_json::json!({
            "pod_id": id,
        }),
    );

    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            state.cns_emitter.emit_with_phase(
                Span::agent_pod("api.pod.deactivate.error"),
                Phase::Observe,
                serde_json::json!({
                    "reason": "invalid_pod_id",
                }),
            );
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    let pod_id = PodID(uuid);

    match state.pod_manager.deactivate_pod(&pod_id).await {
        Ok(_) => {
            state.cns_emitter.emit_with_phase(
                Span::agent_pod("api.pod.deactivate.success"),
                Phase::Observe,
                serde_json::json!({
                    "pod_id": id,
                }),
            );
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            state.cns_emitter.emit_with_phase(
                Span::agent_pod("api.pod.deactivate.error"),
                Phase::Observe,
                serde_json::json!({
                    "reason": e.to_string(),
                }),
            );
            Err(StatusCode::NOT_FOUND)
        }
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

    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.pod.status.start"),
        Phase::Observe,
        serde_json::json!({
            "pod_id": id,
        }),
    );

    let uuid = Uuid::parse_str(&id).map_err(|_| {
        state.cns_emitter.emit_with_phase(
            Span::agent_pod("api.pod.status.error"),
            Phase::Observe,
            serde_json::json!({
                "reason": "invalid_pod_id",
            }),
        );
        StatusCode::BAD_REQUEST
    })?;
    let pod_id = PodID(uuid);

    let status = state
        .pod_manager
        .get_pod_status(&pod_id)
        .await
        .map_err(|e| {
            state.cns_emitter.emit_with_phase(
                Span::agent_pod("api.pod.status.error"),
                Phase::Observe,
                serde_json::json!({
                    "reason": e.to_string(),
                }),
            );
            StatusCode::NOT_FOUND
        })?;

    state.cns_emitter.emit_with_phase(
        Span::agent_pod("api.pod.status.success"),
        Phase::Observe,
        serde_json::json!({
            "pod_id": id,
            "state": status.state.to_string(),
        }),
    );

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
