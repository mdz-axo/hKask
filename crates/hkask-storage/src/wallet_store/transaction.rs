use crate::Store;
use super::types::*;
use hkask_types::{ApiKeyId, Ed25519PublicKey, InfrastructureError, WalletId};
use hkask_wallet_types::*;
use rusqlite::OptionalExtension;
use std::str::FromStr;

impl WalletStore {
    pub fn get_balance(&self, wallet_id: WalletId) -> Result<Option<WalletBalance>, WalletError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT wallet_id, balance_rj, usdc_equivalent_micro FROM wallet_balances WHERE wallet_id = ?1",
        )?;
        let rows: Vec<WalletBalance> = collect_rows_strict!(
            stmt,
            rusqlite::params![wallet_id.to_string()],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<WalletBalanceRow> {
                Ok(WalletBalanceRow {
                    wallet_id: row.get(0)?,
                    balance_rj: row.get(1)?,
                    usdc_equivalent_micro: row.get(2)?,
                })
            },
            |r: WalletBalanceRow| -> Result<WalletBalance, WalletError> {
                Ok(WalletBalance {
                    wallet_id: WalletId::from_str(&r.wallet_id)?,
                    rjoules: r.balance_rj as u64,
                    usdc_equivalent_micro: r.usdc_equivalent_micro as u64,
                    gas_equivalent: 0, // computed by caller with config
                })
            }
        );
        Ok(rows.into_iter().next())
    }
    /// Ensure a wallet row exists (idempotent — creates if missing).
    /// Takes an already-locked connection to avoid deadlock.
    fn ensure_wallet_with_conn(
        &self,
        conn: &rusqlite::Connection,
        wallet_id: WalletId,
    ) -> Result<(), WalletError> {
        conn.execute(
            "INSERT OR IGNORE INTO wallet_balances (wallet_id) VALUES (?1)",
            rusqlite::params![wallet_id.to_string()],
        )?;
        Ok(())
    }
    /// Ensure a wallet row exists (idempotent — creates if missing).
    /// Public version that acquires its own lock.
    /// Ensure a wallet exists (idempotent).
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — idempotently ensure wallet row
    /// pre:  wallet_id is valid
    /// post: wallet row exists (created if missing)
    pub fn ensure_wallet(&self, wallet_id: WalletId) -> Result<(), WalletError> {
        let conn = self.lock_conn()?;
        self.ensure_wallet_with_conn(&conn, wallet_id)
    }
    /// List all wallet IDs in the system.
    /// List all wallet IDs.
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P8\] Motivating: Semantic Grounding — list wallet IDs
    /// post: returns Vec of all WalletId
    pub fn list_wallet_ids(&self) -> Result<Vec<WalletId>, WalletError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT wallet_id FROM wallet_balances")?;
        let rows: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|s| WalletId::from_str(&s))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| WalletError::Infra(InfrastructureError::Database(e.to_string())))
    }
    /// Credit rJoules to a wallet. Returns the new balance.
    /// Creates the wallet row if it doesn't exist.
    /// Credit rJoules to a wallet.
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — credit rJoules
    /// pre:  wallet_id exists, amount > 0
    /// post: balance increased by amount, transaction recorded
    pub fn credit_rjoules(
        &self,
        wallet_id: WalletId,
        amount: RJoule,
    ) -> Result<WalletBalance, WalletError> {
        let conn = self.lock_conn()?;
        self.ensure_wallet_with_conn(&conn, wallet_id)?;
        let now = now_rfc3339();
        conn.execute(
            "UPDATE wallet_balances SET balance_rj = balance_rj + ?1, updated_at = ?2 WHERE wallet_id = ?3",
            rusqlite::params![amount.as_u64() as i64, now, wallet_id.to_string()],
        )?;
        drop(conn);
        self.get_balance(wallet_id)?
            .ok_or(WalletError::Infra(InfrastructureError::Database(
                "wallet vanished after credit".into(),
            )))
    }
    /// Debit rJoules from a wallet. Returns error if balance insufficient.
    /// The caller must verify `balance >= amount` before calling.
    ///
    /// **Idempotency:** This operation is NOT idempotent. Each call independently
    /// debits the balance. Callers MUST ensure that retries do not result in
    /// double-charging. For withdrawals, the caller (`WalletManager::withdraw`)
    /// refunds on chain submission failure via `credit_rjoules`. Callers that
    /// retry at a higher level MUST track whether the original debit succeeded
    /// before issuing a second debit.
    ///
    /// Debit rJoules from a wallet.
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — debit rJoules
    /// pre:  wallet_id exists, amount > 0, balance >= amount
    /// post: balance decreased by amount, transaction recorded
    /// post: returns Err if insufficient balance
    pub fn debit_rjoules(
        &self,
        wallet_id: WalletId,
        amount: RJoule,
    ) -> Result<WalletBalance, WalletError> {
        let conn = self.lock_conn()?;
        let current: i64 = conn.query_row(
            "SELECT balance_rj FROM wallet_balances WHERE wallet_id = ?1",
            rusqlite::params![wallet_id.to_string()],
            |row| row.get(0),
        )?;
        let amount_i64 = amount.as_u64() as i64;
        if current < amount_i64 {
            return Err(WalletError::InsufficientBalance {
                have: RJoule::new(current as u64),
                need: amount,
            });
        }
        let now = now_rfc3339();
        conn.execute(
            "UPDATE wallet_balances SET balance_rj = balance_rj - ?1, updated_at = ?2 WHERE wallet_id = ?3",
            rusqlite::params![amount_i64, now, wallet_id.to_string()],
        )?;
        drop(conn);
        self.get_balance(wallet_id)?
            .ok_or(WalletError::Infra(InfrastructureError::Database(
                "wallet vanished after debit".into(),
            )))
    }
    // ── Transactions ─────────────────────────────────────────────────────────
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
