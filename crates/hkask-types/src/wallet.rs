//! Wallet types for hKask — rJoule payments, multi-chain deposits, API key capabilities.
//!
//! # Epistemic frame (pragmatic-semantics)
//! - rJoule is an internal accounting unit `[OUGHT-DECL]` — not an on-chain token
//! - Every rJoule originates from a verified on-chain deposit `[IS-DECL]`
//! - API keys are Ed25519-signed OCAP capability tokens `[OUGHT-DECL]`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::InfrastructureError;
use crate::id::{ApiKeyId, WalletId};

// ── rJoule — stable value unit ────────────────────────────────────────────────

/// Replicated Joule — a stable value unit for hKask payments.
///
/// 1 rJoule ≈ 0.001 USDC (configurable via `WalletConfig.rj_per_usdc`).
/// Internal gas: 1 rJoule = configurable gas units (default: 1000 gas).
///
/// # Invariant `[OUGHT-DECL]`
/// `RJoule` values are always non-negative. Arithmetic saturates at 0 and `u64::MAX`.
///
/// # Provenance `[IS-DECL]`
/// Every `RJoule` in the system originates from a verified on-chain deposit
/// (`ChainPort::monitor_deposits`) or a shielded deposit
/// (`PrivacyPort::monitor_shielded_transfers`). No `RJoule` is created from thin air.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RJoule(pub u64);

impl RJoule {
    /// Zero rJoules — the additive identity.
    pub const ZERO: RJoule = RJoule(0);

    /// Create from raw u64. Infallible — zero is valid.
    pub fn new(value: u64) -> Self {
        RJoule(value)
    }

    /// Return the raw u64 value.
    pub fn as_u64(self) -> u64 {
        self.0
    }

    /// Saturating addition.
    pub fn saturating_add(self, other: RJoule) -> RJoule {
        RJoule(self.0.saturating_add(other.0))
    }

    /// Saturating subtraction — floors at zero.
    pub fn saturating_sub(self, other: RJoule) -> RJoule {
        RJoule(self.0.saturating_sub(other.0))
    }
}

impl fmt::Display for RJoule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} rJ", self.0)
    }
}

// ── ChainId — supported blockchain networks ────────────────────────────────────

/// Supported blockchain networks for deposits and withdrawals.
///
/// # Extensibility `[OUGHT-DECL]`
/// Adding a new chain requires: a variant here, a `ChainPort` implementation,
/// a treasury key derivation context, and a storage migration.
/// This is intentional — adding a chain is an architectural commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChainId {
    /// Solana — SPL USDC, ~$0.00001/tx, 400ms blocks, Hinkal privacy planned
    Solana,
    /// Hedera — HTS USDC, $0.0001/tx fixed, 2s blocks, deterministic finality
    Hedera,
}

impl fmt::Display for ChainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainId::Solana => write!(f, "solana"),
            ChainId::Hedera => write!(f, "hedera"),
        }
    }
}

impl FromStr for ChainId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "solana" => Ok(ChainId::Solana),
            "hedera" => Ok(ChainId::Hedera),
            other => Err(format!("unknown chain: {other}")),
        }
    }
}

// ── PrivacyMode — deposit and API key privacy level ────────────────────────────

/// Deposit and API key privacy mode.
///
/// # Semantic distinction from bool `[OUGHT-DECL]`
/// `PrivacyMode::Transparent` and `PrivacyMode::Shielded` carry meaning.
/// A bare `bool` (`is_private: true`) would be "boolean blindness" —
/// the reader must decode what `true` means at every use site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrivacyMode {
    /// Direct on-chain deposit/withdrawal — visible to public explorers
    Transparent,
    /// Via Hinkal Shielded Pool — wallet addresses and amounts not visible on-chain
    Shielded,
}

impl fmt::Display for PrivacyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrivacyMode::Transparent => write!(f, "transparent"),
            PrivacyMode::Shielded => write!(f, "shielded"),
        }
    }
}

impl FromStr for PrivacyMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "transparent" => Ok(PrivacyMode::Transparent),
            "shielded" => Ok(PrivacyMode::Shielded),
            other => Err(format!("unknown privacy mode: {other}")),
        }
    }
}

// ── Ed25519PublicKey — type-safe key material ──────────────────────────────────

/// Ed25519 public key — 32 bytes.
///
/// Newtype to prevent accidental mixing with other 32-byte values
/// (hashes, secrets, UUIDs). Conversion to/from `ed25519_dalek::VerifyingKey`
/// lives in `hkask-keystore` where the crypto dependency exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ed25519PublicKey(pub [u8; 32]);

impl Ed25519PublicKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Ed25519PublicKey(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for Ed25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

// ── DepositAddress — validated deposit destination ─────────────────────────────

/// A deposit address with chain and privacy metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepositAddress {
    pub address: String,
    pub chain: ChainId,
    pub privacy_mode: PrivacyMode,
}

impl fmt::Display for DepositAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.chain, self.address)
    }
}

// ── WalletConfig — wallet subsystem configuration ──────────────────────────────

/// Configuration for the wallet subsystem.
///
/// # Defaults `[OUGHT-DECL]`
/// - 1 USDC = 1000 rJoules
/// - 1 rJoule = 1000 gas units
/// - Both Solana and Hedera enabled
/// - Privacy disabled by default (opt-in per P2 Affirmative Consent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    /// rJoules credited per 1 USDC deposited (default: 1000)
    pub rj_per_usdc: u64,
    /// Gas units per rJoule (default: 1000)
    pub gas_per_rjoule: u64,
    /// Minimum deposit in micro-USDC (1 = 0.000001 USDC, default: 1_000_000 = $1.00)
    pub min_deposit_usdc_micro: u64,
    /// Supported chains
    pub enabled_chains: Vec<ChainId>,
    /// Whether the Hinkal privacy layer is enabled
    pub privacy_enabled: bool,
    /// Hinkal relayer endpoint URL (if privacy is enabled)
    pub hinkal_relayer_url: Option<String>,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            rj_per_usdc: 1000,
            gas_per_rjoule: 1000,
            min_deposit_usdc_micro: 1_000_000, // $1.00
            enabled_chains: vec![ChainId::Solana, ChainId::Hedera],
            privacy_enabled: false,
            hinkal_relayer_url: None,
        }
    }
}

// ── WalletBalance — current wallet state ───────────────────────────────────────

/// Current wallet balance with equivalents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    pub wallet_id: WalletId,
    /// rJoule balance
    pub rjoules: u64,
    /// Approximate USDC equivalent (rjoules / rj_per_usdc)
    pub usdc_equivalent_micro: u64,
    /// Gas equivalent (rjoules × gas_per_rjoule)
    pub gas_equivalent: u64,
}

impl fmt::Display for WalletBalance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} rJ  (~{:.6} USDC, ~{} gas)",
            self.rjoules,
            self.usdc_equivalent_micro as f64 / 1_000_000.0,
            self.gas_equivalent
        )
    }
}

// ── ApiKeyCapability — signed OCAP capability token ────────────────────────────

/// An API key is an Ed25519-signed capability token, not an opaque bearer string.
///
/// # OCAP alignment (P4) `[OUGHT-DECL]`
/// The capability carries its own attenuation: `spending_limit_rj`, `expiry`,
/// `privacy_mode`. The Ed25519 signature proves it was issued by a specific wallet.
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
    pub expiry: Option<DateTime<Utc>>,
    pub issued_at: DateTime<Utc>,
    pub privacy_mode: PrivacyMode,
    pub preferred_chain: Option<ChainId>,
}

impl ApiKeyCapability {
    /// Whether this key is currently active (not revoked, not expired).
    /// Revocation is tracked in storage (`revoked_at` timestamp), not on the capability itself.
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expiry.map_or(false, |exp| now > exp)
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
pub struct ApiKeyMaterial {
    pub key_id: ApiKeyId,
    /// Ed25519 private key, hex-encoded — THIS IS THE API KEY
    pub private_key_hex: String,
    /// Public metadata about the key (limits, expiry, privacy mode)
    pub capability: ApiKeyCapability,
}

// ── TransactionType — what kind of wallet event ────────────────────────────────

/// The type of a wallet transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    /// On-chain or shielded deposit detected
    Deposit {
        chain: ChainId,
        privacy: PrivacyMode,
        /// On-chain transaction hash (empty for shielded deposits)
        tx_hash: String,
        /// Amount in micro-USDC (1 = 0.000001 USDC)
        amount_usdc_micro: u64,
    },
    /// Withdrawal submitted
    Withdrawal {
        chain: ChainId,
        privacy: PrivacyMode,
        tx_hash: String,
        amount_usdc_micro: u64,
    },
    /// rJoules spent via an API key
    Spend {
        key_id: ApiKeyId,
        /// Tool that consumed the gas
        tool: String,
        /// Gas units consumed
        gas: u64,
        /// rJoules debited
        rj: RJoule,
    },
    /// rJoules refunded (e.g., on key revocation)
    Refund {
        key_id: ApiKeyId,
        reason: String,
        rj: RJoule,
    },
}

// ── WalletTransaction — a single entry in the append-only ledger ───────────────

/// A single wallet transaction — the append-only ledger entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransaction {
    pub id: u64,
    pub wallet_id: WalletId,
    pub tx_type: TransactionType,
    /// Positive = credit, negative = debit
    pub rjoules_delta: i64,
    /// Balance after this transaction
    pub balance_after: u64,
    pub timestamp: DateTime<Utc>,
}

// ── DepositReference — one-time shielded deposit identifier ────────────────────

/// A one-time reference for shielded (Hinkal) deposits.
///
/// # Privacy property `[IS-DECL]`
/// Derived via HKDF from the wallet seed + nonce + expiry.
/// Appears random on-chain but hKask can verify it belongs to a specific wallet.
///
/// # Anti-replay `[OUGHT-DECL]`
/// References are burned on use (consumed in `WalletStore`).
/// References expire after `validity_duration` (default 24h).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositReference {
    /// The reference string (hex-encoded HKDF output)
    pub reference: String,
    pub wallet_id: WalletId,
    pub chain: ChainId,
    /// Random nonce for uniqueness
    pub nonce: [u8; 16],
    /// When this reference expires
    pub expires_at: DateTime<Utc>,
}

impl fmt::Display for DepositReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "dep_{} (wallet: {}, chain: {}, expires: {})",
            self.reference, self.wallet_id, self.chain, self.expires_at
        )
    }
}

// ── WalletError — typed error domain ───────────────────────────────────────────

/// Wallet-specific error domain.
///
/// # Design principles (rust-expertise §7) `[OUGHT-DECL]`
/// - Typed errors for library code (`thiserror`)
/// - Each variant carries context, not just a name
/// - Never discard errors silently
#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("infrastructure error: {0}")]
    Infra(InfrastructureError),

    #[error("insufficient rJoule balance: have {have}, need {need}")]
    InsufficientBalance { have: RJoule, need: RJoule },

    #[error("API key {key_id} spending limit exceeded: {spent} / {limit}")]
    SpendingLimitExceeded {
        key_id: ApiKeyId,
        spent: RJoule,
        limit: RJoule,
    },

    #[error("API key {key_id} expired at {expiry}")]
    KeyExpired {
        key_id: ApiKeyId,
        expiry: DateTime<Utc>,
    },

    #[error("API key {key_id} has been revoked")]
    KeyRevoked { key_id: ApiKeyId },

    #[error("chain {chain} is not enabled for this wallet")]
    ChainNotEnabled { chain: ChainId },

    #[error("privacy layer unavailable for chain {chain}")]
    PrivacyUnavailable { chain: ChainId },

    #[error("deposit reference {reference} not found or expired")]
    DepositReferenceInvalid { reference: String },

    #[error("chain error ({chain}): {message}")]
    ChainError { chain: ChainId, message: String },

    #[error("privacy layer error: {message}")]
    PrivacyError { message: String },
}

impl From<InfrastructureError> for WalletError {
    fn from(e: InfrastructureError) -> Self {
        WalletError::Infra(e)
    }
}

#[cfg(feature = "sql")]
impl From<rusqlite::Error> for WalletError {
    fn from(e: rusqlite::Error) -> Self {
        WalletError::Infra(InfrastructureError::Database(e.to_string()))
    }
}

impl From<uuid::Error> for WalletError {
    fn from(e: uuid::Error) -> Self {
        WalletError::Infra(InfrastructureError::Database(e.to_string()))
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: P1-wallet-types — RJoule newtype prevents accidental mixing with gas units
    #[test]
    fn rjoule_newtype_prevents_gas_confusion() {
        let rj = RJoule::new(100);
        let gas: u64 = 100;
        // These cannot be compared or added without explicit conversion:
        // rj == gas  // compile error: can't compare RJoule with u64
        assert_eq!(rj.as_u64(), gas); // explicit conversion required
    }

    // REQ: P1-wallet-types — RJoule saturating_sub floors at zero
    #[test]
    fn rjoule_saturating_sub_floors_at_zero() {
        let a = RJoule::new(10);
        let b = RJoule::new(20);
        assert_eq!(a.saturating_sub(b), RJoule::ZERO);
    }

    // REQ: P1-wallet-types — ChainId FromStr is case-insensitive
    #[test]
    fn chain_id_from_str_case_insensitive() {
        assert_eq!("solana".parse::<ChainId>().unwrap(), ChainId::Solana);
        assert_eq!("SOLANA".parse::<ChainId>().unwrap(), ChainId::Solana);
        assert_eq!("hedera".parse::<ChainId>().unwrap(), ChainId::Hedera);
        assert!("bitcoin".parse::<ChainId>().is_err());
    }

    // REQ: P1-wallet-types — PrivacyMode is an enum, not a bool
    #[test]
    fn privacy_mode_is_exhaustive_enum() {
        let modes = [PrivacyMode::Transparent, PrivacyMode::Shielded];
        for mode in modes {
            match mode {
                PrivacyMode::Transparent => assert_eq!(mode.to_string(), "transparent"),
                PrivacyMode::Shielded => assert_eq!(mode.to_string(), "shielded"),
            }
        }
    }

    // REQ: P1-wallet-types — WalletId and ApiKeyId are distinct types
    #[test]
    fn wallet_id_and_api_key_id_are_distinct() {
        let wallet = WalletId::new();
        let key = ApiKeyId::new();
        // These cannot be assigned to each other:
        // let _: WalletId = key;  // compile error
        // let _: ApiKeyId = wallet;  // compile error
        assert_ne!(wallet.as_uuid(), key.as_uuid()); // different UUIDs
    }

    // REQ: P1-wallet-types — ApiKeyCapability tracks remaining budget
    #[test]
    fn api_key_capability_remaining_budget() {
        let cap = ApiKeyCapability {
            wallet_id: WalletId::new(),
            key_id: ApiKeyId::new(),
            public_key: Ed25519PublicKey([0u8; 32]),
            spending_limit_rj: RJoule::new(5000),
            spent_rj: RJoule::new(1200),
            expiry: None,
            issued_at: Utc::now(),
            privacy_mode: PrivacyMode::Transparent,
            preferred_chain: None,
        };
        assert_eq!(cap.remaining_rj(), RJoule::new(3800));
    }

    // REQ: P1-wallet-types — WalletConfig has sensible defaults
    #[test]
    fn wallet_config_defaults() {
        let cfg = WalletConfig::default();
        assert_eq!(cfg.rj_per_usdc, 1000);
        assert_eq!(cfg.gas_per_rjoule, 1000);
        assert_eq!(cfg.min_deposit_usdc_micro, 1_000_000);
        assert!(cfg.enabled_chains.contains(&ChainId::Solana));
        assert!(cfg.enabled_chains.contains(&ChainId::Hedera));
        assert!(!cfg.privacy_enabled); // opt-in per P2
    }

    // REQ: P1-wallet-types — WalletError Display impls are human-readable
    #[test]
    fn wallet_error_display_is_readable() {
        let err = WalletError::InsufficientBalance {
            have: RJoule::new(100),
            need: RJoule::new(500),
        };
        let msg = err.to_string();
        assert!(msg.contains("100"));
        assert!(msg.contains("500"));
        assert!(msg.contains("insufficient"));
    }
}
