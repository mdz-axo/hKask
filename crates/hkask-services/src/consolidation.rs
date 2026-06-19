//! Consolidation — passphrase verify, per-agent DB, episodic→semantic pipeline.
//! # REQ: P2 (Affirmative Consent) — consolidation requires passphrase verification.
//! # expect: "Service operations require explicit, scoped consent"

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use hkask_memory::{ConsolidationBridge, EpisodicMemory, SemanticMemory};
use hkask_storage::{Database, EmbeddingStore, TripleStore};
use hkask_types::WebID;
use hkask_types::loops::CuratorHandle;
use hkask_types::ports::{ConsolidationOutcome, ConsolidationRequest};

use crate::ServiceError;

/// Minimum seconds between consolidation requests.
///
/// Each request runs Argon2id key derivation (~100ms CPU) for passphrase
/// verification. Without rate limiting, a tight loop of requests becomes
/// a CPU denial-of-service vector. 30s is appropriate for an admin operation
/// that runs at most a few times per session.
const CONSOLIDATION_MIN_INTERVAL_SECS: u64 = 30;

/// Coarse-grained rate limiter for consolidation requests.
///
/// Uses a single `AtomicU64` timestamp (epoch seconds). Intentionally simple —
/// one global gate, not per-user. For a single-user headless system, this is sufficient.
static LAST_CONSOLIDATION_EPOCH_SECS: AtomicU64 = AtomicU64::new(0);

/// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  none (always succeeds or returns rate-limit error)
/// post: Ok(()) if rate limit not exceeded; Err(RateLimited) with remaining seconds if within 30s window
pub fn check_rate_limit() -> Result<(), ServiceError> {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let prev = LAST_CONSOLIDATION_EPOCH_SECS.load(Ordering::Relaxed);
    if prev != 0 && now_secs.saturating_sub(prev) < CONSOLIDATION_MIN_INTERVAL_SECS {
        let remaining = CONSOLIDATION_MIN_INTERVAL_SECS - now_secs.saturating_sub(prev);
        return Err(ServiceError::RateLimited {
            source: None,
            message: format!("Rate limited: try again in {}s", remaining),
        });
    }
    LAST_CONSOLIDATION_EPOCH_SECS.store(now_secs, Ordering::Relaxed);
    Ok(())
}

/// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  webid must be a valid WebID
/// post: returns "hkask-memory-agent-{webid}.db" path string
pub fn db_path_for_agent(webid: &WebID) -> String {
    format!("hkask-memory-agent-{}.db", webid)
}
/// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  passphrase must be non-empty; server passphrase must be configured in keystore
/// post: returns the expected passphrase string on match; Err(Keystore) if not configured; Err(InvalidPassphrase) if mismatch
pub fn verify_passphrase(passphrase: &str) -> Result<String, ServiceError> {
    let expected =
        hkask_keystore::keychain::resolve_db_passphrase().map_err(|_| ServiceError::Keystore {
            source: None,
            message: "Server passphrase not configured".into(),
        })?;
    let expected_str = String::from_utf8_lossy(&expected).to_string();
    let secrets = hkask_keystore::master_key::derive_all_internal_secrets(passphrase);
    if secrets.capability_key != expected_str {
        return Err(ServiceError::InvalidPassphrase {
            source: None,
            message: "Passphrase verification failed".into(),
        });
    }
    Ok(expected_str)
}

/// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  webid must be a valid WebID; db_passphrase must be correct; db_path must point to a valid database; request must be a valid ConsolidationRequest
/// post: returns ConsolidationOutcome with consolidated_count, deleted_count, failed_count; Err on DB open failure or consolidation failure
pub fn consolidate(
    webid: &WebID,
    db_passphrase: &str,
    db_path: &str,
    request: ConsolidationRequest,
) -> Result<ConsolidationOutcome, ServiceError> {
    let db = Database::open(db_path, db_passphrase).map_err(|e| ServiceError::Storage {
        message: e.to_string(),
    })?;

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
    let domain_service = hkask_memory::ConsolidationService::new(bridge, semantic_memory, token);

    let outcome =
        domain_service
            .consolidate(webid, request)
            .map_err(|e| ServiceError::Consolidation {
                source: None,
                message: format!("Consolidation failed: {e}"),
            })?;

    Ok(ConsolidationOutcome {
        consolidated_count: outcome.consolidated_count,
        deleted_count: outcome.deleted_count,
        failed_count: outcome.failed_count,
    })
}
