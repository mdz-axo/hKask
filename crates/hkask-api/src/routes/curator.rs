//! Curator escalation and metacognition routes

use axum::extract::Extension;
use axum::{Json, extract::Path, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ApiError;
use crate::ApiState;
use crate::middleware::AuthContext;

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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListEscalationsResponse {
    pub escalations: Vec<EscalationEntryResponse>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResolveEscalationRequest {
    pub resolved_by: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ResolveEscalationResponse {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DismissEscalationRequest {
    pub dismissed_by: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DismissEscalationResponse {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EscalationStatsResponse {
    pub total: i64,
    pub pending: i64,
    pub resolved: i64,
    pub dismissed: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BotStatusReportResponse {
    pub bot_name: String,
    pub status: String,
    pub last_report: Option<String>,
    pub issues: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MetacognitionStatusResponse {
    pub escalation_stats: EscalationStatsResponse,
    pub bot_reports: Vec<BotStatusReportResponse>,
}

pub fn curator_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(list_escalations))
        .routes(routes!(resolve_escalation))
        .routes(routes!(dismiss_escalation))
        .routes(routes!(metacognition_status))
}

/// List all pending curator escalations.
#[utoipa::path(
    get, path = "/api/v1/curator/escalations", tag = "curator",
    responses(
        (status = 200, description = "List of pending escalations", body = ListEscalationsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn list_escalations(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
) -> Result<Json<ListEscalationsResponse>, ApiError> {
    let entries = hkask_services::CuratorService::list_escalations(&state.agent_service)
        .map_err(ApiError::from)?;
    let escalations: Vec<EscalationEntryResponse> = entries
        .into_iter()
        .map(|e| EscalationEntryResponse {
            id: e.id,
            template_id: e.template_id,
            bot_id: e.bot_id,
            output: e.output,
            confidence: e.confidence,
            retry_count: e.retry_count,
            error_context: e.error_context,
            created_at: e.created_at,
            status: e.status,
            resolved_at: e.resolved_at,
            resolved_by: e.resolved_by,
        })
        .collect();
    Ok(Json(ListEscalationsResponse { escalations }))
}

/// Resolve an escalation by marking it handled with the resolver's identity.
#[utoipa::path(
    post, path = "/api/v1/curator/escalations/{id}/resolve", tag = "curator",
    params(
        ("id" = String, Path, description = "Escalation ID"),
    ),
    request_body = ResolveEscalationRequest,
    responses(
        (status = 200, description = "Escalation resolved", body = ResolveEscalationResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Escalation not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn resolve_escalation(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(req): Json<ResolveEscalationRequest>,
) -> Result<Json<ResolveEscalationResponse>, ApiError> {
    hkask_services::CuratorService::resolve(&state.agent_service, &id, &req.resolved_by)
        .map_err(ApiError::from)?;
    Ok(Json(ResolveEscalationResponse {
        id,
        status: "resolved".into(),
    }))
}

/// Dismiss an escalation as non-actionable, recording who dismissed it.
#[utoipa::path(
    post, path = "/api/v1/curator/escalations/{id}/dismiss", tag = "curator",
    params(
        ("id" = String, Path, description = "Escalation ID"),
    ),
    request_body = DismissEscalationRequest,
    responses(
        (status = 200, description = "Escalation dismissed", body = DismissEscalationResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Escalation not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn dismiss_escalation(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(req): Json<DismissEscalationRequest>,
) -> Result<Json<DismissEscalationResponse>, ApiError> {
    hkask_services::CuratorService::dismiss(&state.agent_service, &id, &req.dismissed_by)
        .map_err(ApiError::from)?;
    Ok(Json(DismissEscalationResponse {
        id,
        status: "dismissed".into(),
    }))
}

/// Get Curator metacognition status — escalation statistics and bot health reports.
#[utoipa::path(
    get, path = "/api/v1/curator/metacognition", tag = "curator",
    responses(
        (status = 200, description = "Curator metacognition status", body = MetacognitionStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn metacognition_status(
    State(state): State<ApiState>,
    Extension(_auth): Extension<AuthContext>,
) -> Result<Json<MetacognitionStatusResponse>, ApiError> {
    let queue = state.agent_service.escalation_queue();
    let stats = queue
        .stats()
        .map_err(|e| ApiError::from(hkask_services::ServiceError::Escalation(e)))?;
    let escalation_stats = EscalationStatsResponse {
        total: stats.total,
        pending: stats.pending,
        resolved: stats.resolved,
        dismissed: stats.dismissed,
    };
    Ok(Json(MetacognitionStatusResponse {
        escalation_stats,
        bot_reports: Vec::new(),
    }))
}
