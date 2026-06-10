//! Episodic Memory API routes.
//!
//! HTTP surface for episodic memory operations, routed through
//! `EpisodicStoragePort` for OCAP discipline. All requests carry a
//! `DelegationToken` via the HTTP auth middleware (`AuthContext`).

use axum::extract::Extension;
use axum::{Json, extract::Query, extract::State, routing::Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ApiError;
use crate::ApiState;
use crate::middleware::AuthContext;
use hkask_agents::{MemoryError, RecallRequest, StorageRequest};
use hkask_types::Confidence;

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

/// Store an episodic triple for the authenticated caller.
///
/// Routes through `EpisodicStoragePort` with OCAP discipline:
/// the `DelegationToken` from HTTP auth is verified at the membrane.
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
) -> Result<Json<StoreEpisodeResponse>, ApiError> {
    if req.entity.is_empty() || req.attribute.is_empty() {
        return Err(ApiError::BadRequest {
            message: "entity and attribute must not be empty".to_string(),
        });
    }

    let confidence = Confidence::new(req.confidence.unwrap_or(1.0));
    let request = StorageRequest::episodic(
        &req.entity,
        &req.attribute,
        req.value,
        confidence,
        auth.webid,
    );
    state
        .agent_service
        .episodic_storage()
        .store_episodic(request, &auth.token)
        .map_err(|e| match &e {
            MemoryError::CapabilityDenied { resource, action } => ApiError::Forbidden {
                reason: format!("Capability denied: {} on {}", action, resource),
            },
            _ => ApiError::Internal {
                message: e.to_string(),
            },
        })?;

    tracing::debug!(
        target: "cns.memory.episodic",
        entity = %req.entity,
        attribute = %req.attribute,
        confidence = %confidence,
        "Episodic triple stored via API (through port membrane)"
    );

    Ok(Json(StoreEpisodeResponse {
        entity: req.entity,
        attribute: req.attribute,
    }))
}

/// Query episodic memories for the authenticated caller by entity.
///
/// Routes through `EpisodicStoragePort` with OCAP discipline.
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
) -> Result<Json<QueryEpisodesResponse>, ApiError> {
    if params.entity.is_empty() {
        return Err(ApiError::BadRequest {
            message: "entity query parameter must not be empty".to_string(),
        });
    }

    let request = RecallRequest::episodic(&params.entity, auth.webid, auth.token);
    let results = state
        .agent_service
        .episodic_storage()
        .recall_episodic(&request)
        .map_err(|e| match &e {
            MemoryError::CapabilityDenied { resource, action } => ApiError::Forbidden {
                reason: format!("Capability denied: {} on {}", action, resource),
            },
            _ => ApiError::Internal {
                message: e.to_string(),
            },
        })?;

    let episodes: Vec<EpisodeResponse> = results
        .into_iter()
        .map(|ep| EpisodeResponse {
            id: ep.id,
            entity: ep.entity,
            attribute: ep.attribute,
            value: ep.value,
            confidence: ep.confidence.value(),
            perspective: ep.perspective.map(|p| p.to_string()),
            visibility: ep.visibility.as_str().to_string(),
            valid_from: ep.valid_from,
        })
        .collect();

    Ok(Json(QueryEpisodesResponse { episodes }))
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
) -> Result<Json<EpisodicUsageResponse>, ApiError> {
    let count = state
        .agent_service
        .episodic_storage()
        .episodic_storage_usage(&auth.webid)
        .map_err(|e| ApiError::Internal {
            message: e.to_string(),
        })?;
    let budget = state
        .agent_service
        .episodic_storage()
        .episodic_storage_budget();

    Ok(Json(EpisodicUsageResponse { count, budget }))
}
