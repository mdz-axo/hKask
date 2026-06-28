//! Consolidation authentication helpers — passphrase verification and rate limiting.
//!
//! The actual per-agent DB open + consolidation pipeline now lives in
//! `hkask_services_context::AgentService::consolidate_agent_memory`, which is
//! the single OCAP-gated, consent-checked entry point. This module only keeps
//! the helpers that surfaces (CLI/API) use as additional auth gates.
//! # REQ: P2 (Affirmative Consent) — consolidation requires explicit consent.
//! # expect: "Service operations require explicit, scoped consent"

use std::sync::atomic::{AtomicU64, Ordering};

use hkask_services_core::ServiceError;

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

/// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
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

/// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
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
