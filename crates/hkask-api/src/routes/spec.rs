//! Specification management routes — thin passthrough to SpecStore.
//!
//! All business logic lives in `hkask-storage::spec_types` and `SpecStore`.
//! These routes read/write specs directly; spec validation is handled by
//! the QA system (`kask qa spec-check`).

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use hkask_storage::SpecStore;
use hkask_storage::spec_types::{DomainAnchor, GoalSpec, Spec, SpecCategory, infer_spec_category};
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

/// Spec list response — summary of a single MDS specification.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecListResponse {
    pub spec_id: String,
    pub name: String,
    pub category: String,
    pub complete: bool,
}

/// Spec detail response — full details for a single MDS specification.
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

/// Spec coherence response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecCoherenceResponse {
    pub coherence_score: f64,
    pub violations: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Spec writing quality response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecWritingQualityResponse {
    pub dimensions_passing: usize,
    pub meets_publication_standard: bool,
}

/// Create spec router
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
    let store = state.agent_service.spec_store();
    let result = match query.category.as_deref() {
        Some(cat_str) => {
            let cat = SpecCategory::parse_str(cat_str).unwrap_or(SpecCategory::Domain);
            store.list_by_category(cat)
        }
        None => store.list_all(),
    };
    match result {
        Ok(specs) => {
            let response: Vec<SpecListResponse> = specs
                .into_iter()
                .map(|s| {
                    let complete = s.is_complete();
                    SpecListResponse {
                        spec_id: s.id.to_string(),
                        name: s.name,
                        category: s.category.as_str().to_string(),
                        complete,
                    }
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
    match parse_spec_id(&spec_id) {
        Ok(id) => {
            let store = state.agent_service.spec_store();
            match store.load(id) {
                Ok(spec) => {
                    let requirements: Vec<String> = spec
                        .goals
                        .iter()
                        .flat_map(|g| g.criteria.iter().map(|c| c.description.clone()))
                        .collect();
                    Json(SpecDetailResponse {
                        spec_id: spec.id.to_string(),
                        name: spec.name,
                        category: spec.category.as_str().to_string(),
                        domain_anchor: spec.domain_anchor.as_str().to_string(),
                        requirements,
                    })
                    .into_response()
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
                    .into_response(),
            }
        }
        Err(msg) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": msg })),
        )
            .into_response(),
    }
}

/// Capture a new specification
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
    let cat = infer_spec_category(req.context.as_deref());

    let mut goal = GoalSpec::new(&req.description);
    for sentence in req.description.split('.') {
        let trimmed = sentence.trim();
        if !trimmed.is_empty() && trimmed.len() < 200 {
            goal = goal.with_criterion(trimmed);
        }
    }

    let spec = Spec::new(&req.description, cat, DomainAnchor::Hkask).with_goal(goal);
    let store = state.agent_service.spec_store();
    match store.save(&spec) {
        Ok(()) => Json(serde_json::json!({
            "goal_id": spec.id.to_string(),
            "category": spec.category.as_str(),
            "domain_anchor": spec.domain_anchor.as_str(),
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

/// Get specification collection coherence
#[utoipa::path(
    get,
    path = "/api/specs/coherence",
    tag = "specs",
    responses(
        (status = 200, description = "Coherence assessment", body = SpecCoherenceResponse),
    ),
)]
pub(crate) async fn get_coherence(State(state): State<ApiState>) -> impl IntoResponse {
    let store = state.agent_service.spec_store();
    let specs = match store.list_all() {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    if specs.is_empty() {
        return Json(SpecCoherenceResponse {
            coherence_score: 0.0,
            violations: vec!["No specifications in collection".to_string()],
            suggestions: SpecCategory::all()
                .iter()
                .map(|c| format!("Missing category: {}", c.as_str()))
                .collect(),
        })
        .into_response();
    }

    let covered: std::collections::HashSet<SpecCategory> =
        specs.iter().map(|s| s.category).collect();
    let missing: Vec<String> = SpecCategory::all()
        .iter()
        .filter(|c| !covered.contains(c))
        .map(|c| format!("Missing category: {}", c.as_str()))
        .collect();
    let covered_count = SpecCategory::all().len() - missing.len();
    let coherence_score = covered_count as f64 / SpecCategory::all().len() as f64;

    Json(SpecCoherenceResponse {
        coherence_score,
        violations: missing,
        suggestions: if coherence_score < 1.0 {
            vec!["Add at least one specification per MDS category".to_string()]
        } else {
            vec![]
        },
    })
    .into_response()
}

/// Get writing quality assessment for a spec
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
    let id = match parse_spec_id(&spec_id) {
        Ok(id) => id,
        Err(msg) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": msg })),
            )
                .into_response();
        }
    };
    let store = state.agent_service.spec_store();
    let spec = match store.load(id) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let dimensions = [
        !spec.name.is_empty(),
        true, // has_category
        !spec.goals.iter().all(|g| g.criteria.is_empty()),
        spec.is_complete(),
    ];
    let dimensions_passing = dimensions.iter().filter(|&&p| p).count();

    Json(SpecWritingQualityResponse {
        dimensions_passing,
        meets_publication_standard: dimensions_passing == dimensions.len(),
    })
    .into_response()
}

fn parse_spec_id(s: &str) -> Result<hkask_storage::spec_types::SpecId, String> {
    uuid::Uuid::parse_str(s)
        .map(hkask_storage::spec_types::SpecId)
        .map_err(|_| format!("Invalid spec ID '{}'", s))
}
