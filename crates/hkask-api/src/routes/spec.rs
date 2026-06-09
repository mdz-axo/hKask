//! Specification management routes — MDS-aligned HTTP API
//!
//! Provides REST endpoints for spec capture, listing, coherence assessment,
//! and writing-quality checking. These routes surface the MDS §3 tool set
//! through the HTTP API surface.

use axum::{Json, extract::State, routing::Router};
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
        .route("/api/specs/coherence", axum::routing::get(get_coherence))
        .route(
            "/api/specs/{spec_id}/writing-quality",
            axum::routing::get(get_writing_quality),
        )
}

/// List specifications
#[utoipa::path(
    get,
    path = "/api/specs",
    tag = "specs",
    responses(
        (status = 200, description = "List of specifications", body = Vec<SpecListResponse>),
    ),
)]
async fn list_specs(State(_state): State<ApiState>) -> Json<Vec<SpecListResponse>> {
    Json(vec![])
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
    State(_state): State<ApiState>,
    Json(req): Json<SpecCaptureRequest>,
) -> Json<SpecCaptureResponse> {
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

    Json(SpecCaptureResponse {
        goal_id: spec.id.to_string(),
        category: spec.category.as_str().to_string(),
        domain_anchor: spec.domain_anchor.as_str().to_string(),
        requirements,
    })
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
async fn get_coherence(State(_state): State<ApiState>) -> Json<SpecCoherenceResponse> {
    Json(SpecCoherenceResponse {
        coherence_score: 0.0,
        violations: vec!["No specifications in collection".to_string()],
        suggestions: SpecCategory::all()
            .iter()
            .map(|c| format!("Missing category: {}", c.as_str()))
            .collect(),
    })
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
    State(_state): State<ApiState>,
    axum::extract::Path(_spec_id): axum::extract::Path<String>,
) -> Json<SpecWritingQualityResponse> {
    Json(SpecWritingQualityResponse {
        dimensions_passing: 0,
        meets_publication_standard: false,
    })
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
