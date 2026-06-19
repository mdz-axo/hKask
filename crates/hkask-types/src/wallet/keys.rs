//! API key and encumbrance types — capabilities, rate limits, rJoule locks.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::chain::{ChainId, Ed25519PublicKey, PrivacyMode};
use super::types::RJoule;
use crate::id::{ApiKeyId, WalletId};

// ── RateLimitConfig — per-key rate limiting ────────────────────────────────────

/// Rate limit configuration for an API key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per minute.
    pub requests_per_minute: u32,
    /// Maximum tokens per day.
    pub tokens_per_day: u64,
}

// ── ApiKeyCapability — signed OCAP capability token ────────────────────────────

/// An API key is an Ed25519-signed capability token, not an opaque bearer string.
///
/// # OCAP alignment (P4) `[OUGHT-DECL]`
/// The capability carries its own attenuation: `spending_limit_rj`, `expiry`,
/// `privacy_mode`, `scope`, `rate_limit`. The Ed25519 signature proves it was
/// issued by a specific wallet.
///
/// # Invariant `[OUGHT-DECL]`
/// `spent_rj <= spending_limit_rj` at all times. The `WalletBackedBudget` enforces
/// this before every tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyCapability {
    pub wallet_id: WalletId,
    pub key_id: ApiKeyId,
    pub public_key: Ed25519PublicKey,
    pub spending_limit_rj: RJoule,
    pub spent_rj: RJoule,
    /// Allowed endpoint scopes (e.g., ["embed-corpus", "read-specs"]).
    pub scope: Vec<String>,
    /// Stated purpose for this key (≥20 chars per 7R7 approval gate 4).
    pub purpose: String,
    /// Optional rate limit configuration.
    pub rate_limit: Option<RateLimitConfig>,
    pub expiry: Option<DateTime<Utc>>,
    pub issued_at: DateTime<Utc>,
    pub privacy_mode: PrivacyMode,
    pub preferred_chain: Option<ChainId>,
}

impl ApiKeyCapability {
    /// Whether this key is currently active (not revoked, not expired).
    /// Revocation is tracked in storage (`revoked_at` timestamp), not on the capability itself.
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expiry.is_some_and(|exp| now > exp)
    }

    /// Remaining rJoule budget on this key.
    pub fn remaining_rj(&self) -> RJoule {
        self.spending_limit_rj.saturating_sub(self.spent_rj)
    }
}

// ── ApiKeyMaterial — what the user receives when a key is "printed" ────────────

/// The material returned when an API key is created.
///
/// The `private_key_hex` IS the API key — the user stores this and presents it
/// as a Bearer token. It is shown exactly once at creation time.
///
/// # Security `[OUGHT-DECL]`
/// Debug output redacts `private_key_hex` to prevent accidental key leakage
/// in logs, error messages, or debug formatting (MUST-2).
pub struct ApiKeyMaterial {
    pub key_id: ApiKeyId,
    /// Ed25519 private key, hex-encoded — THIS IS THE API KEY
    pub private_key_hex: String,
    /// Public metadata about the key (limits, expiry, privacy mode)
    pub capability: ApiKeyCapability,
}

// contract: MUST-2
impl fmt::Debug for ApiKeyMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiKeyMaterial")
            .field("key_id", &self.key_id)
            .field("private_key_hex", &"[REDACTED]")
            .field("capability", &self.capability)
            .finish()
    }
}

// ── EncumbranceStatus — lifecycle of an rJoule lock ────────────────────────────

/// Status of an encumbrance (rJoule lock for API key allocation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncumbranceStatus {
    /// rJoules are locked and available for consumption.
    Active,
    /// All rJoules have been consumed (consumed_rj == amount_rj).
    Consumed,
    /// Encumbrance released — unspent rJ returned to wallet.
    Released,
}

impl fmt::Display for EncumbranceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Consumed => write!(f, "consumed"),
            Self::Released => write!(f, "released"),
        }
    }
}

impl FromStr for EncumbranceStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "consumed" => Ok(Self::Consumed),
            "released" => Ok(Self::Released),
            other => Err(format!("unknown encumbrance status: {other}")),
        }
    }
}

// ── Encumbrance — rJoule lock for API key allocations ──────────────────────────

/// An encumbrance — rJoules locked from a wallet for an API key's use.
///
/// Each API key has at most one active encumbrance. The encumbrance locks
/// rJoules against the wallet balance; as the key consumes gas, rJoules are
/// deducted from the encumbrance. Unspent rJoules are returned to the wallet
/// on release (key expiry or revocation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encumbrance {
    /// The API key this encumbrance funds (also serves as the encumbrance ID).
    pub key_id: ApiKeyId,
    pub wallet_id: WalletId,
    /// Total rJoules locked.
    pub amount_rj: u64,
    /// rJoules consumed so far.
    pub consumed_rj: u64,
    pub status: EncumbranceStatus,
    pub created_at: String,
    pub released_at: Option<String>,
}

impl Encumbrance {
    /// rJoules remaining in this encumbrance.
    pub fn remaining_rj(&self) -> u64 {
        self.amount_rj.saturating_sub(self.consumed_rj)
    }

    /// Whether this encumbrance is still active (can be consumed from).
    pub fn is_active(&self) -> bool {
        self.status == EncumbranceStatus::Active
    }
}
