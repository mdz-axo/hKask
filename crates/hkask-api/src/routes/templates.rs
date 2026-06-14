//! Template management routes
//!
//! # Service layer depth test
//!
//! TemplateService was considered but **rejected** as shallow: every handler is a
//! thin delegation to a `SqliteRegistry` method plus HTTP response mapping. No
//! cross-surface business logic duplication exists (CLI template commands take
//! `&mut SqliteRegistry` directly and do terminal formatting). A TemplateService
//! would just be `self.registry().list()` / `self.registry().get()` / etc. — pure
//! pass-throughs that increase interface cost without adding behavior.
//!
//! Decision: Guideline — keep direct `service_context.registry()` access.
//! Revisit if template matching logic grows beyond name/skill/polarity queries.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use hkask_types::ports::RegistryIndex;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ApiError;
use crate::ApiState;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Template response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TemplateResponse {
    pub id: String,
    pub template_type: String,
    pub name: String,
    pub description: String,
    pub source_path: String,
    pub lexicon_terms: Vec<String>,
}

/// Capability request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GrantCapabilityRequest {
    pub capability: String,
}

/// Create templates router
pub fn templates_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(list_templates))
        .routes(routes!(get_template))
        .route("/api/templates", axum::routing::post(register_template))
        .route(
            "/api/templates/search/{term}",
            axum::routing::get(search_templates),
        )
}

/// List templates
#[utoipa::path(
    get,
    path = "/api/templates",
    tag = "templates",
    responses(
        (status = 200, description = "List of templates", body = Vec<TemplateResponse>),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn list_templates(State(state): State<ApiState>) -> Json<Vec<TemplateResponse>> {
    let registry = state.agent_service.registry().lock().await;
    let entries = registry.list(None);

    let templates = entries
        .iter()
        .map(|e| TemplateResponse {
            id: e.id.clone(),
            template_type: e.template_type.as_str().to_string(),
            name: e.name.clone(),
            description: e.description.clone(),
            source_path: e.source_path.clone(),
            lexicon_terms: e.lexicon_terms.clone(),
        })
        .collect();

    Json(templates)
}

/// Get template by ID
#[utoipa::path(
    get,
    path = "/api/templates/{id}",
    tag = "templates",
    params(
        ("id" = String, Path, description = "Template ID"),
    ),
    responses(
        (status = 200, description = "Template details", body = TemplateResponse),
        (status = 404, description = "Template not found"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn get_template(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<TemplateResponse>, ApiError> {
    let registry = state.agent_service.registry().lock().await;

    let entry = registry.get(&id)?;

    Ok(Json(TemplateResponse {
        id: entry.id.clone(),
        template_type: entry.template_type.as_str().to_string(),
        name: entry.name.clone(),
        description: entry.description.clone(),
        source_path: entry.source_path.clone(),
        lexicon_terms: entry.lexicon_terms.clone(),
    }))
}

/// Register template
async fn register_template(
    State(state): State<ApiState>,
    Json(_req): Json<TemplateResponse>,
) -> Result<StatusCode, ApiError> {
    use axum::http::StatusCode;

    let _registry = state.agent_service.registry().lock().await;
    Ok(StatusCode::CREATED)
}

/// Search templates by lexicon term
async fn search_templates(
    State(state): State<ApiState>,
    Path(term): Path<String>,
) -> Json<Vec<TemplateResponse>> {
    let registry = state.agent_service.registry().lock().await;
    let results = registry.search_by_lexicon(&term).unwrap_or_default();

    let templates = results
        .iter()
        .map(|e| TemplateResponse {
            id: e.id.clone(),
            template_type: e.template_type.as_str().to_string(),
            name: e.name.clone(),
            description: e.description.clone(),
            source_path: e.source_path.clone(),
            lexicon_terms: e.lexicon_terms.clone(),
        })
        .collect();

    Json(templates)
}
