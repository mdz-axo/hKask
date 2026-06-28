//! WalletStore — SQLite-backed persistence for rJoule balances, transactions, API keys.
//!
//! # Schema (5 tables)
//! - `wallet_balances` — one row per wallet, current rJoule balance
//! - `wallet_transactions` — append-only ledger of all balance changes
//! - `api_keys` — issued Ed25519 capability tokens with spending limits
//! - `deposit_addresses` — derived deposit addresses per wallet per chain
//! - `deposit_references` — one-time shielded deposit references (anti-replay)
use crate::Store;
use hkask_types::time::now_rfc3339;
use hkask_types::{ApiKeyId, Ed25519PublicKey, InfrastructureError, WalletId};
use hkask_wallet_types::{
    ApiKeyCapability, ChainId, DepositAddress, DepositReference, Encumbrance, EncumbranceStatus,
    PrivacyMode, RJoule, RateLimitConfig, TransactionType, TxHash, WalletBalance, WalletConfig,
    WalletError, WalletTransaction,
};
use rusqlite::OptionalExtension;
use std::str::FromStr;

define_store!(WalletStore);

// ── Row types for query mapping ────────────────────────────────────────────────

#[allow(dead_code)] // fields populated by rusqlite query mapping
pub(crate) struct WalletBalanceRow {
    pub wallet_id: String,
    pub balance_rj: i64,
    pub usdc_equivalent_micro: i64,
}

pub(crate) struct WalletTransactionRow {
    pub id: i64,
    pub wallet_id: String,
    pub tx_type: String,
    pub tx_subtype: Option<String>,
    pub chain: Option<String>,
    pub on_chain_tx_hash: Option<String>,
    pub amount_rj: i64,
    pub balance_after_rj: i64,
    pub key_id: Option<String>,
    pub tool_name: Option<String>,
    pub gas_units: Option<i64>,
    pub created_at: String,
}

#[allow(dead_code)] // fields populated by rusqlite query mapping
pub(crate) struct ApiKeyRow {
    pub key_id: String,
    pub wallet_id: String,
    pub public_key: Vec<u8>,
    pub spending_limit_rj: i64,
    pub spent_rj: i64,
    pub scope: String,
    pub purpose: String,
    pub rate_limit_json: Option<String>,
    pub privacy_mode: String,
    pub preferred_chain: Option<String>,
    pub expires_at: Option<String>,
    pub issued_at: String,
}

#[allow(dead_code)] // fields populated by rusqlite query mapping
pub(crate) struct DepositAddressRow {
    pub chain: String,
    pub address: String,
    pub privacy_mode: String,
}

// ── Row conversion helpers ─────────────────────────────────────────────────────

pub(crate) type TxTypeColumns = (
    &'static str,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i64>,
);

pub(crate) fn tx_type_to_columns(tx_type: &TransactionType) -> TxTypeColumns {
    match tx_type {
        TransactionType::Deposit { tx_hash, .. } => (
            "deposit",
            Some("transparent".to_string()),
            Some("hedera".to_string()),
            Some(tx_hash.clone()),
            None,
            None,
            None,
        ),
        TransactionType::Withdrawal { tx_hash, .. } => (
            "withdrawal",
            Some("transparent".to_string()),
            Some("hedera".to_string()),
            Some(tx_hash.clone()),
            None,
            None,
            None,
        ),
        TransactionType::Spend {
            key_id, tool, gas, ..
        } => (
            "spend",
            None,
            None,
            None,
            Some(key_id.to_string()),
            Some(tool.clone()),
            Some(*gas as i64),
        ),
        TransactionType::Refund { key_id, reason, .. } => (
            "refund",
            None,
            None,
            None,
            Some(key_id.to_string()),
            Some(reason.clone()),
            None,
        ),
        TransactionType::Shield { chain, tx_hash, .. } => (
            "shield",
            None,
            Some(chain.to_string()),
            Some(tx_hash.clone()),
            None,
            None,
            None,
        ),
    }
}

pub(crate) fn row_to_wallet_transaction(
    r: WalletTransactionRow,
) -> Result<WalletTransaction, WalletError> {
    let tx_type = match r.tx_type.as_str() {
        "deposit" => TransactionType::Deposit {
            chain: ChainId::from_str(r.chain.as_deref().unwrap_or("hedera"))
                .map_err(|e| WalletError::Infra(InfrastructureError::Database(e)))?,
            privacy: PrivacyMode::from_str(r.tx_subtype.as_deref().unwrap_or("transparent"))
                .map_err(|e| WalletError::Infra(InfrastructureError::Database(e)))?,
            tx_hash: r.on_chain_tx_hash.unwrap_or_default(),
            amount_usdc_micro: 0,
        },
        "withdrawal" => TransactionType::Withdrawal {
            chain: ChainId::from_str(r.chain.as_deref().unwrap_or("hedera"))
                .map_err(|e| WalletError::Infra(InfrastructureError::Database(e)))?,
            privacy: PrivacyMode::from_str(r.tx_subtype.as_deref().unwrap_or("transparent"))
                .map_err(|e| WalletError::Infra(InfrastructureError::Database(e)))?,
            tx_hash: r.on_chain_tx_hash.unwrap_or_default(),
            amount_usdc_micro: 0,
        },
        "spend" => TransactionType::Spend {
            key_id: ApiKeyId::from_str(r.key_id.as_deref().unwrap_or(""))
                .map_err(|e| WalletError::Infra(InfrastructureError::Database(e.to_string())))?,
            tool: r.tool_name.unwrap_or_default(),
            gas: r.gas_units.unwrap_or(0) as u64,
            rj: RJoule::new(r.amount_rj.unsigned_abs()),
        },
        "refund" => TransactionType::Refund {
            key_id: ApiKeyId::from_str(r.key_id.as_deref().unwrap_or(""))
                .map_err(|e| WalletError::Infra(InfrastructureError::Database(e.to_string())))?,
            reason: r.tool_name.unwrap_or_default(),
            rj: RJoule::new(r.amount_rj.unsigned_abs()),
        },
        "shield" => TransactionType::Shield {
            chain: ChainId::from_str(r.chain.as_deref().unwrap_or("hedera"))
                .map_err(|e| WalletError::Infra(InfrastructureError::Database(e)))?,
            tx_hash: r.on_chain_tx_hash.unwrap_or_default(),
            amount_usdc_micro: 0,
        },
        other => {
            return Err(WalletError::Infra(InfrastructureError::Database(format!(
                "unknown tx_type: {other}"
            ))));
        }
    };
    Ok(WalletTransaction {
        id: r.id as u64,
        wallet_id: WalletId::from_str(&r.wallet_id)?,
        tx_type,
        rjoules_delta: r.amount_rj,
        balance_after: r.balance_after_rj as u64,
        timestamp: chrono::NaiveDateTime::parse_from_str(&r.created_at, "%Y-%m-%d %H:%M:%S")
            .map(|dt| dt.and_utc())
            .map_err(|e| WalletError::Infra(InfrastructureError::Database(e.to_string())))?,
    })
}

pub(crate) fn row_to_api_key_capability(r: ApiKeyRow) -> Result<ApiKeyCapability, WalletError> {
    let public_key_bytes: [u8; 32] = r.public_key.try_into().map_err(|_| {
        WalletError::Infra(InfrastructureError::Database(
            "public_key must be 32 bytes".into(),
        ))
    })?;
    let scope: Vec<String> = serde_json::from_str(&r.scope).unwrap_or_default();
    let rate_limit: Option<RateLimitConfig> = r
        .rate_limit_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok());
    Ok(ApiKeyCapability {
        wallet_id: WalletId::from_str(&r.wallet_id)?,
        key_id: ApiKeyId::from_str(&r.key_id)?,
        public_key: Ed25519PublicKey(public_key_bytes),
        spending_limit_rj: RJoule::new(r.spending_limit_rj as u64),
        spent_rj: RJoule::new(r.spent_rj as u64),
        scope,
        purpose: r.purpose,
        rate_limit,
        expiry: r.expires_at.map(|e| {
            chrono::DateTime::parse_from_rfc3339(&e)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now())
        }),
        issued_at: chrono::DateTime::parse_from_rfc3339(&r.issued_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        privacy_mode: PrivacyMode::Transparent,
        preferred_chain: None,
    })
}
