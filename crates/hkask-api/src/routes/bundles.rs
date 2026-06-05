//! Bundle management routes

use axum::{
    Json, extract::Path, extract::State, http::StatusCode, response::IntoResponse, routing::Router,
};
use hkask_types::ports::BundleRegistryIndex;

use crate::ApiState;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Bundle summary for list responses
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BundleSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub visibility: String,
    pub skill_count: usize,
}

/// Compose bundle request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ComposeBundleRequest {
    /// Skill IDs to bundle
    pub skills: Vec<String>,
    /// Optional bundle name
    pub name: Option<String>,
    /// Visibility: private or shared
    #[serde(default = "default_visibility")]
    pub visibility: String,
}

fn default_visibility() -> String {
    "private".to_string()
}

/// Compose bundle response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ComposeBundleResponse {
    /// Whether an existing bundle was found matching these skills
    pub existing_match: Option<BundleSummary>,
    /// The composed bundle manifest (if composition was performed)
    pub manifest: Option<serde_json::Value>,
    /// Message about what happened
    pub message: String,
}

/// Apply bundle response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApplyBundleResponse {
    pub status: String,
    pub bundle_id: String,
    pub name: String,
    pub skill_count: usize,
}

/// Evolve bundle response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EvolveBundleResponse {
    pub evolved_manifest: Option<serde_json::Value>,
    pub changes: Vec<String>,
    pub message: String,
}

/// List bundles response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BundleListResponse {
    pub bundles: Vec<BundleSummary>,
}

/// Deactivate bundle response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeactivateBundleResponse {
    pub status: String,
}

/// Create bundles router
pub fn bundles_router() -> Router<ApiState> {
    Router::new()
        .route("/api/v1/bundles", axum::routing::get(list_bundles))
        .route(
            "/api/v1/bundles/compose",
            axum::routing::post(compose_bundle),
        )
        .route("/api/v1/bundles/:id", axum::routing::get(get_bundle))
        .route(
            "/api/v1/bundles/:id/apply",
            axum::routing::post(apply_bundle),
        )
        .route(
            "/api/v1/bundles/:id/evolve",
            axum::routing::post(evolve_bundle),
        )
        .route(
            "/api/v1/bundles/:id/deactivate",
            axum::routing::delete(deactivate_bundle),
        )
}

/// List all bundles
#[utoipa::path(
    get,
    path = "/api/v1/bundles",
    tag = "bundles",
    responses(
        (status = 200, description = "List of bundles", body = BundleListResponse),
    ),
)]
async fn list_bundles(State(state): State<ApiState>) -> Json<BundleListResponse> {
    let registry = state.registry.lock().await;

    // Collect bundles from the registry
    let bundles: Vec<BundleSummary> = registry
        .list_bundles()
        .into_iter()
        .map(|b| BundleSummary {
            id: b.id.clone(),
            name: b.name.clone(),
            description: b.description.clone(),
            version: b.version.clone(),
            visibility: b.visibility.as_str().to_string(),
            skill_count: b.skills.len(),
        })
        .collect();

    Json(BundleListResponse { bundles })
}

/// Get a specific bundle
#[utoipa::path(
    get,
    path = "/api/v1/bundles/{id}",
    tag = "bundles",
    responses(
        (status = 200, description = "Bundle manifest", body = serde_json::Value),
        (status = 404, description = "Bundle not found"),
    ),
)]
async fn get_bundle(State(state): State<ApiState>, Path(id): Path<String>) -> impl IntoResponse {
    let registry = state.registry.lock().await;
    match registry.get_bundle(&id) {
        Some(bundle) => {
            let value =
                serde_json::to_value(&bundle).unwrap_or(serde_json::json!({"id": bundle.id}));
            (StatusCode::OK, Json(value)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Bundle '{}' not found", id)})),
        )
            .into_response(),
    }
}

/// Compose a new bundle from specified skills
#[utoipa::path(
    post,
    path = "/api/v1/bundles/compose",
    tag = "bundles",
    responses(
        (status = 200, description = "Bundle composed", body = ComposeBundleResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn compose_bundle(
    State(state): State<ApiState>,
    Json(request): Json<ComposeBundleRequest>,
) -> impl IntoResponse {
    if request.skills.len() < 2 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "A bundle requires at least 2 skills"
            })),
        )
            .into_response();
    }

    let registry = state.registry.lock().await;

    // Check for existing bundle with these skills (smart matching)
    let existing = registry.find_bundle_by_skills(&request.skills);

    let existing_match = existing.map(|b| BundleSummary {
        id: b.id.clone(),
        name: b.name.clone(),
        description: b.description.clone(),
        version: b.version.clone(),
        visibility: b.visibility.as_str().to_string(),
        skill_count: b.skills.len(),
    });

    let message = if existing_match.is_some() {
        "An existing bundle matches these skills. Use apply or evolve instead of composing a new one.".to_string()
    } else {
        "Bundle composition requires template rendering. Use `kask bundle compose` for full composition.".to_string()
    };

    (
        StatusCode::OK,
        Json(ComposeBundleResponse {
            existing_match,
            manifest: None, // Composition requires template rendering — not available in API alone
            message,
        }),
    )
        .into_response()
}

/// Apply a bundle to the current session
#[utoipa::path(
    post,
    path = "/api/v1/bundles/{id}/apply",
    tag = "bundles",
    responses(
        (status = 200, description = "Bundle applied", body = ApplyBundleResponse),
        (status = 404, description = "Bundle not found"),
    ),
)]
async fn apply_bundle(State(state): State<ApiState>, Path(id): Path<String>) -> impl IntoResponse {
    let registry = state.registry.lock().await;
    match registry.get_bundle(&id) {
        Some(bundle) => (
            StatusCode::OK,
            Json(ApplyBundleResponse {
                status: "active".to_string(),
                bundle_id: bundle.id.clone(),
                name: bundle.name.clone(),
                skill_count: bundle.skills.len(),
            }),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("Bundle '{}' not found", id) })),
        )
            .into_response(),
    }
}

/// Evolve a bundle (re-compose when skills have changed)
#[utoipa::path(
    post,
    path = "/api/v1/bundles/{id}/evolve",
    tag = "bundles",
    responses(
        (status = 200, description = "Bundle evolved", body = EvolveBundleResponse),
        (status = 404, description = "Bundle not found"),
    ),
)]
async fn evolve_bundle(State(state): State<ApiState>, Path(id): Path<String>) -> impl IntoResponse {
    let registry = state.registry.lock().await;
    match registry.get_bundle(&id) {
        Some(_bundle) => (
            StatusCode::OK,
            Json(EvolveBundleResponse {
                evolved_manifest: None, // Evolution requires template rendering
                changes: vec![],
                message: "Bundle evolution requires template rendering. Use `kask bundle evolve` for full evolution.".to_string(),
            }),
        )
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("Bundle '{}' not found", id) })),
        )
            .into_response(),
    }
}

/// Deactivate the current bundle
#[utoipa::path(
    delete,
    path = "/api/v1/bundles/{id}/deactivate",
    tag = "bundles",
    responses(
        (status = 200, description = "Bundle deactivated", body = DeactivateBundleResponse),
    ),
)]
async fn deactivate_bundle(Path(_id): Path<String>) -> Json<DeactivateBundleResponse> {
    Json(DeactivateBundleResponse {
        status: "deactivated".to_string(),
    })
}
