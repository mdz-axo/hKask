//! Specification management routes — MDS-aligned HTTP API, delegates to SpecService.
//!
//! Provides REST endpoints for spec capture, listing, query, coherence assessment,
//! and writing-quality checking. All business logic moved to `hkask-services::SpecService`.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use hkask_services::SpecCaptureRequest;
use hkask_services::SpecService;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ApiState;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Spec capture request — MDS-aligned: uses description + context (not category/domain/criteria).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecCaptureRequestDto {
    pub description: String,
    pub context: Option<String>,
}

/// Spec list response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecListResponse {
    pub spec_id: String,
    pub name: String,
    pub category: String,
    pub complete: bool,
}

/// Spec detail response (single spec by ID)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecDetailResponse {
    pub spec_id: String,
    pub name: String,
    pub category: String,
    pub domain_anchor: String,
    pub requirements: Vec<String>,
}

/// Query parameters for listing specs
#[derive(Debug, Deserialize)]
pub struct SpecListQuery {
    pub category: Option<String>,
}

/// Spec coherence response — MDS §3: spec/graph/coherence
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecCoherenceResponse {
    pub coherence_score: f64,
    pub violations: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Spec writing quality response — MDS §3: spec/require/writing-quality
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecWritingQualityResponse {
    pub dimensions_passing: usize,
    pub meets_publication_standard: bool,
}

/// Create spec router
///
/// REQ: API-001
/// pre:  none
/// post: returns OpenApiRouter<ApiState> with spec routes registered
pub fn spec_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(list_specs))
        .routes(routes!(capture_spec))
        .routes(routes!(get_spec))
        .routes(routes!(get_coherence))
        .routes(routes!(get_writing_quality))
}

/// List specifications — with optional category filter
#[utoipa::path(
    get,
    path = "/api/specs",
    tag = "specs",
    responses(
        (status = 200, description = "List of specifications", body = Vec<SpecListResponse>),
    ),
)]
pub(crate) async fn list_specs(
    State(state): State<ApiState>,
    Query(query): Query<SpecListQuery>,
) -> impl IntoResponse {
    match SpecService::list(&state.agent_service, query.category.as_deref()) {
        Ok(entries) => {
            let response: Vec<SpecListResponse> = entries
                .into_iter()
                .map(|e| SpecListResponse {
                    spec_id: e.spec_id,
                    name: e.name,
                    category: e.category,
                    complete: e.complete,
                })
                .collect();
            Json(response).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Get a single specification by ID
#[utoipa::path(
    get,
    path = "/api/specs/{spec_id}",
    tag = "specs",
    params(
        ("spec_id" = String, Path, description = "Specification ID"),
    ),
    responses(
        (status = 200, description = "Specification details", body = SpecDetailResponse),
        (status = 404, description = "Spec not found"),
    ),
)]
pub(crate) async fn get_spec(
    State(state): State<ApiState>,
    Path(spec_id): Path<String>,
) -> impl IntoResponse {
    match SpecService::get_by_id(&state.agent_service, &spec_id) {
        Ok(detail) => Json(SpecDetailResponse {
            spec_id: detail.spec_id,
            name: detail.name,
            category: detail.category,
            domain_anchor: detail.domain_anchor,
            requirements: detail.requirements,
        })
        .into_response(),
        Err(hkask_services::ServiceError::ValidationError { .. }) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("Invalid spec ID: {spec_id}") })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Capture a new specification (MDS §3: spec/goal/capture)
#[utoipa::path(
    post,
    path = "/api/specs/capture",
    tag = "specs",
    request_body = SpecCaptureRequestDto,
    responses(
        (status = 200, description = "Captured specification"),
    ),
)]
pub(crate) async fn capture_spec(
    State(state): State<ApiState>,
    Json(req): Json<SpecCaptureRequestDto>,
) -> impl IntoResponse {
    let svc_req = SpecCaptureRequest {
        name_or_description: req.description,
        category: None,
        domain: None,
        criteria: None,
        context: req.context,
    };
    match SpecService::capture(&state.agent_service, svc_req) {
        Ok(resp) => Json(serde_json::json!({
            "goal_id": resp.spec_id,
            "category": resp.category,
            "domain_anchor": resp.domain_anchor,
            "requirements": Vec::<String>::new(),
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Get specification collection coherence (MDS §3: spec/graph/coherence)
#[utoipa::path(
    get,
    path = "/api/specs/coherence",
    tag = "specs",
    responses(
        (status = 200, description = "Coherence assessment", body = SpecCoherenceResponse),
    ),
)]
pub(crate) async fn get_coherence(State(state): State<ApiState>) -> impl IntoResponse {
    match SpecService::category_coverage(&state.agent_service) {
        Ok(r) => Json(SpecCoherenceResponse {
            coherence_score: r.coherence_score,
            violations: r.violations,
            suggestions: r.suggestions,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Get writing quality assessment for a spec (MDS §3: spec/require/writing-quality)
#[utoipa::path(
    get,
    path = "/api/specs/{spec_id}/writing-quality",
    tag = "specs",
    params(
        ("spec_id" = String, Path, description = "Specification ID"),
    ),
    responses(
        (status = 200, description = "Writing quality assessment", body = SpecWritingQualityResponse),
    ),
)]
pub(crate) async fn get_writing_quality(
    State(state): State<ApiState>,
    Path(spec_id): Path<String>,
) -> impl IntoResponse {
    match SpecService::structural_quality_check(&state.agent_service, &spec_id) {
        Ok(q) => Json(SpecWritingQualityResponse {
            dimensions_passing: q.dimensions_passing,
            meets_publication_standard: q.meets_publication_standard,
        })
        .into_response(),
        Err(hkask_services::ServiceError::ValidationError { .. }) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("Invalid spec ID: {spec_id}") })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
