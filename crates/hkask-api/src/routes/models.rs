//! Model listing routes
//!
//! Two endpoints for discovering and switching Okapi LLM models:
//!
//! - `GET /api/models` — List all locally available models from Okapi
//! - `GET /api/models/search?q=...` — Fuzzy search models by name
//!
//! Model names returned here can be passed as the `model` field in
//! `POST /api/chat` requests to select which LLM the Curator or agent uses.

use axum::{Json, extract::State, routing::Router};
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::ToSchema;

use crate::ApiState;

/// Create models router
pub fn models_router() -> Router<ApiState> {
    Router::new()
        .route("/api/models", axum::routing::get(list_models))
        .route("/api/models/search", axum::routing::get(search_models))
}

/// A model available through Okapi.
///
/// Includes metadata from Okapi's `/api/tags` endpoint: model family,
/// parameter count, quantization level, and disk size.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelEntry {
    /// Model identifier (e.g., "qwen3:8b", "llama3.1:70b")
    pub name: String,
    /// Model family (e.g., "llama", "qwen2")
    pub family: Option<String>,
    /// Parameter count (e.g., "8B", "70B")
    pub parameter_size: Option<String>,
    /// Quantization level (e.g., "Q4_0", "Q5_K_M")
    pub quantization_level: Option<String>,
    /// Model size in gigabytes
    pub size_gb: Option<f64>,
}

/// Response containing available Okapi models.
///
/// Returned by `GET /api/models` and `GET /api/models/search`.
/// Model names from this response can be used as the `model` field
/// in `POST /api/chat` requests.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelListResponse {
    /// List of available models
    pub models: Vec<ModelEntry>,
    /// Total number of models in the list
    pub count: usize,
}

/// Query parameters for fuzzy model search.
///
/// The `q` parameter performs case-insensitive substring matching
/// against model names. For example, `q=llama` matches "llama3.1:8b",
/// "llama3.1:70b", etc.
#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
pub struct ModelSearchQuery {
    /// Fuzzy search query — matches model name (case-insensitive substring)
    pub q: String,
}

/// List all available models from Okapi.
///
/// Queries Okapi's `/api/tags` endpoint and returns metadata for each
/// locally loaded model. Returns an empty list if Okapi is unreachable
/// (graceful degradation).
#[utoipa::path(
    get,
    path = "/api/models",
    tag = "models",
    responses(
        (status = 200, description = "List of available models", body = ModelListResponse),
        (status = 503, description = "Okapi unreachable (returns empty list)"),
    ),
)]
async fn list_models(State(state): State<ApiState>) -> Json<ModelListResponse> {
    use hkask_services::InferenceService;

    let ctx = hkask_services::InferenceContext::from(&*state.service_context);
    let models = InferenceService::list_models(&ctx)
        .await
        .unwrap_or_default();

    let models: Vec<ModelEntry> = models
        .into_iter()
        .map(|m| ModelEntry {
            name: m.name,
            family: m.family,
            parameter_size: m.parameter_size,
            quantization_level: m.quantization_level,
            size_gb: m.size_bytes.map(|s| s as f64 / 1_073_741_824.0),
        })
        .collect();

    let count = models.len();
    Json(ModelListResponse { models, count })
}

/// Search available models by name (fuzzy matching).
///
/// Performs case-insensitive substring matching against Okapi model names.
/// Use this endpoint to discover valid model identifiers before passing
/// them to `POST /api/chat` via the `model` field.
#[utoipa::path(
    get,
    path = "/api/models/search",
    tag = "models",
    params(
        ("q" = String, Query, description = "Fuzzy search query — matches model name (case-insensitive substring)")
    ),
    responses(
        (status = 200, description = "Matching models", body = ModelListResponse),
    ),
)]
async fn search_models(
    State(state): State<ApiState>,
    axum::extract::Query(query): axum::extract::Query<ModelSearchQuery>,
) -> Json<ModelListResponse> {
    use hkask_services::InferenceService;

    let ctx = hkask_services::InferenceContext::from(&*state.service_context);
    let models = InferenceService::search_models(&ctx, &query.q)
        .await
        .unwrap_or_default();

    let models: Vec<ModelEntry> = models
        .into_iter()
        .map(|m| ModelEntry {
            name: m.name,
            family: m.family,
            parameter_size: m.parameter_size,
            quantization_level: m.quantization_level,
            size_gb: m.size_bytes.map(|s| s as f64 / 1_073_741_824.0),
        })
        .collect();

    let count = models.len();
    Json(ModelListResponse { models, count })
}
