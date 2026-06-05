//! Consolidation API — user-triggered episodic→semantic consolidation + semantic cleanup

use std::sync::atomic::{AtomicU64, Ordering};

use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};
use hkask_memory::{ConsolidationBridge, ConsolidationService, EpisodicMemory, SemanticMemory};
use hkask_storage::{Database, EmbeddingStore, TripleStore};
use hkask_types::WebID;
use hkask_types::loops::CuratorHandle;
use hkask_types::ports::{ConsolidationPort, ConsolidationRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use super::error_response;
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

fn check_rate_limit() -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let prev = LAST_CONSOLIDATION_EPOCH_SECS.load(Ordering::Relaxed);
    if prev != 0 && now_secs.saturating_sub(prev) < CONSOLIDATION_MIN_INTERVAL_SECS {
        let remaining = CONSOLIDATION_MIN_INTERVAL_SECS - now_secs.saturating_sub(prev);
        Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(error_response(
                429,
                &format!("Rate limited: try again in {}s", remaining),
            )),
        ))
    } else {
        LAST_CONSOLIDATION_EPOCH_SECS.store(now_secs, Ordering::Relaxed);
        Ok(())
    }
}

// =============================================================================
// Request / Response types
// =============================================================================

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
        (status = 429, description = "Rate limited — try again later"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn consolidate(
    State(_state): State<ApiState>,
    Extension(_auth): Extension<crate::middleware::auth::AuthContext>,
    Json(req): Json<ConsolidateRequest>,
) -> impl axum::response::IntoResponse {
    // Rate-limit: Argon2id derivation is ~100ms CPU per request.
    // Prevent CPU DoS by enforcing a minimum interval between calls.
    if let Err(rate_limit_response) = check_rate_limit() {
        return rate_limit_response.into_response();
    }

    // Parse agent WebID
    let webid = match req.agent_webid.parse::<WebID>() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(error_response(
                    400,
                    "Invalid agent_webid: must be a valid UUID",
                )),
            )
                .into_response();
        }
    };

    // Verify passphrase using the master-passphrase → capability_key derivation chain.
    // This matches onboarding: derive_all_internal_secrets(master_passphrase) produces
    // a capability_key that is stored in the keychain as "hkask-db-passphrase" and used
    // as the DB encryption key. We verify the user-supplied master passphrase by
    // deriving the capability_key and comparing it against the resolved DB passphrase.
    let expected = match hkask_keystore::resolve_db_passphrase() {
        Ok(db_pass) => String::from_utf8_lossy(&db_pass).to_string(),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(error_response(500, "Server passphrase not configured")),
            )
                .into_response();
        }
    };
    let secrets = hkask_keystore::master_key::derive_all_internal_secrets(&req.passphrase);
    if secrets.capability_key != expected {
        tracing::warn!(
            target: "api.consolidation",
            agent_webid = %req.agent_webid,
            "Consolidation rejected: passphrase mismatch"
        );
        return (
            StatusCode::UNAUTHORIZED,
            Json(error_response(401, "Invalid passphrase")),
        )
            .into_response();
    }

    // Resolve the agent's per-agent memory DB.
    // Consolidation operates on the agent's actual episodic and semantic triples,
    // which live in hkask-memory-{agent}.db — not the registry DB or in-memory DBs.
    // Derive the agent name from the WebID for DB path resolution.
    let agent_name = format!("agent-{}", webid);
    let db_path = format!("hkask-memory-{}.db", agent_name);
    let db_passphrase = expected;
    let db = match Database::open(&db_path, &db_passphrase) {
        Ok(db) => db,
        Err(e) => {
            tracing::error!(
                target: "api.consolidation",
                db_path = %db_path,
                error = %e,
                "Failed to open agent memory DB"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(error_response(
                    500,
                    &format!("Failed to open agent memory DB: {}", e),
                )),
            )
                .into_response();
        }
    };

    // Build ConsolidationService from the agent's per-agent memory DB.
    // Same pattern as CLI and REPL: EpisodicMemory + SemanticMemory from
    // the same DB, ConsolidationBridge connecting them.
    let conn = db.conn_arc();
    let ts1 = TripleStore::new(Arc::clone(&conn));
    let episodic_memory = Arc::new(EpisodicMemory::new(ts1));
    let ts2 = TripleStore::new(Arc::clone(&conn));
    let embedding_store = EmbeddingStore::new(Arc::clone(&conn));
    let semantic_memory = Arc::new(SemanticMemory::new(ts2, embedding_store));
    let bridge = Arc::new(ConsolidationBridge::new(
        Arc::clone(&episodic_memory),
        Arc::clone(&semantic_memory),
    ));
    let handle = CuratorHandle::system();
    let token = handle.issue_consolidation_token();
    let service =
        ConsolidationService::new(bridge as Arc<dyn ConsolidationPort>, semantic_memory, token);

    // Build consolidation request
    let consolidation_request = ConsolidationRequest {
        limit: req.limit.unwrap_or(100),
        confidence_floor: req.confidence_floor,
        max_semantic_triples: req.max_semantic_triples,
    };

    // Execute via ConsolidationService
    match service.consolidate(&webid, consolidation_request) {
        Ok(outcome) => (
            StatusCode::OK,
            Json(ConsolidateResponse {
                consolidated_count: outcome.consolidated_count,
                deleted_count: outcome.deleted_count,
                failed_count: outcome.failed_count,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(error_response(500, &e)),
        )
            .into_response(),
    }
}
