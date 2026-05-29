//! Curator escalation and metacognition routes

use axum::extract::Extension;
use axum::{Json, extract::Path, extract::State, http::StatusCode, routing::Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::middleware::AuthContext;
use crate::{ApiState, ErrorResponse};

// ── Request / Response types ──────────────────────────────────────────────

/// Escalation entry response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EscalationEntryResponse {
    pub id: String,
    pub template_id: String,
    pub bot_id: String,
    pub output: String,
    pub confidence: f64,
    pub retry_count: u32,
    pub error_context: String,
    pub created_at: String,
    pub status: String,
    pub resolved_at: Option<String>,
    pub resolved_by: Option<String>,
}

/// List escalations response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListEscalationsResponse {
    pub escalations: Vec<EscalationEntryResponse>,
}

/// Resolve escalation request
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResolveEscalationRequest {
    pub resolved_by: String,
}

/// Resolve escalation response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ResolveEscalationResponse {
    pub id: String,
    pub status: String,
}

/// Dismiss escalation request
#[derive(Debug, Deserialize, ToSchema)]
pub struct DismissEscalationRequest {
    pub dismissed_by: String,
}

/// Dismiss escalation response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DismissEscalationResponse {
    pub id: String,
    pub status: String,
}

/// Escalation stats response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EscalationStatsResponse {
    pub total: i64,
    pub pending: i64,
    pub in_review: i64,
    pub resolved: i64,
    pub dismissed: i64,
}

/// Bot status report response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BotStatusReportResponse {
    pub bot_name: String,
    pub status: String,
    pub last_report: Option<String>,
    pub issues: Vec<String>,
}

/// Metacognition status response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MetacognitionStatusResponse {
    pub escalation_stats: EscalationStatsResponse,
    pub bot_reports: Vec<BotStatusReportResponse>,
}

// ── Router ───────────────────────────────────────────────────────────────

/// Create curator router
pub fn curator_router() -> Router<ApiState> {
    Router::new()
        .route(
            "/api/v1/curator/escalations",
            axum::routing::get(list_escalations),
        )
        .route(
            "/api/v1/curator/escalations/:id/resolve",
            axum::routing::post(resolve_escalation),
        )
        .route(
            "/api/v1/curator/escalations/:id/dismiss",
            axum::routing::post(dismiss_escalation),
        )
        .route(
            "/api/v1/curator/metacognition",
            axum::routing::get(metacognition_status),
        )
}

// ── Handlers ─────────────────────────────────────────────────────────────

/// List pending escalations
#[utoipa::path(
    get,
    path = "/api/v1/curator/escalations",
    tag = "curator",
    responses(
        (status = 200, description = "List of pending escalations", body = ListEscalationsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn list_escalations(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
) -> Result<Json<ListEscalationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    state.cns_emitter.emit_agent_pod(
        "api.curator.escalations.list.start",
        serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    );

    let queue = state.escalation_queue.clone();
    let entries = queue.list_pending().map_err(|e| {
        state.cns_emitter.emit_agent_pod(
            "api.curator.escalations.list.error",
            serde_json::json!({ "error": e.to_string() }),
        );
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "escalation_list_failed".to_string(),
                code: "CURATOR_ERROR".to_string(),
                details: Some(serde_json::json!({ "message": e.to_string() })),
            }),
        )
    })?;

    let escalations: Vec<EscalationEntryResponse> = entries
        .into_iter()
        .map(|e| EscalationEntryResponse {
            id: e.id,
            template_id: e.template_id.to_string(),
            bot_id: e.bot_id.to_string(),
            output: e.output,
            confidence: e.confidence,
            retry_count: e.retry_count,
            error_context: e.error_context,
            created_at: e.created_at.to_rfc3339(),
            status: format!("{:?}", e.status).to_lowercase(),
            resolved_at: e.resolved_at.map(|dt| dt.to_rfc3339()),
            resolved_by: e.resolved_by,
        })
        .collect();

    state.cns_emitter.emit_agent_pod(
        "api.curator.escalations.list.success",
        serde_json::json!({ "count": escalations.len() }),
    );

    Ok(Json(ListEscalationsResponse { escalations }))
}

/// Resolve an escalation
#[utoipa::path(
    post,
    path = "/api/v1/curator/escalations/{id}/resolve",
    tag = "curator",
    request_body = ResolveEscalationRequest,
    responses(
        (status = 200, description = "Escalation resolved", body = ResolveEscalationResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Escalation not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn resolve_escalation(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(req): Json<ResolveEscalationRequest>,
) -> Result<Json<ResolveEscalationResponse>, (StatusCode, Json<ErrorResponse>)> {
    state.cns_emitter.emit_agent_pod(
        "api.curator.escalation.resolve.start",
        serde_json::json!({
            "escalation_id": id,
            "resolved_by": req.resolved_by,
        }),
    );

    let queue = state.escalation_queue.clone();

    // Verify escalation exists
    let entry = queue.get(&id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "escalation_lookup_failed".to_string(),
                code: "CURATOR_ERROR".to_string(),
                details: Some(serde_json::json!({ "message": e.to_string() })),
            }),
        )
    })?;

    if entry.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "escalation_not_found".to_string(),
                code: "CURATOR_NOT_FOUND".to_string(),
                details: Some(serde_json::json!({
                    "message": format!("Escalation '{}' not found", id)
                })),
            }),
        ));
    }

    queue.resolve(&id, &req.resolved_by).map_err(|e| {
        state.cns_emitter.emit_agent_pod(
            "api.curator.escalation.resolve.error",
            serde_json::json!({ "error": e.to_string() }),
        );
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "escalation_resolve_failed".to_string(),
                code: "CURATOR_ERROR".to_string(),
                details: Some(serde_json::json!({ "message": e.to_string() })),
            }),
        )
    })?;

    state.cns_emitter.emit_agent_pod(
        "api.curator.escalation.resolve.success",
        serde_json::json!({ "escalation_id": id }),
    );

    Ok(Json(ResolveEscalationResponse {
        id,
        status: "resolved".to_string(),
    }))
}

/// Dismiss an escalation
#[utoipa::path(
    post,
    path = "/api/v1/curator/escalations/{id}/dismiss",
    tag = "curator",
    request_body = DismissEscalationRequest,
    responses(
        (status = 200, description = "Escalation dismissed", body = DismissEscalationResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Escalation not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn dismiss_escalation(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(req): Json<DismissEscalationRequest>,
) -> Result<Json<DismissEscalationResponse>, (StatusCode, Json<ErrorResponse>)> {
    state.cns_emitter.emit_agent_pod(
        "api.curator.escalation.dismiss.start",
        serde_json::json!({
            "escalation_id": id,
            "dismissed_by": req.dismissed_by,
        }),
    );

    let queue = state.escalation_queue.clone();

    // Verify escalation exists
    let entry = queue.get(&id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "escalation_lookup_failed".to_string(),
                code: "CURATOR_ERROR".to_string(),
                details: Some(serde_json::json!({ "message": e.to_string() })),
            }),
        )
    })?;

    if entry.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "escalation_not_found".to_string(),
                code: "CURATOR_NOT_FOUND".to_string(),
                details: Some(serde_json::json!({
                    "message": format!("Escalation '{}' not found", id)
                })),
            }),
        ));
    }

    queue.dismiss(&id, &req.dismissed_by).map_err(|e| {
        state.cns_emitter.emit_agent_pod(
            "api.curator.escalation.dismiss.error",
            serde_json::json!({ "error": e.to_string() }),
        );
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "escalation_dismiss_failed".to_string(),
                code: "CURATOR_ERROR".to_string(),
                details: Some(serde_json::json!({ "message": e.to_string() })),
            }),
        )
    })?;

    state.cns_emitter.emit_agent_pod(
        "api.curator.escalation.dismiss.success",
        serde_json::json!({ "escalation_id": id }),
    );

    Ok(Json(DismissEscalationResponse {
        id,
        status: "dismissed".to_string(),
    }))
}

/// Get Curator metacognition status
#[utoipa::path(
    get,
    path = "/api/v1/curator/metacognition",
    tag = "curator",
    responses(
        (status = 200, description = "Curator metacognition status", body = MetacognitionStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn metacognition_status(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
) -> Result<Json<MetacognitionStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    state.cns_emitter.emit_agent_pod(
        "api.curator.metacognition.start",
        serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    );

    let queue = state.escalation_queue.clone();
    let stats = queue.stats().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "escalation_stats_failed".to_string(),
                code: "CURATOR_ERROR".to_string(),
                details: Some(serde_json::json!({ "message": e.to_string() })),
            }),
        )
    })?;

    let escalation_stats = EscalationStatsResponse {
        total: stats.total,
        pending: stats.pending,
        in_review: stats.in_review,
        resolved: stats.resolved,
        dismissed: stats.dismissed,
    };

    state.cns_emitter.emit_agent_pod(
        "api.curator.metacognition.success",
        serde_json::json!({
            "pending_escalations": escalation_stats.pending,
            "total_escalations": escalation_stats.total,
        }),
    );

    // Bot reports are not persisted across restarts in the current
    // MetacognitionLoop, so we return an empty list here. The route
    // wiring can be upgraded to hold a MetacognitionLoop reference
    // once the daemon lifecycle is fully integrated.
    let bot_reports: Vec<BotStatusReportResponse> = Vec::new();

    Ok(Json(MetacognitionStatusResponse {
        escalation_stats,
        bot_reports,
    }))
}
