use super::types::*;
use hkask_types::time::now_rfc3339;
use hkask_types::{ApiKeyId, Ed25519PublicKey, WalletId};
use hkask_wallet_types::*;
use std::str::FromStr;

fn row_to_wallet_transaction(r: WalletTransactionRow) -> Result<WalletTransaction, WalletError> {
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
fn row_to_api_key_capability(r: ApiKeyRow) -> Result<ApiKeyCapability, WalletError> {
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
