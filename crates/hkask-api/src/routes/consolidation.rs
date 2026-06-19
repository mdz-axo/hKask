//! Consolidation API — user-triggered episodic→semantic consolidation + semantic cleanup

use axum::{Extension, Json, extract::State};
use hkask_services::ServiceError;
use hkask_services::consolidation;
use hkask_types::WebID;
use hkask_types::ports::ConsolidationRequest;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ApiState;
use crate::error::ServiceErrorResponse;

// Handlers

/// Consolidate request — triggers episodic-to-semantic memory consolidation.
///
/// Requires the agents master passphrase for authorization (P4 OCAP).
/// The passphrase is derived via HKDF-SHA256 to produce the capability_key
/// used as the DB passphrase.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ConsolidateRequest {
    /// Agent WebID whose episodic memory to consolidate
    pub agent_webid: String,
    /// Master passphrase for authorization (derived via HKDF-SHA256 to produce
    /// the capability_key used as the DB passphrase, matching onboarding flow)
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

/// Consolidate response — outcome counts from the semantic consolidation loop.
#[derive(Debug, Serialize, ToSchema)]
pub struct ConsolidateResponse {
    pub consolidated_count: usize,
    pub deleted_count: usize,
    pub failed_count: usize,
}

// Router

/// expect: "API endpoints enforce OCAP boundaries"
/// pre:  none
/// post: returns OpenApiRouter<ApiState> with consolidation route registered
pub fn consolidation_router() -> OpenApiRouter<crate::ApiState> {
    OpenApiRouter::new().routes(routes!(consolidate))
}

// Handlers

/// Consolidate episodic memories for an agent.
///
/// Triggers the semantic consolidation loop: reads raw episodic triples,
/// derives confidence-weighted semantic knowledge, and deletes low-confidence
/// ephemera. Requires the agent's master passphrase for authorization.
#[utoipa::path(
    post,
    path = "/api/consolidate",
    tag = "consolidation",
    request_body = ConsolidateRequest,
    responses(
        (status = 200, description = "Consolidation complete", body = ConsolidateResponse),
        (status = 401, description = "Unauthorized — invalid passphrase"),
        (status = 429, description = "Rate limited — try again later"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn consolidate(
    State(_state): State<ApiState>,
    Extension(_auth): Extension<crate::middleware::auth::AuthContext>,
    Json(req): Json<ConsolidateRequest>,
) -> Result<Json<ConsolidateResponse>, ServiceErrorResponse> {
    // Rate-limit: Argon2id derivation is ~100ms CPU per request.
    // Prevent CPU DoS by enforcing a minimum interval between calls.
    consolidation::check_rate_limit()?;

    // Parse agent WebID
    let webid: WebID = req
        .agent_webid
        .parse()
        .map_err(|_| ServiceError::ValidationError {
            source: None,
            message: "Invalid agent_webid: must be a valid UUID".to_string(),
        })?;

    // Verify passphrase via ConsolidationService (keystore → key derivation → comparison)
    let db_passphrase = consolidation::verify_passphrase(&req.passphrase)?;

    // Build consolidation request
    let consolidation_request = ConsolidationRequest {
        limit: req.limit.unwrap_or(100),
        confidence_floor: req.confidence_floor,
        max_semantic_triples: req.max_semantic_triples,
    };

    // Execute via ConsolidationService (per-agent DB + pipeline assembly + consolidation)
    let db_path = consolidation::db_path_for_agent(&webid);
    let outcome =
        consolidation::consolidate(&webid, &db_passphrase, &db_path, consolidation_request)?;

    Ok(Json(ConsolidateResponse {
        consolidated_count: outcome.consolidated_count,
        deleted_count: outcome.deleted_count,
        failed_count: outcome.failed_count,
    }))
}
