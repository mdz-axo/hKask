use super::WalletStore;
use crate::Store;
use crate::collect_rows_strict;
use hkask_types::{ApiKeyId, InfrastructureError, WalletId};
use hkask_wallet_types::{
    ChainId, PrivacyMode, RJoule, TransactionType, WalletError, WalletTransaction,
};
use std::str::FromStr;

// ── Row type for query mapping ─────────────────────────────────────────────────

struct WalletTransactionRow {
    id: i64,
    wallet_id: String,
    tx_type: String,
    tx_subtype: Option<String>,
    chain: Option<String>,
    on_chain_tx_hash: Option<String>,
    amount_rj: i64,
    balance_after_rj: i64,
    key_id: Option<String>,
    tool_name: Option<String>,
    gas_units: Option<i64>,
    created_at: String,
}

// ── Transaction methods ────────────────────────────────────────────────────────

impl WalletStore {
    /// Record a transaction in the append-only ledger.
    /// Record a wallet transaction.
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — record wallet transaction
    /// pre:  tx has valid wallet_id and rjoules_delta
    /// post: transaction inserted into ledger
    pub fn record_transaction(&self, tx: &WalletTransaction) -> Result<(), WalletError> {
        let conn = self.lock_conn()?;
        let (tx_type_str, tx_subtype, chain, tx_hash, key_id, tool_name, gas_units) =
            tx_type_to_columns(&tx.tx_type);
        conn.execute(
            "INSERT INTO wallet_transactions (wallet_id, tx_type, tx_subtype, chain, on_chain_tx_hash, amount_rj, balance_after_rj, key_id, tool_name, gas_units) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                tx.wallet_id.to_string(),
                tx_type_str,
                tx_subtype,
                chain,
                tx_hash,
                tx.rjoules_delta,
                tx.balance_after as i64,
                key_id,
                tool_name,
                gas_units,
            ],
        )?;
        Ok(())
    }

    /// Get paginated transaction history for a wallet.
    /// Get transactions for a wallet.
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — list transactions
    /// pre:  wallet_id is valid
    /// post: returns Vec of transactions, optionally limited
    #[must_use = "result must be used"]
    pub fn get_transactions(
        &self,
        wallet_id: WalletId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<WalletTransaction>, WalletError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, wallet_id, tx_type, tx_subtype, chain, on_chain_tx_hash, amount_rj, balance_after_rj, key_id, tool_name, gas_units, created_at FROM wallet_transactions WHERE wallet_id = ?1 ORDER BY id DESC LIMIT ?2 OFFSET ?3",
        )?;
        let rows: Vec<WalletTransaction> = collect_rows_strict!(
            stmt,
            rusqlite::params![wallet_id.to_string(), limit, offset],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<WalletTransactionRow> {
                Ok(WalletTransactionRow {
                    id: row.get(0)?,
                    wallet_id: row.get(1)?,
                    tx_type: row.get(2)?,
                    tx_subtype: row.get(3)?,
                    chain: row.get(4)?,
                    on_chain_tx_hash: row.get(5)?,
                    amount_rj: row.get(6)?,
                    balance_after_rj: row.get(7)?,
                    key_id: row.get(8)?,
                    tool_name: row.get(9)?,
                    gas_units: row.get(10)?,
                    created_at: row.get(11)?,
                })
            },
            |r: WalletTransactionRow| -> Result<WalletTransaction, WalletError> {
                row_to_wallet_transaction(r)
            }
        );
        Ok(rows)
    }

    /// Check if a transaction with the given on-chain tx_hash already exists.
    /// Used for deposit idempotency — prevents double-crediting on restart.
    /// Check if a transaction hash exists.
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P4\] Motivating: Clear Boundaries — anti-replay hash check
    /// pre:  tx_hash is non-empty
    /// post: returns true if hash exists (anti-replay)
    #[must_use = "result must be used"]
    pub fn transaction_exists_by_hash(&self, tx_hash: &str) -> Result<bool, WalletError> {
        let conn = self.lock_conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM wallet_transactions WHERE on_chain_tx_hash = ?1",
            rusqlite::params![tx_hash],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

// ── Row conversion helpers ─────────────────────────────────────────────────────

type TxTypeColumns = (
    &'static str,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i64>,
);

fn tx_type_to_columns(tx_type: &TransactionType) -> TxTypeColumns {
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
