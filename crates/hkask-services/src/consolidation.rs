//! Consolidation service — passphrase verification, per-agent DB construction,
//! and episodic→semantic consolidation pipeline assembly.
//!
//! # Depth test
//!
//! Deleting this module would cause ~30 lines of infrastructure assembly
//! (keystore access, Argon2id key derivation, per-agent DB opening, memory
//! pipeline construction) to reappear in any caller. The passphrase
//! verification crosses three domain boundaries (keystore → master_key →
//! memory). Passes deletion test.
//!
//! # Design decisions
//!
//! - **Constraint: Guardrail (P1)** — Rate limiting moved to the service layer.
//!   Both CLI and API need consolidation rate limiting (Argon2id CPU cost).
//!   The service enforces it; surfaces no longer need separate gates.
//! - **Constraint: Guideline** — WebID parsing stays in the surface.
//!   Both CLI and API parse WebID from different sources (persona vs request).
//! - **Naming** — `ConsolidationService` here wraps
//!   `hkask_memory::ConsolidationService` (the domain execution engine).
//!   This service owns the infrastructure assembly that precedes execution.

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

/// Consolidation service — passphrase verification and pipeline execution.
///
/// This service owns the infrastructure assembly that precedes consolidation:
/// verifying the master passphrase, opening the per-agent memory DB, and
/// constructing the episodic→semantic pipeline. Surfaces call
/// `ConsolidationService::verify_passphrase()` then `consolidate()`.
pub struct ConsolidationService;

impl ConsolidationService {
    /// Check the consolidation rate limit.
    ///
    /// Returns `Ok(())` if enough time has elapsed since the last consolidation.
    /// Returns `Err(ServiceError::RateLimited)` if called too soon.
    ///
    /// # REQ: svc-consolidation-003 — check_rate_limit enforces minimum interval
    pub fn check_rate_limit() -> Result<(), ServiceError> {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let prev = LAST_CONSOLIDATION_EPOCH_SECS.load(Ordering::Relaxed);
        if prev != 0 && now_secs.saturating_sub(prev) < CONSOLIDATION_MIN_INTERVAL_SECS {
            let remaining = CONSOLIDATION_MIN_INTERVAL_SECS - now_secs.saturating_sub(prev);
            return Err(ServiceError::RateLimited(format!(
                "Rate limited: try again in {}s",
                remaining
            )));
        }
        LAST_CONSOLIDATION_EPOCH_SECS.store(now_secs, Ordering::Relaxed);
        Ok(())
    }

    /// Build the per-agent memory DB path for a given WebID.
    ///
    /// The convention is `hkask-memory-agent-{webid}.db`. This is a domain
    /// convention that should not be duplicated in surfaces.
    ///
    /// # REQ: svc-consolidation-004 — db_path_for_agent returns per-agent DB path
    pub fn db_path_for_agent(webid: &WebID) -> String {
        format!("hkask-memory-agent-{}.db", webid)
    }
    /// Verify a master passphrase against the stored DB passphrase.
    ///
    /// Derives internal secrets from the supplied passphrase via Argon2id +
    /// HKDF-SHA256 and compares the capability_key against the resolved DB
    /// passphrase. Returns the verified DB passphrase string on success.
    ///
    /// # REQ: svc-consolidation-001 — verify_passphrase rejects invalid passphrase
    pub fn verify_passphrase(passphrase: &str) -> Result<String, ServiceError> {
        let expected = hkask_keystore::resolve_db_passphrase()
            .map_err(|_| ServiceError::Keystore("Server passphrase not configured".into()))?;
        let expected_str = String::from_utf8_lossy(&expected).to_string();
        let secrets = hkask_keystore::master_key::derive_all_internal_secrets(passphrase);
        if secrets.capability_key != expected_str {
            return Err(ServiceError::InvalidPassphrase(
                "Passphrase verification failed".into(),
            ));
        }
        Ok(expected_str)
    }

    /// Execute consolidation for an agent's episodic→semantic memory.
    ///
    /// Opens the per-agent memory DB at `db_path`, constructs the consolidation
    /// pipeline (EpisodicMemory → ConsolidationBridge → SemanticMemory), and
    /// executes the consolidation with the given parameters.
    ///
    /// `db_path` is the per-agent memory DB path (e.g., `hkask-memory-curator.db`).
    /// Surfaces derive this differently: CLI uses agent name; API uses WebID.
    ///
    /// # REQ: svc-consolidation-002 — consolidate runs pipeline and returns outcome
    pub fn consolidate(
        webid: &WebID,
        db_passphrase: &str,
        db_path: &str,
        request: ConsolidationRequest,
    ) -> Result<ConsolidationOutcome, ServiceError> {
        let db = Database::open(db_path, db_passphrase)?;

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
        let domain_service =
            hkask_memory::ConsolidationService::new(bridge, semantic_memory, token);

        let outcome = domain_service
            .consolidate(webid, request)
            .map_err(ServiceError::Consolidation)?;

        Ok(ConsolidationOutcome {
            consolidated_count: outcome.consolidated_count,
            deleted_count: outcome.deleted_count,
            failed_count: outcome.failed_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: svc-consolidation-001 — verify_passphrase rejects invalid passphrase
    #[test]
    fn verify_passphrase_rejects_invalid_passphrase() {
        // This test can't test the happy path (it requires keychain access),
        // but it can verify that an arbitrary passphrase fails verification
        // when the keystore is not configured.
        let result = ConsolidationService::verify_passphrase("arbitrary-wrong-passphrase");
        // In test environments without keychain config, this should fail at
        // keystore resolution OR passphrase mismatch. Either way, it's an error.
        assert!(
            result.is_err(),
            "arbitrary passphrase should fail verification"
        );
    }

    // REQ: svc-consolidation-002 — consolidate pipeline constructs from fresh DB
    #[test]
    fn consolidate_pipeline_constructs_from_fresh_db() {
        // Database::open creates the DB if it doesn't exist (SQLCipher behavior).
        // Verify the pipeline assembly works on a fresh per-agent DB.
        let tmp = tempfile::tempdir().expect("temp dir");
        let db_path = tmp.path().join("hkask-memory-agent-test.db");
        let db =
            Database::open(db_path.to_str().unwrap(), "test-passphrase").expect("fresh DB opens");
        let conn = db.conn_arc();
        let ts = TripleStore::new(Arc::clone(&conn));
        let _episodic = EpisodicMemory::new(ts);
        // Pipeline assembly succeeds — actual consolidation with data
        // is covered by hkask-memory integration tests.
    }
}
