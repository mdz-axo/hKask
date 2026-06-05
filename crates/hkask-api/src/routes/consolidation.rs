//! Consolidation API — user-triggered episodic→semantic consolidation + semantic cleanup

use axum::{Extension, Json, extract::State};
use hkask_types::WebID;
use hkask_types::ports::ConsolidationRequest;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::error_response;
use crate::ApiState;

// =============================================================================
// Request / Response types
// =============================================================================

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConsolidateRequest {
    /// Agent WebID whose episodic memory to consolidate
    pub agent_webid: String,
    /// Database passphrase for the agent (required for authorization)
    pub passphrase: String,
    /// Maximum episodic triples to consolidate (default: 100)
    #[schema(default = 100)]
    pub limit: Option<usize>,
    /// Confidence floor — semantic triples at or below this are deleted.
    /// Overrides the SemanticLoop default (0.33).
    pub confidence_floor: Option<f64>,
    /// Maximum semantic triples to retain after consolidation.
    /// If exceeded, lowest-confidence triples are deleted.
    pub max_semantic_triples: Option<usize>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConsolidateResponse {
    pub consolidated_count: usize,
    pub deleted_count: usize,
    pub failed_count: usize,
}

// =============================================================================
// Router
// =============================================================================

pub fn consolidation_router() -> axum::Router<crate::ApiState> {
    axum::Router::new().route("/api/consolidate", axum::routing::post(consolidate))
}

// =============================================================================
// Handlers
// =============================================================================

#[utoipa::path(
    post,
    path = "/api/consolidate",
    request_body = ConsolidateRequest,
    responses(
        (status = 200, description = "Consolidation complete", body = ConsolidateResponse),
        (status = 401, description = "Unauthorized — invalid passphrase"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn consolidate(
    State(state): State<ApiState>,
    Extension(_auth): Extension<crate::middleware::auth::AuthContext>,
    Json(req): Json<ConsolidateRequest>,
) -> axum::Json<serde_json::Value> {
    // Parse agent WebID
    let webid = match uuid::Uuid::parse_str(&req.agent_webid) {
        Ok(id) => WebID(id),
        Err(_) => {
            return axum::Json(error_response(
                400,
                "Invalid agent_webid: must be a valid UUID",
            ));
        }
    };

    // Verify passphrase
    // The passphrase must match the database passphrase used to encrypt the
    // agent's data. We verify by attempting to open the database with it.
    // For the API, we verify against the configured system passphrase.
    let db_passphrase = match std::env::var("HKASK_DB_PASSPHRASE") {
        Ok(p) => p,
        Err(_) => {
            return axum::Json(error_response(500, "Server passphrase not configured"));
        }
    };

    if req.passphrase != db_passphrase {
        tracing::warn!(
            target: "api.consolidation",
            agent_webid = %req.agent_webid,
            "Consolidation rejected: passphrase mismatch"
        );
        return axum::Json(error_response(401, "Invalid passphrase"));
    }

    // Build consolidation request
    let consolidation_request = ConsolidationRequest {
        limit: req.limit.unwrap_or(100),
        confidence_floor: req.confidence_floor,
        max_semantic_triples: req.max_semantic_triples,
    };

    // Execute via ConsolidationService
    let service = state.consolidation_service();
    match service.consolidate(&webid, consolidation_request) {
        Ok(outcome) => axum::Json(serde_json::json!({
            "status": "ok",
            "consolidated_count": outcome.consolidated_count,
            "deleted_count": outcome.deleted_count,
            "failed_count": outcome.failed_count,
        })),
        Err(e) => axum::Json(error_response(500, &e)),
    }
}
