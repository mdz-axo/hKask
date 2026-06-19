//! Core wallet types — rJoule, balances, config, transactions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::chain::{ChainId, PrivacyMode};
pub use crate::id::{ApiKeyId, WalletId};

// ── rJoule — stable value unit ────────────────────────────────────────────────

/// Replicated Joule — a stable value unit for hKask payments.
///
/// 1 rJoule ≈ 0.001 USDC (configurable via `WalletConfig.rj_per_usdc`).
/// Internal gas: 1 rJoule = configurable gas units (default: 1000 gas).
///
/// # Invariant `[OUGHT-DECL]`
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

// ── WalletConfig — wallet subsystem configuration ──────────────────────────────

/// User-configurable price feed source selection.
///
/// # User sovereignty `[OUGHT-DECL]`
/// The user chooses which price sources to use and in what priority order.
/// No source is hardcoded — the wallet resolves the user's choice at build time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PriceFeedConfig {
    /// Hardcoded rates for development/testing (no network dependency).
    Static,
    /// EODHD API — primary canonical source (requires `HKASK_EODHD_API_KEY`).
    Eodhd,
    /// CoinGecko free public API (no API key required).
    CoinGecko,
    /// Composite: try sources in priority order, cache results, fall back on failure.
    Composite {
        /// Ordered list of source names: "eodhd", "coingecko".
        /// First successful source wins; subsequent sources are fallbacks.
        sources: Vec<String>,
        /// Cache TTL in seconds (default: 30).
        #[serde(default = "default_price_cache_ttl")]
        cache_ttl_secs: u64,
    },
}

fn default_price_cache_ttl() -> u64 {
    30
}

impl Default for PriceFeedConfig {
    fn default() -> Self {
        PriceFeedConfig::Composite {
            sources: vec!["eodhd".to_string(), "coingecko".to_string()],
            cache_ttl_secs: 30,
        }
    }
}

/// Configuration for the wallet subsystem.
///
/// # Defaults `[OUGHT-DECL]`
/// - 1 USDC = 1000 rJoules
/// - 1 rJoule = 1000 gas units
/// - Hinkal, Solana, and Hedera enabled
/// - Privacy enabled by default (shielded-first operation)
/// - Price feed: composite (EODHD → CoinGecko fallback) with 30s cache
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
    /// Price feed source configuration (user-selectable).
    #[serde(default)]
    pub price_feed: PriceFeedConfig,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            rj_per_usdc: 1000,
            gas_per_rjoule: 1000,
            min_deposit_usdc_micro: 1_000_000, // $1.00
            enabled_chains: vec![ChainId::Hinkal, ChainId::Solana, ChainId::Hedera],
            privacy_enabled: true,
            hinkal_relayer_url: None,
            price_feed: PriceFeedConfig::default(),
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
    /// Assets shielded into privacy pool (transparent → shielded movement).
    /// Does not affect rJoule balance — pure asset layer transition.
    Shield {
        chain: ChainId,
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

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::{ApiKeyId, WalletId};
    use crate::wallet::chain::{ChainId, Ed25519PublicKey, PrivacyMode};
    use crate::wallet::error::WalletError;
    use crate::wallet::keys::ApiKeyCapability;

    #[test]
    fn rjoule_newtype_prevents_gas_confusion() {
        let rj = RJoule::new(100);
        let gas: u64 = 100;
        // These cannot be compared or added without explicit conversion:
        // rj == gas  // compile error: can't compare RJoule with u64
        assert_eq!(rj.as_u64(), gas); // explicit conversion required
    }

    #[test]
    fn rjoule_saturating_sub_floors_at_zero() {
        let a = RJoule::new(10);
        let b = RJoule::new(20);
        assert_eq!(a.saturating_sub(b), RJoule::ZERO);
    }

    #[test]
    fn chain_id_from_str_case_insensitive() {
        assert_eq!("solana".parse::<ChainId>().unwrap(), ChainId::Solana);
        assert_eq!("SOLANA".parse::<ChainId>().unwrap(), ChainId::Solana);
        assert_eq!("hedera".parse::<ChainId>().unwrap(), ChainId::Hedera);
        assert!("bitcoin".parse::<ChainId>().is_err());
    }

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

    #[test]
    fn wallet_id_and_api_key_id_are_distinct() {
        let wallet = WalletId::new();
        let key = ApiKeyId::new();
        // These cannot be assigned to each other:
        // let _: WalletId = key;  // compile error
        // let _: ApiKeyId = wallet;  // compile error
        assert_ne!(wallet.as_uuid(), key.as_uuid()); // different UUIDs
    }

    #[test]
    fn api_key_capability_remaining_budget() {
        let cap = ApiKeyCapability {
            wallet_id: WalletId::new(),
            key_id: ApiKeyId::new(),
            public_key: Ed25519PublicKey([0u8; 32]),
            spending_limit_rj: RJoule::new(5000),
            spent_rj: RJoule::new(1200),
            scope: vec!["read-specs".to_string()],
            purpose: "test capability".to_string(),
            rate_limit: None,
            expiry: None,
            issued_at: Utc::now(),
            privacy_mode: PrivacyMode::Transparent,
            preferred_chain: None,
        };
        assert_eq!(cap.remaining_rj(), RJoule::new(3800));
    }

    #[test]
    fn wallet_config_defaults() {
        let cfg = WalletConfig::default();
        assert_eq!(cfg.rj_per_usdc, 1000);
        assert_eq!(cfg.gas_per_rjoule, 1000);
        assert_eq!(cfg.min_deposit_usdc_micro, 1_000_000);
        assert!(cfg.enabled_chains.contains(&ChainId::Hinkal));
        assert!(cfg.enabled_chains.contains(&ChainId::Solana));
        assert!(cfg.enabled_chains.contains(&ChainId::Hedera));
        assert!(cfg.privacy_enabled);
    }

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
