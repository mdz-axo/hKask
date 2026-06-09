//! Consolidation API — user-triggered episodic→semantic consolidation + semantic cleanup

use std::sync::atomic::{AtomicU64, Ordering};

use axum::{Extension, Json, extract::State};
use hkask_services::ConsolidationService;
use hkask_types::WebID;
use hkask_types::ports::ConsolidationRequest;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::ApiError;
use crate::ApiState;

/// Minimum seconds between consolidation requests to the API.
///
/// Each request runs Argon2id key derivation (~100ms CPU) for passphrase
/// verification. Without rate limiting, a tight loop of requests becomes
/// a CPU denial-of-service vector. 30s is appropriate for an admin operation
/// that runs at most a few times per session.
const CONSOLIDATION_MIN_INTERVAL_SECS: u64 = 30;

/// Coarse-grained rate limiter for the consolidation endpoint.
///
/// Uses a single `AtomicU64` timestamp (seconds since `Instant::now()` is not
/// available across threads, so we use `SystemTime` epoch seconds). This is
/// intentionally simple — one global gate, not per-user. For a single-user
/// headless system, this is sufficient.
static LAST_CONSOLIDATION_EPOCH_SECS: AtomicU64 = AtomicU64::new(0);

fn check_rate_limit() -> Result<(), ApiError> {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let prev = LAST_CONSOLIDATION_EPOCH_SECS.load(Ordering::Relaxed);
    if prev != 0 && now_secs.saturating_sub(prev) < CONSOLIDATION_MIN_INTERVAL_SECS {
        let remaining = CONSOLIDATION_MIN_INTERVAL_SECS - now_secs.saturating_sub(prev);
        Err(ApiError::BadRequest {
            message: format!("Rate limited: try again in {}s", remaining),
        })
    } else {
        LAST_CONSOLIDATION_EPOCH_SECS.store(now_secs, Ordering::Relaxed);
        Ok(())
    }
}

// Request / Response types

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

#[derive(Debug, Serialize, ToSchema)]
pub struct ConsolidateResponse {
    pub consolidated_count: usize,
    pub deleted_count: usize,
    pub failed_count: usize,
}

// Router

pub fn consolidation_router() -> axum::Router<crate::ApiState> {
    axum::Router::new().route("/api/consolidate", axum::routing::post(consolidate))
}

// Handlers

#[utoipa::path(
    post,
    path = "/api/consolidate",
    request_body = ConsolidateRequest,
    responses(
        (status = 200, description = "Consolidation complete", body = ConsolidateResponse),
        (status = 401, description = "Unauthorized — invalid passphrase"),
        (status = 429, description = "Rate limited — try again later"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn consolidate(
    State(_state): State<ApiState>,
    Extension(_auth): Extension<crate::middleware::auth::AuthContext>,
    Json(req): Json<ConsolidateRequest>,
) -> Result<Json<ConsolidateResponse>, ApiError> {
    // Rate-limit: Argon2id derivation is ~100ms CPU per request.
    // Prevent CPU DoS by enforcing a minimum interval between calls.
    check_rate_limit()?;

    // Parse agent WebID
    let webid = req
        .agent_webid
        .parse::<WebID>()
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid agent_webid: must be a valid UUID".to_string(),
        })?;

    // Verify passphrase via ConsolidationService (keystore → key derivation → comparison)
    let db_passphrase =
        ConsolidationService::verify_passphrase(&req.passphrase).map_err(|e| match e {
            hkask_services::ServiceError::InvalidPassphrase(_) => {
                tracing::warn!(
                    target: "api.consolidation",
                    agent_webid = %req.agent_webid,
                    "Consolidation rejected: passphrase mismatch"
                );
                ApiError::Unauthorized {
                    reason: "Invalid passphrase".to_string(),
                }
            }
            hkask_services::ServiceError::Keystore(_) => ApiError::Internal {
                message: "Server passphrase not configured".to_string(),
            },
            other => ApiError::Internal {
                message: other.to_string(),
            },
        })?;

    // Build consolidation request
    let consolidation_request = ConsolidationRequest {
        limit: req.limit.unwrap_or(100),
        confidence_floor: req.confidence_floor,
        max_semantic_triples: req.max_semantic_triples,
    };

    // Execute via ConsolidationService (per-agent DB + pipeline assembly + consolidation)
    let db_path = format!("hkask-memory-agent-{}.db", webid);
    let outcome =
        ConsolidationService::consolidate(&webid, &db_passphrase, &db_path, consolidation_request)
            .map_err(|e| ApiError::Internal {
                message: e.to_string(),
            })?;

    Ok(Json(ConsolidateResponse {
        consolidated_count: outcome.consolidated_count,
        deleted_count: outcome.deleted_count,
        failed_count: outcome.failed_count,
    }))
}
