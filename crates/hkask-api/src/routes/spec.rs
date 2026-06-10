//! Specification management routes — MDS-aligned HTTP API
//!
//! Provides REST endpoints for spec capture, listing, query, coherence assessment,
//! and writing-quality checking. These routes surface the MDS §3 tool set
//! through the HTTP API surface.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::Router,
};
use hkask_storage::SpecStore;
use hkask_storage::spec_types::{DomainAnchor, GoalSpec, Spec, SpecCategory};

use crate::ApiState;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Spec capture request — MDS-aligned: uses description + context (not category/domain/criteria).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecCaptureRequest {
    pub description: String,
    pub context: Option<String>,
}

/// Spec capture response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecCaptureResponse {
    pub goal_id: String,
    pub category: String,
    pub domain_anchor: String,
    pub requirements: Vec<String>,
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
pub fn spec_router() -> Router<ApiState> {
    Router::new()
        .route("/api/specs", axum::routing::get(list_specs))
        .route("/api/specs/capture", axum::routing::post(capture_spec))
        .route("/api/specs/{spec_id}", axum::routing::get(get_spec))
        .route("/api/specs/coherence", axum::routing::get(get_coherence))
        .route(
            "/api/specs/{spec_id}/writing-quality",
            axum::routing::get(get_writing_quality),
        )
}

/// List specifications — with optional category filter
///
/// `GET /api/specs` — list all stored specs
/// `GET /api/specs?category=trust` — filter by MDS category
#[utoipa::path(
    get,
    path = "/api/specs",
    tag = "specs",
    responses(
        (status = 200, description = "List of specifications", body = Vec<SpecListResponse>),
    ),
)]
async fn list_specs(
    State(state): State<ApiState>,
    Query(query): Query<SpecListQuery>,
) -> impl IntoResponse {
    let store = &state.agent_service.spec_store;
    let specs = match query.category.as_deref() {
        Some(cat_str) => {
            match SpecCategory::parse_str(cat_str) {
                Some(cat) => match store.list_by_category(cat) {
                    Ok(s) => s,
                    Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                        "error": e.to_string()
                    }))).into_response(),
                },
                None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                    "error": format!("Unknown category: {cat_str}. Valid: domain, composition, trust, lifecycle, curation")
                }))).into_response(),
            }
        }
        None => match store.list_all() {
            Ok(s) => s,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": e.to_string()
            }))).into_response(),
        },
    };
    let response: Vec<SpecListResponse> = specs
        .into_iter()
        .map(|s| {
            let complete = s.is_complete();
            let name = s.name;
            let cat = s.category.as_str().to_string();
            let id = s.id.to_string();
            SpecListResponse {
                spec_id: id,
                name,
                category: cat,
                complete,
            }
        })
        .collect();
    Json(response).into_response()
}

/// Get a single specification by ID
///
/// `GET /api/specs/{spec_id}` — MDS §3: spec/graph/query (single-spec lookup)
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
async fn get_spec(State(state): State<ApiState>, Path(spec_id): Path<String>) -> impl IntoResponse {
    let id = match uuid::Uuid::parse_str(&spec_id) {
        Ok(u) => hkask_storage::spec_types::SpecId(u),
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Invalid spec ID: {spec_id}")
                })),
            )
                .into_response();
        }
    };
    let store = &state.agent_service.spec_store;
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
        Err(hkask_storage::spec_types::SpecError::NotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Spec not found: {spec_id}")
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Capture a new specification (MDS §3: spec/goal/capture)
#[utoipa::path(
    post,
    path = "/api/specs/capture",
    tag = "specs",
    request_body = SpecCaptureRequest,
    responses(
        (status = 200, description = "Captured specification", body = SpecCaptureResponse),
    ),
)]
async fn capture_spec(
    State(state): State<ApiState>,
    Json(req): Json<SpecCaptureRequest>,
) -> impl IntoResponse {
    let context = req.context.as_deref();
    let cat = infer_category(context);
    let anchor = DomainAnchor::Hkask;
    let mut goal = GoalSpec::new(&req.description);
    // Seed criteria from description sentences
    for sentence in req.description.split('.') {
        let trimmed = sentence.trim();
        if !trimmed.is_empty() && trimmed.len() < 200 {
            goal = goal.with_criterion(trimmed);
        }
    }
    let spec = Spec::new(&req.description, cat, anchor).with_goal(goal);
    let requirements: Vec<String> = spec
        .goals
        .iter()
        .flat_map(|g| g.criteria.iter().map(|c| c.description.clone()))
        .collect();

    let store = &state.agent_service.spec_store;
    match store.save(&spec) {
        Ok(()) => Json(SpecCaptureResponse {
            goal_id: spec.id.to_string(),
            category: spec.category.as_str().to_string(),
            domain_anchor: spec.domain_anchor.as_str().to_string(),
            requirements,
        })
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
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
async fn get_coherence(State(state): State<ApiState>) -> impl IntoResponse {
    let store = &state.agent_service.spec_store;
    let specs = match store.list_all() {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": e.to_string()
                })),
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

    // Compute category coverage: each of 5 categories should have ≥1 spec
    let category_coverage: std::collections::HashSet<SpecCategory> =
        specs.iter().map(|s| s.category).collect();
    let missing_categories: Vec<String> = SpecCategory::all()
        .iter()
        .filter(|c| !category_coverage.contains(c))
        .map(|c| format!("Missing category: {}", c.as_str()))
        .collect();
    let covered = SpecCategory::all().len() - missing_categories.len();
    let coherence_score = covered as f64 / SpecCategory::all().len() as f64;

    Json(SpecCoherenceResponse {
        coherence_score,
        violations: missing_categories,
        suggestions: if coherence_score < 1.0 {
            vec!["Add at least one specification per MDS category".to_string()]
        } else {
            vec![]
        },
    })
    .into_response()
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
async fn get_writing_quality(
    State(state): State<ApiState>,
    Path(spec_id): Path<String>,
) -> impl IntoResponse {
    let id = match uuid::Uuid::parse_str(&spec_id) {
        Ok(u) => hkask_storage::spec_types::SpecId(u),
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Invalid spec ID: {spec_id}")
                })),
            )
                .into_response();
        }
    };
    let store = &state.agent_service.spec_store;
    match store.load(id) {
        Ok(spec) => {
            // Writing quality: dimensions based on spec completeness signals
            let dimensions = [
                ("has_name", !spec.name.is_empty()),
                ("has_category", true),
                (
                    "has_criteria",
                    !spec.goals.iter().all(|g| g.criteria.is_empty()),
                ),
                ("has_completeness", spec.is_complete()),
            ];
            let dimensions_passing = dimensions.iter().filter(|(_, pass)| *pass).count();
            Json(SpecWritingQualityResponse {
                dimensions_passing,
                meets_publication_standard: dimensions_passing == dimensions.len(),
            })
            .into_response()
        }
        Err(hkask_storage::spec_types::SpecError::NotFound(_)) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Spec not found: {spec_id}")
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Infer spec category from context string keywords (matches MCP server logic).
fn infer_category(context: Option<&str>) -> SpecCategory {
    let ctx = match context {
        Some(c) => c.to_lowercase(),
        None => return SpecCategory::Domain,
    };
    if ctx.contains("trust") || ctx.contains("security") || ctx.contains("threat") {
        SpecCategory::Trust
    } else if ctx.contains("compose") || ctx.contains("interface") || ctx.contains("api") {
        SpecCategory::Composition
    } else if ctx.contains("lifecycle") || ctx.contains("bootstrap") || ctx.contains("evolve") {
        SpecCategory::Lifecycle
    } else if ctx.contains("curat") || ctx.contains("review") || ctx.contains("coherence") {
        SpecCategory::Curation
    } else {
        SpecCategory::Domain
    }
}
