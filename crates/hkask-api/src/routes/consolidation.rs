//! Consolidation API — user-triggered episodic→semantic consolidation + semantic cleanup

use axum::{Extension, Json, extract::State};
use hkask_memory::consolidation_ops;
use hkask_ports::ConsolidationRequest;
use hkask_services::ServiceError;
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ApiState;
use crate::error::ServiceErrorResponse;

// Handlers

/// Consolidate request — triggers episodic-to-semantic memory consolidation.
///
/// Requires the agents master passphrase for authorization (P4 OCAP) and P2
/// affirmative consent for both `EpisodicMemory` and `SemanticMemory` on the
/// target agent. The caller's authenticated WebID must match the target
/// agent's derived WebID.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ConsolidateRequest {
    /// Agent name whose episodic memory to consolidate (e.g. `curator` or a replicant name)
    pub agent_name: String,
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
/// post: returns OpenApi`Router<ApiState>` with consolidation route registered
pub fn consolidation_router() -> OpenApiRouter<crate::ApiState> {
    OpenApiRouter::new().routes(routes!(consolidate))
}

// Handlers

/// Consolidate episodic memories for an agent.
///
/// Triggers the semantic consolidation loop: reads raw episodic triples,
/// derives confidence-weighted semantic knowledge, and deletes low-confidence
/// ephemera. Requires the agent's master passphrase for authorization and
/// affirmative consent for both memory categories.
#[utoipa::path(
    post,
    path = "/api/consolidate",
    tag = "consolidation",
    request_body = ConsolidateRequest,
    responses(
        (status = 200, description = "Consolidation complete", body = ConsolidateResponse),
        (status = 401, description = "Unauthorized — invalid passphrase"),
        (status = 403, description = "Forbidden — caller not authorized for target agent or consent denied"),
        (status = 429, description = "Rate limited — try again later"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub(crate) async fn consolidate(
    State(state): State<ApiState>,
    Extension(auth): Extension<crate::middleware::auth::AuthContext>,
    Json(req): Json<ConsolidateRequest>,
) -> Result<Json<ConsolidateResponse>, ServiceErrorResponse> {
    // Rate-limit: Argon2id derivation is ~100ms CPU per request.
    // Prevent CPU DoS by enforcing a minimum interval between calls.
    consolidation_ops::check_rate_limit()?;

    // Derive the target agent WebID from its name and authorize the caller.
    let target_webid = WebID::from_persona(req.agent_name.as_bytes());
    if auth.webid != target_webid {
        return Err(ServiceError::ConsentDenied {
            message: "Caller is not authorized to consolidate this agent's memory".to_string(),
        }
        .into());
    }

    // Verify passphrase via ConsolidationService (keystore → key derivation → comparison)
    let _db_passphrase = consolidation_ops::verify_passphrase(&req.passphrase)?;

    // Build consolidation request
    let consolidation_request = ConsolidationRequest {
        limit: req.limit.unwrap_or(100),
        confidence_floor: req.confidence_floor,
        max_semantic_triples: req.max_semantic_triples,
    };

    // Route through AgentService: consent check + per-agent DB + consolidation
    let outcome = state
        .agent_service
        .consolidate_agent_memory(&req.agent_name, consolidation_request)?;

    Ok(Json(ConsolidateResponse {
        consolidated_count: outcome.consolidated_count,
        deleted_count: outcome.deleted_count,
        failed_count: outcome.failed_count,
    }))
}
