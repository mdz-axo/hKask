//! Model listing routes

use axum::{Json, extract::State, routing::Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ApiState;

/// Create models router
pub fn models_router() -> Router<ApiState> {
    Router::new()
        .route("/api/models", axum::routing::get(list_models))
        .route("/api/models/search", axum::routing::get(search_models))
}

/// Model entry from Okapi
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelEntry {
    pub name: String,
    pub family: Option<String>,
    pub parameter_size: Option<String>,
    pub quantization_level: Option<String>,
    pub size_gb: Option<f64>,
}

/// Model list response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelListResponse {
    pub models: Vec<ModelEntry>,
    pub count: usize,
}

/// List available models from Okapi
#[utoipa::path(
    get,
    path = "/api/models",
    tag = "models",
    responses(
        (status = 200, description = "List of available models", body = ModelListResponse),
        (status = 500, description = "Okapi unreachable"),
    ),
)]
async fn list_models(State(_state): State<ApiState>) -> Json<ModelListResponse> {
    use hkask_templates::{OkapiConfig, list_okapi_models};

    let config = OkapiConfig::local_dev();
    let okapi_models = list_okapi_models(&config).await;

    let models: Vec<ModelEntry> = okapi_models
        .into_iter()
        .map(|m| ModelEntry {
            name: m.name,
            family: m.details.as_ref().and_then(|d| d.family.clone()),
            parameter_size: m.details.as_ref().and_then(|d| d.parameter_size.clone()),
            quantization_level: m
                .details
                .as_ref()
                .and_then(|d| d.quantization_level.clone()),
            size_gb: m.size.map(|s| s as f64 / 1_073_741_824.0),
        })
        .collect();

    let count = models.len();
    Json(ModelListResponse { models, count })
}

/// Search query for models
#[derive(Debug, Deserialize, ToSchema)]
pub struct ModelSearchQuery {
    /// Fuzzy search query (matches model name)
    pub q: String,
}

/// Search available models from Okapi
#[utoipa::path(
    get,
    path = "/api/models/search",
    tag = "models",
    params(
        ("q" = String, Path, description = "Fuzzy search query")
    ),
    responses(
        (status = 200, description = "Matching models", body = ModelListResponse),
    ),
)]
async fn search_models(
    State(_state): State<ApiState>,
    axum::extract::Query(query): axum::extract::Query<ModelSearchQuery>,
) -> Json<ModelListResponse> {
    use hkask_templates::{OkapiConfig, search_okapi_models};

    let config = OkapiConfig::local_dev();
    let okapi_models = search_okapi_models(&config, &query.q).await;

    let models: Vec<ModelEntry> = okapi_models
        .into_iter()
        .map(|m| ModelEntry {
            name: m.name,
            family: m.details.as_ref().and_then(|d| d.family.clone()),
            parameter_size: m.details.as_ref().and_then(|d| d.parameter_size.clone()),
            quantization_level: m
                .details
                .as_ref()
                .and_then(|d| d.quantization_level.clone()),
            size_gb: m.size.map(|s| s as f64 / 1_073_741_824.0),
        })
        .collect();

    let count = models.len();
    Json(ModelListResponse { models, count })
}
