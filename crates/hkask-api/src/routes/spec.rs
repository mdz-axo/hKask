//! Specification management routes

use axum::{Json, extract::State, routing::Router};
use hkask_storage::spec_types::{DomainAnchor, GoalSpec, Spec, SpecCategory};

use crate::ApiState;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Spec capture request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecCaptureRequest {
    pub description: String,
    pub category: String,
    pub domain_anchor: String,
    pub criteria: Vec<String>,
}

/// Spec capture response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecCaptureResponse {
    pub spec_id: String,
    pub name: String,
    pub category: String,
    pub domain_anchor: String,
}

/// Spec list response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecListResponse {
    pub spec_id: String,
    pub name: String,
    pub category: String,
    pub complete: bool,
}

/// Spec validate request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecValidateRequest {
    pub threshold: f64,
}

/// Spec validate response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecValidateResponse {
    pub valid: bool,
    pub coherence_score: f64,
    pub threshold: f64,
    pub violations: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Spec cultivate response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpecCultivateResponse {
    pub coherence_score: f64,
    pub spec_count: usize,
    pub categories_covered: Vec<String>,
    pub categories_missing: Vec<String>,
}

/// Create spec router
pub fn spec_router() -> Router<ApiState> {
    Router::new()
        .route("/api/specs", axum::routing::get(list_specs))
        .route("/api/specs/capture", axum::routing::post(capture_spec))
        .route("/api/specs/validate", axum::routing::post(validate_specs))
        .route("/api/specs/cultivate", axum::routing::get(cultivate_specs))
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

/// Capture a new specification
#[utoipa::path(
    post,
    path = "/api/specs/capture",
    tag = "specs",
    responses(
        (status = 200, description = "Captured specification", body = SpecCaptureResponse),
    ),
)]
async fn capture_spec(
    State(_state): State<ApiState>,
    Json(req): Json<SpecCaptureRequest>,
) -> Json<SpecCaptureResponse> {
    let spec = {
        let cat = SpecCategory::parse_str(&req.category).unwrap_or(SpecCategory::Domain);
        let anchor = DomainAnchor::parse_str(&req.domain_anchor).unwrap_or(DomainAnchor::Hkask);
        let mut goal = GoalSpec::new(&req.description);
        for c in &req.criteria {
            goal = goal.with_criterion(c);
        }
        Spec::new(&req.description, cat, anchor).with_goal(goal)
    };

    Json(SpecCaptureResponse {
        spec_id: spec.id.to_string(),
        name: spec.name,
        category: spec.category.as_str().to_string(),
        domain_anchor: spec.domain_anchor.as_str().to_string(),
    })
}

/// Validate specification collection
#[utoipa::path(
    post,
    path = "/api/specs/validate",
    tag = "specs",
    responses(
        (status = 200, description = "Validation result", body = SpecValidateResponse),
    ),
)]
async fn validate_specs(
    State(_state): State<ApiState>,
    Json(req): Json<SpecValidateRequest>,
) -> Json<SpecValidateResponse> {
    Json(SpecValidateResponse {
        valid: false,
        coherence_score: 0.0,
        threshold: req.threshold,
        violations: vec!["No specifications in collection".to_string()],
        suggestions: SpecCategory::all()
            .iter()
            .map(|c| format!("Missing category: {}", c.as_str()))
            .collect(),
    })
}

/// Cultivate specification collection
#[utoipa::path(
    get,
    path = "/api/specs/cultivate",
    tag = "specs",
    responses(
        (status = 200, description = "Cultivation result", body = SpecCultivateResponse),
    ),
)]
async fn cultivate_specs(State(_state): State<ApiState>) -> Json<SpecCultivateResponse> {
    Json(SpecCultivateResponse {
        coherence_score: 0.0,
        spec_count: 0,
        categories_covered: vec![],
        categories_missing: SpecCategory::all()
            .iter()
            .map(|c| c.as_str().to_string())
            .collect(),
    })
}
