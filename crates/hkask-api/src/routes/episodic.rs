//! Episodic Memory API routes.
//!
//! HTTP surface for episodic memory operations, exposing the `EpisodicMemory`
//! pub methods: `store`, `query_for_deduped`, `storage_usage`, `storage_budget`.

use axum::extract::Extension;
use axum::{Json, extract::Query, extract::State, http::StatusCode, routing::Router};
use hkask_storage::Triple;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::middleware::AuthContext;
use crate::{ApiState, ErrorResponse};

/// Create episodic memory router
pub fn episodic_router() -> Router<ApiState> {
    Router::new()
        .route("/api/episodic/store", axum::routing::post(store_episode))
        .route("/api/episodic/query", axum::routing::get(query_episodes))
        .route("/api/episodic/usage", axum::routing::get(storage_usage))
}

/// Request to store an episodic triple.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct StoreEpisodeRequest {
    /// Entity (subject of the experience).
    pub entity: String,
    /// Attribute (predicate/property).
    pub attribute: String,
    /// Value (object of the triple).
    pub value: serde_json::Value,
    /// Confidence (0.0–1.0). Defaults to 1.0.
    pub confidence: Option<f64>,
}

/// Response from storing an episodic triple.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct StoreEpisodeResponse {
    /// The entity that was stored.
    pub entity: String,
    /// The attribute that was stored.
    pub attribute: String,
}

/// Query parameters for episodic recall.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct QueryEpisodesParams {
    /// Entity to query by.
    pub entity: String,
}

/// A single episodic triple as returned over the API.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct EpisodeResponse {
    pub id: String,
    pub entity: String,
    pub attribute: String,
    pub value: serde_json::Value,
    pub confidence: f64,
    pub perspective: Option<String>,
    pub visibility: String,
    pub valid_from: String,
}

/// Response from querying episodic memories.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct QueryEpisodesResponse {
    pub episodes: Vec<EpisodeResponse>,
}

/// Response for episodic storage usage.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct EpisodicUsageResponse {
    /// Current triple count for the caller's perspective.
    pub count: usize,
    /// Configured storage budget (max triples).
    pub budget: usize,
}

fn error_response(
    status: StatusCode,
    code: &str,
    message: &str,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            error: "episodic_operation_failed".to_string(),
            code: code.to_string(),
            details: Some(serde_json::json!({ "message": message })),
        }),
    )
}

fn triple_to_response(t: &Triple) -> EpisodeResponse {
    EpisodeResponse {
        id: t.id.to_string(),
        entity: t.entity.clone(),
        attribute: t.attribute.clone(),
        value: t.value.clone(),
        confidence: t.confidence,
        perspective: t.perspective.map(|p| p.to_string()),
        visibility: t.visibility.as_str().to_string(),
        valid_from: t.valid_from.to_rfc3339(),
    }
}

/// Store an episodic triple for the authenticated caller.
#[utoipa::path(
    post,
    path = "/api/episodic/store",
    tag = "episodic",
    request_body = StoreEpisodeRequest,
    responses(
        (status = 200, description = "Episode stored", body = StoreEpisodeResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Storage error"),
    ),
)]
async fn store_episode(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<StoreEpisodeRequest>,
) -> Result<Json<StoreEpisodeResponse>, (StatusCode, Json<ErrorResponse>)> {
    if req.entity.is_empty() || req.attribute.is_empty() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "EPISODIC_BAD_REQUEST",
            "entity and attribute must not be empty",
        ));
    }

    let confidence = req.confidence.unwrap_or(1.0);
    let triple = Triple::new(&req.entity, &req.attribute, req.value, auth.webid)
        .with_visibility(hkask_types::Visibility::Private)
        .with_perspective(auth.webid)
        .with_confidence(confidence);

    state.episodic_memory.store(triple).map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "EPISODIC_STORE_ERROR",
            &e.to_string(),
        )
    })?;

    tracing::debug!(
        target: "cns.memory.episodic",
        entity = %req.entity,
        attribute = %req.attribute,
        confidence = confidence,
        "Episodic triple stored via API"
    );

    Ok(Json(StoreEpisodeResponse {
        entity: req.entity,
        attribute: req.attribute,
    }))
}

/// Query episodic memories for the authenticated caller by entity.
///
/// Applies confidence decay, temporal attention weighting, and deduplication
/// (subloops 2a.2–2a.4) before returning results.
#[utoipa::path(
    get,
    path = "/api/episodic/query",
    tag = "episodic",
    params(
        ("entity" = String, Query, description = "Entity to query"),
    ),
    responses(
        (status = 200, description = "Episodes retrieved", body = QueryEpisodesResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Query error"),
    ),
)]
async fn query_episodes(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<QueryEpisodesParams>,
) -> Result<Json<QueryEpisodesResponse>, (StatusCode, Json<ErrorResponse>)> {
    if params.entity.is_empty() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "EPISODIC_BAD_REQUEST",
            "entity query parameter must not be empty",
        ));
    }

    let triples = state
        .episodic_memory
        .query_for_deduped(&params.entity, auth.webid)
        .map_err(|e| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "EPISODIC_QUERY_ERROR",
                &e.to_string(),
            )
        })?;

    Ok(Json(QueryEpisodesResponse {
        episodes: triples.iter().map(triple_to_response).collect(),
    }))
}

/// Get episodic storage usage for the authenticated caller.
#[utoipa::path(
    get,
    path = "/api/episodic/usage",
    tag = "episodic",
    responses(
        (status = 200, description = "Storage usage", body = EpisodicUsageResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Query error"),
    ),
)]
async fn storage_usage(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<EpisodicUsageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let count = state
        .episodic_memory
        .storage_usage(&auth.webid)
        .map_err(|e| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "EPISODIC_USAGE_ERROR",
                &e.to_string(),
            )
        })?;
    let budget = state.episodic_memory.storage_budget();

    Ok(Json(EpisodicUsageResponse { count, budget }))
}
