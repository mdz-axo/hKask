//! Wallet types — split from hkask-types to slim the foundation crate.
//!
//! These types are needed by hkask-storage (WalletStore) which sits below
//! hkask-wallet in the dependency chain. The hkask-wallet crate re-exports
//! them so downstream code can use `hkask_wallet::ChainId` etc.
//!
//! # Epistemic frame (pragmatic-semantics)
//! - rJoule is an internal accounting unit `[OUGHT-DECL]` — not an on-chain token
//! - Every rJoule originates from a verified on-chain deposit `[IS-DECL]`
//! - API keys are Ed25519-signed OCAP capability tokens `[OUGHT-DECL]`

use chrono::{DateTime, Utc};
use hkask_types::{ApiKeyId, Ed25519PublicKey, InfrastructureError, WalletId};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// ── ChainId — supported blockchain networks ────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChainId {
    Solana,
    Hedera,
    Hinkal,
}

impl fmt::Display for ChainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainId::Solana => write!(f, "solana"),
            ChainId::Hedera => write!(f, "hedera"),
            ChainId::Hinkal => write!(f, "hinkal"),
        }
    }
}

impl FromStr for ChainId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "solana" => Ok(ChainId::Solana),
            "hedera" => Ok(ChainId::Hedera),
            "hinkal" => Ok(ChainId::Hinkal),
            other => Err(format!("unknown chain: {other}")),
        }
    }
}

// ── PrivacyMode ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrivacyMode {
    Transparent,
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

// ── TxHash ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TxHash(pub String);

impl fmt::Display for TxHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── DepositAddress ─────────────────────────────────────────────────────────────

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

// ── DepositReference ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositReference {
    pub reference: String,
    pub wallet_id: WalletId,
    pub chain: ChainId,
    pub nonce: [u8; 16],
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

// ── RJoule — stable value unit ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RJoule(pub u64);

impl RJoule {
    pub const ZERO: RJoule = RJoule(0);

    pub fn new(value: u64) -> Self {
        RJoule(value)
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }

    pub fn saturating_add(self, other: RJoule) -> RJoule {
        RJoule(self.0.saturating_add(other.0))
    }

    pub fn saturating_sub(self, other: RJoule) -> RJoule {
        RJoule(self.0.saturating_sub(other.0))
    }
}

impl fmt::Display for RJoule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} rJ", self.0)
    }
}

// ── PriceFeedConfig ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PriceFeedConfig {
    Static,
    Eodhd,
    CoinGecko,
    Composite {
        sources: Vec<String>,
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

// ── WalletConfig ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub rj_per_usdc: u64,
    pub gas_per_rjoule: u64,
    pub min_deposit_usdc_micro: u64,
    pub enabled_chains: Vec<ChainId>,
    pub privacy_enabled: bool,
    pub hinkal_relayer_url: Option<String>,
    #[serde(default)]
    pub price_feed: PriceFeedConfig,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            rj_per_usdc: 1000,
            gas_per_rjoule: 1000,
            min_deposit_usdc_micro: 1_000_000,
            enabled_chains: vec![ChainId::Hinkal, ChainId::Solana, ChainId::Hedera],
            privacy_enabled: true,
            hinkal_relayer_url: None,
            price_feed: PriceFeedConfig::default(),
        }
    }
}

// ── WalletBalance ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    pub wallet_id: WalletId,
    pub rjoules: u64,
    pub usdc_equivalent_micro: u64,
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

// ── TransactionType ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Deposit {
        chain: ChainId,
        privacy: PrivacyMode,
        tx_hash: String,
        amount_usdc_micro: u64,
    },
    Withdrawal {
        chain: ChainId,
        privacy: PrivacyMode,
        tx_hash: String,
        amount_usdc_micro: u64,
    },
    Shield {
        chain: ChainId,
        tx_hash: String,
        amount_usdc_micro: u64,
    },
    Spend {
        key_id: ApiKeyId,
        tool: String,
        gas: u64,
        rj: RJoule,
    },
    Refund {
        key_id: ApiKeyId,
        reason: String,
        rj: RJoule,
    },
}

// ── WalletTransaction ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransaction {
    pub id: u64,
    pub wallet_id: WalletId,
    pub tx_type: TransactionType,
    pub rjoules_delta: i64,
    pub balance_after: u64,
    pub timestamp: DateTime<Utc>,
}

// ── WalletError ────────────────────────────────────────────────────────────────

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

    #[error("API key {key_id} already has an active encumbrance")]
    EncumbranceAlreadyExists { key_id: ApiKeyId },

    #[error("no active encumbrance found for API key {key_id}")]
    EncumbranceNotFound { key_id: ApiKeyId },

    #[error(
        "encumbrance for key {key_id} has insufficient remaining: have {remaining}, need {need}"
    )]
    EncumbranceInsufficient {
        key_id: ApiKeyId,
        remaining: RJoule,
        need: RJoule,
    },
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

// ── RateLimitConfig ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub tokens_per_day: u64,
}

// ── ApiKeyCapability ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyCapability {
    pub wallet_id: WalletId,
    pub key_id: ApiKeyId,
    pub public_key: Ed25519PublicKey,
    pub spending_limit_rj: RJoule,
    pub spent_rj: RJoule,
    pub scope: Vec<String>,
    pub purpose: String,
    pub rate_limit: Option<RateLimitConfig>,
    pub expiry: Option<DateTime<Utc>>,
    pub issued_at: DateTime<Utc>,
    pub privacy_mode: PrivacyMode,
    pub preferred_chain: Option<ChainId>,
}

impl ApiKeyCapability {
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expiry.is_some_and(|exp| now > exp)
    }

    pub fn remaining_rj(&self) -> RJoule {
        self.spending_limit_rj.saturating_sub(self.spent_rj)
    }
}

// ── ApiKeyMaterial ─────────────────────────────────────────────────────────────

pub struct ApiKeyMaterial {
    pub key_id: ApiKeyId,
    pub private_key_hex: String,
    pub capability: ApiKeyCapability,
}

impl fmt::Debug for ApiKeyMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiKeyMaterial")
            .field("key_id", &self.key_id)
            .field("private_key_hex", &"[REDACTED]")
            .field("capability", &self.capability)
            .finish()
    }
}

// ── EncumbranceStatus ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncumbranceStatus {
    Active,
    Consumed,
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

// ── Encumbrance ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encumbrance {
    pub key_id: ApiKeyId,
    pub wallet_id: WalletId,
    pub amount_rj: u64,
    pub consumed_rj: u64,
    pub status: EncumbranceStatus,
    pub created_at: String,
    pub released_at: Option<String>,
}

impl Encumbrance {
    pub fn remaining_rj(&self) -> u64 {
        self.amount_rj.saturating_sub(self.consumed_rj)
    }

    pub fn is_active(&self) -> bool {
        self.status == EncumbranceStatus::Active
    }
}
