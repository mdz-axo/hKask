use super::WalletStore;
use crate::collect_rows_strict;
use crate::Store;
use hkask_types::time::now_rfc3339;
use hkask_types::{InfrastructureError, WalletId};
use hkask_wallet_types::{RJoule, WalletBalance, WalletError};
use std::str::FromStr;

// ── Row type for query mapping ─────────────────────────────────────────────────

struct WalletBalanceRow {
    wallet_id: String,
    balance_rj: i64,
    usdc_equivalent_micro: i64,
}

// ── Balance & wallet lifecycle ─────────────────────────────────────────────────

impl WalletStore {
    /// Get the current balance for a wallet, or None if the wallet doesn't exist.
    /// Get wallet balance.
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — get wallet balance
    /// pre:  wallet_id is valid
    /// post: returns Some(WalletBalance) if wallet exists, None otherwise
    #[must_use = "result must be used"]
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
    #[must_use = "result must be used"]
    pub fn credit_rjoules(
        &self,
        wallet_id: WalletId,
        amount: RJoule,
    ) -> Result<WalletBalance, WalletError> {
        let conn = self.lock_conn()?;
        self.ensure_wallet_with_conn(&conn, wallet_id)?;
        let amount_u64 = amount.as_u64();
        let amount_i64 = i64::try_from(amount_u64).map_err(|_| {
            WalletError::Infra(InfrastructureError::Database(
                "credit amount exceeds i64::MAX".into(),
            ))
        })?;
        let current: i64 = conn.query_row(
            "SELECT balance_rj FROM wallet_balances WHERE wallet_id = ?1",
            rusqlite::params![wallet_id.to_string()],
            |row| row.get(0),
        )?;
        let new_balance = current.checked_add(amount_i64).ok_or_else(|| {
            WalletError::Infra(InfrastructureError::Database(
                "balance overflow on credit".into(),
            ))
        })?;
        let now = now_rfc3339();
        conn.execute(
            "UPDATE wallet_balances SET balance_rj = ?1, updated_at = ?2 WHERE wallet_id = ?3",
            rusqlite::params![new_balance, now, wallet_id.to_string()],
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
    #[must_use = "result must be used"]
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
        let amount_i64 = i64::try_from(amount.as_u64()).map_err(|_| {
            WalletError::Infra(InfrastructureError::Database(
                "debit amount exceeds i64::MAX".into(),
            ))
        })?;
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
}
