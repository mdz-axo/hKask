use super::WalletStore;
use crate::Store;
use hkask_types::time::now_rfc3339;
use hkask_types::{ApiKeyId, InfrastructureError, WalletId};
use hkask_wallet_types::{Encumbrance, EncumbranceStatus, RJoule, WalletError};
use rusqlite::OptionalExtension;
use std::str::FromStr;

// ── Encumbrance methods ────────────────────────────────────────────────────────

impl WalletStore {
    /// Lock rJoules from a wallet for an API key's use.
    ///
    /// Debits the wallet balance by `amount_rj` and creates an active
    /// encumbrance row. Returns an error if the key already has an active
    /// encumbrance or the wallet has insufficient balance.
    /// Encumber rJoules for an API key (lock funds for spending).
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — encumber rJoules for key
    /// pre:  wallet_id exists, key_id is valid, amount > 0, balance >= amount
    /// post: rJoules encumbered, balance decreased
    pub fn encumber_rjoules(
        &self,
        wallet_id: WalletId,
        key_id: ApiKeyId,
        amount_rj: RJoule,
    ) -> Result<(), WalletError> {
        let conn = self.lock_conn()?;
        let now = now_rfc3339();
        let amount = amount_rj.as_u64() as i64;
        // Check no existing active encumbrance for this key
        let existing: Option<String> = conn
            .query_row(
                "SELECT status FROM encumbrances WHERE key_id = ?1",
                rusqlite::params![key_id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(status) = existing
            && status == "active"
        {
            return Err(WalletError::EncumbranceAlreadyExists { key_id });
        }
        // Debit wallet
        let rows = conn.execute(
            "UPDATE wallet_balances SET balance_rj = balance_rj - ?1, updated_at = ?2 WHERE wallet_id = ?3 AND balance_rj >= ?1",
            rusqlite::params![amount, now, wallet_id.to_string()],
        )?;
        if rows == 0 {
            let balance = self.get_balance(wallet_id)?;
            let have = balance.map(|b| b.rjoules).unwrap_or(0);
            return Err(WalletError::InsufficientBalance {
                have: RJoule::new(have),
                need: amount_rj,
            });
        }
        // Create encumbrance row
        conn.execute(
            "INSERT INTO encumbrances (key_id, wallet_id, amount_rj, consumed_rj, status, created_at) VALUES (?1, ?2, ?3, 0, 'active', ?4)",
            rusqlite::params![key_id.to_string(), wallet_id.to_string(), amount, now],
        )?;
        Ok(())
    }

    /// Release an encumbrance, returning unspent rJoules to the wallet.
    ///
    /// Idempotent — releasing an already-released or consumed encumbrance
    /// is a no-op.
    /// Release an encumbrance (return unspent rJoules to wallet).
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — release encumbrance
    /// pre:  key_id has active encumbrance
    /// post: encumbrance released, unspent rJ returned to wallet
    pub fn release_encumbrance(&self, key_id: ApiKeyId) -> Result<(), WalletError> {
        let conn = self.lock_conn()?;
        let now = now_rfc3339();
        // Read current state
        let row: Option<(String, i64, i64)> = conn
            .query_row(
                "SELECT wallet_id, amount_rj, consumed_rj FROM encumbrances WHERE key_id = ?1 AND status = 'active'",
                rusqlite::params![key_id.to_string()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)),
            )
            .optional()?;
        let (wallet_id_str, amount, consumed) = match row {
            Some(r) => r,
            None => return Ok(()), // already released/consumed or doesn't exist
        };
        // Mark released
        conn.execute(
            "UPDATE encumbrances SET status = 'released', released_at = ?1 WHERE key_id = ?2 AND status = 'active'",
            rusqlite::params![now, key_id.to_string()],
        )?;
        // Return unspent rJoules to wallet
        let unspent = amount - consumed;
        if unspent > 0 {
            conn.execute(
                "UPDATE wallet_balances SET balance_rj = balance_rj + ?1, updated_at = ?2 WHERE wallet_id = ?3",
                rusqlite::params![unspent, now, wallet_id_str],
            )?;
        }
        Ok(())
    }

    /// Atomically consume rJoules from an active encumbrance.
    ///
    /// This is a single SQL UPDATE that checks `amount_rj - consumed_rj >= cost`
    /// and deducts. No separate check+deduct pair — the operation is atomic.
    /// If the encumbrance is fully consumed, status transitions to 'consumed'.
    /// Consume from an encumbrance (spend locked rJoules).
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — consume from encumbrance
    /// pre:  key_id has active encumbrance with sufficient remaining
    /// post: consumed_rj increased, api_keys.spent_rj synced
    /// post: returns Err if insufficient or not active
    pub fn consume_encumbrance(
        &self,
        key_id: ApiKeyId,
        cost_rj: RJoule,
    ) -> Result<(), WalletError> {
        let conn = self.lock_conn()?;
        let cost = cost_rj.as_u64() as i64;
        // Atomic consume
        let rows = conn.execute(
            "UPDATE encumbrances SET consumed_rj = consumed_rj + ?1 WHERE key_id = ?2 AND status = 'active' AND (amount_rj - consumed_rj) >= ?1",
            rusqlite::params![cost, key_id.to_string()],
        )?;
        if rows == 0 {
            return Self::diagnose_consume_failure(&conn, key_id, cost_rj);
        }
        // Sync api_keys.spent_rj
        conn.execute(
            "UPDATE api_keys SET spent_rj = spent_rj + ?1 WHERE key_id = ?2",
            rusqlite::params![cost, key_id.to_string()],
        )?;
        // Transition status if fully consumed
        conn.execute(
            "UPDATE encumbrances SET status = 'consumed', released_at = ?1 WHERE key_id = ?2 AND status = 'active' AND consumed_rj >= amount_rj",
            rusqlite::params![now_rfc3339(), key_id.to_string()],
        )?;
        Ok(())
    }

    fn diagnose_consume_failure(
        conn: &rusqlite::Connection,
        key_id: ApiKeyId,
        cost_rj: RJoule,
    ) -> Result<(), WalletError> {
        let enc_row: Option<(String, i64, i64, String)> = conn
            .query_row(
                "SELECT wallet_id, amount_rj, consumed_rj, status FROM encumbrances WHERE key_id = ?1",
                rusqlite::params![key_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        match enc_row {
            Some((_wallet_id_str, amount, consumed, status_str)) => {
                let status = EncumbranceStatus::from_str(&status_str)
                    .map_err(|e| WalletError::Infra(InfrastructureError::Database(e)))?;
                if status != EncumbranceStatus::Active {
                    return Err(WalletError::EncumbranceNotFound { key_id });
                }
                let remaining = (amount as u64).saturating_sub(consumed as u64);
                Err(WalletError::EncumbranceInsufficient {
                    key_id,
                    remaining: RJoule::new(remaining),
                    need: cost_rj,
                })
            }
            None => Err(WalletError::EncumbranceNotFound { key_id }),
        }
    }

    /// Get an encumbrance by key ID.
    /// Get an encumbrance by key ID.
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — get encumbrance
    /// pre:  key_id is valid
    /// post: returns Some(Encumbrance) if found, None otherwise
    #[must_use = "result must be used"]
    pub fn get_encumbrance(&self, key_id: ApiKeyId) -> Result<Option<Encumbrance>, WalletError> {
        let conn = self.lock_conn()?;
        let row: Option<(String, i64, i64, String, String, Option<String>)> = conn
            .query_row(
                "SELECT wallet_id, amount_rj, consumed_rj, status, created_at, released_at FROM encumbrances WHERE key_id = ?1",
                rusqlite::params![key_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .optional()?;
        match row {
            Some((wallet_id_str, amount, consumed, status_str, created_at, released_at)) => {
                let wallet_id = WalletId::from_str(&wallet_id_str).map_err(|e| {
                    WalletError::Infra(InfrastructureError::Database(e.to_string()))
                })?;
                let status = EncumbranceStatus::from_str(&status_str)
                    .map_err(|e| WalletError::Infra(InfrastructureError::Database(e)))?;
                Ok(Some(Encumbrance {
                    key_id,
                    wallet_id,
                    amount_rj: amount as u64,
                    consumed_rj: consumed as u64,
                    status,
                    created_at,
                    released_at,
                }))
            }
            None => Ok(None),
        }
    }
}
