//! Bundle management routes
//!
//! # REQ: P11 (Digital Public/Private Sphere) — API surface for bundle management
//!
//! Delegates to `BundleService` for all business logic. The `compose` and
//! `evolve` endpoints now use inference-driven composition via the shared
//! service layer, replacing the previous stub responses.

use axum::Json;
use axum::extract::{Path, State};
use hkask_services::BundleService;
use hkask_types::Visibility;

use crate::ApiError;
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
    /// The composed bundle manifest (as JSON)
    pub manifest: Option<serde_json::Value>,
    /// Warnings from composition
    pub warnings: Vec<String>,
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
pub fn bundles_router() -> utoipa_axum::router::OpenApiRouter<ApiState> {
    utoipa_axum::router::OpenApiRouter::new()
        .routes(utoipa_axum::routes!(list_bundles))
        .routes(utoipa_axum::routes!(compose_bundle))
        .routes(utoipa_axum::routes!(get_bundle))
        .routes(utoipa_axum::routes!(apply_bundle))
        .routes(utoipa_axum::routes!(evolve_bundle))
        .routes(utoipa_axum::routes!(deactivate_bundle))
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
    let bundles = BundleService::list(&state.agent_service)
        .await
        .unwrap_or_default();

    let bundles: Vec<BundleSummary> = bundles
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
    params(
        ("id" = String, Path, description = "Bundle ID"),
    ),
    responses(
        (status = 200, description = "Bundle manifest", body = serde_json::Value),
        (status = 404, description = "Bundle not found"),
    ),
)]
pub(crate) async fn get_bundle(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match BundleService::get(&state.agent_service, &id).await {
        Ok(Some(bundle)) => {
            let value =
                serde_json::to_value(&bundle).unwrap_or(serde_json::json!({"id": bundle.id}));
            Ok(Json(value))
        }
        Ok(None) => Err(ApiError::NotFound {
            resource: "bundle".into(),
            id,
        }),
        Err(e) => Err(ApiError::Internal {
            message: e.to_string(),
        }),
    }
}

/// Resolve an inference port for bundle composition from the API service context.
///
/// Uses the shared inference port from `AgentService::coordination()` when
/// available, or creates a fresh port as fallback.
fn resolve_api_composition_port(
    state: &ApiState,
) -> Result<std::sync::Arc<dyn hkask_types::ports::InferencePort>, ApiError> {
    // Prefer the shared port from AgentService
    if let Some(port) = state.agent_service.inference_port() {
        return Ok(port);
    }
    // Fallback: create a fresh inference port
    let okapi_url = state.agent_service.config().okapi_base_url.clone();
    let ctx = hkask_services::InferenceContext::from_parts(
        None,
        &state.agent_service.config().default_model,
        &okapi_url,
    );
    hkask_services::InferenceService::resolve_port(
        &ctx,
        &state.agent_service.config().default_model,
    )
    .map_err(|e| ApiError::Internal {
        message: format!("Failed to initialize inference port: {}", e),
    })
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
pub(crate) async fn compose_bundle(
    State(state): State<ApiState>,
    Json(request): Json<ComposeBundleRequest>,
) -> Result<Json<ComposeBundleResponse>, ApiError> {
    if request.skills.len() < 2 {
        return Err(ApiError::BadRequest {
            message: "A bundle requires at least 2 skills".to_string(),
        });
    }

    let vis = Visibility::parse_str(&request.visibility).unwrap_or(Visibility::Private);
    let inference_port = resolve_api_composition_port(&state)?;
    let editor = hkask_services::resolve_replicant_name();

    let result = BundleService::compose(
        &state.agent_service,
        &request.skills,
        request.name.as_deref(),
        vis,
        inference_port,
        &editor,
    )
    .await
    .map_err(|e| ApiError::Internal {
        message: format!("Bundle composition failed: {}", e),
    })?;

    let manifest_json = serde_json::to_value(&result.manifest).unwrap_or(serde_json::Value::Null);

    Ok(Json(ComposeBundleResponse {
        manifest: Some(manifest_json),
        warnings: result.warnings,
        message: format!(
            "Bundle '{}' composed with {} skills",
            result.manifest.id,
            result.manifest.skills.len()
        ),
    }))
}

/// Apply a bundle to the current session
#[utoipa::path(
    post,
    path = "/api/v1/bundles/{id}/apply",
    tag = "bundles",
    params(
        ("id" = String, Path, description = "Bundle ID to apply"),
    ),
    responses(
        (status = 200, description = "Bundle applied", body = ApplyBundleResponse),
        (status = 404, description = "Bundle not found"),
    ),
)]
pub(crate) async fn apply_bundle(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<ApplyBundleResponse>, ApiError> {
    match BundleService::apply(&state.agent_service, &id).await {
        Ok(bundle) => Ok(Json(ApplyBundleResponse {
            status: "active".to_string(),
            bundle_id: bundle.id.clone(),
            name: bundle.name.clone(),
            skill_count: bundle.skills.len(),
        })),
        Err(_) => Err(ApiError::NotFound {
            resource: "bundle".into(),
            id,
        }),
    }
}

/// Evolve a bundle (re-compose when skills have changed)
#[utoipa::path(
    post,
    path = "/api/v1/bundles/{id}/evolve",
    tag = "bundles",
    params(
        ("id" = String, Path, description = "Bundle ID to evolve"),
    ),
    responses(
        (status = 200, description = "Bundle evolved", body = EvolveBundleResponse),
        (status = 404, description = "Bundle not found"),
    ),
)]
pub(crate) async fn evolve_bundle(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<EvolveBundleResponse>, ApiError> {
    let inference_port = resolve_api_composition_port(&state)?;
    let editor = hkask_services::resolve_replicant_name();

    let result = BundleService::evolve(&state.agent_service, &id, inference_port, &editor)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                ApiError::NotFound {
                    resource: "bundle".into(),
                    id: id.clone(),
                }
            } else {
                ApiError::Internal {
                    message: format!("Bundle evolution failed: {}", e),
                }
            }
        })?;

    let manifest_json = serde_json::to_value(&result.manifest).unwrap_or(serde_json::Value::Null);

    Ok(Json(EvolveBundleResponse {
        evolved_manifest: Some(manifest_json),
        changes: result.warnings.clone(),
        message: format!(
            "Bundle '{}' evolved with {} skills",
            result.manifest.id,
            result.manifest.skills.len()
        ),
    }))
}

/// Deactivate the current bundle
#[utoipa::path(
    delete,
    path = "/api/v1/bundles/{id}/deactivate",
    tag = "bundles",
    params(
        ("id" = String, Path, description = "Bundle ID to deactivate"),
    ),
    responses(
        (status = 200, description = "Bundle deactivated", body = DeactivateBundleResponse),
    ),
)]
pub(crate) async fn deactivate_bundle(Path(_id): Path<String>) -> Json<DeactivateBundleResponse> {
    Json(DeactivateBundleResponse {
        status: "deactivated".to_string(),
    })
}
