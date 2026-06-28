use crate::Store;
use super::types::*;
impl WalletStore {
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
// ── Tests ──────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::in_memory_db;
    fn make_store() -> WalletStore {
        let db = in_memory_db();
        WalletStore::new(db.conn_arc())
    }
    #[test]
    fn enable_wal_mode_succeeds() {
        let store = make_store();
        // WAL mode should succeed on in-memory databases (no-op but no error)
        let result = store.enable_wal_mode();
        assert!(
            result.is_ok(),
            "WAL mode enable should succeed: {:?}",
            result
        );
    }
    #[test]
    fn credit_rjoules_increases_balance() {
        let store = make_store();
        let wallet = WalletId::new();
        let balance = store.credit_rjoules(wallet, RJoule::new(1000)).unwrap();
        assert_eq!(balance.rjoules, 1000);
    }
    #[test]
    fn debit_rjoules_decreases_balance() {
        let store = make_store();
        let wallet = WalletId::new();
        store.credit_rjoules(wallet, RJoule::new(1000)).unwrap();
        let balance = store.debit_rjoules(wallet, RJoule::new(300)).unwrap();
        assert_eq!(balance.rjoules, 700);
    }
    #[test]
    fn debit_rjoules_rejects_insufficient_balance() {
        let store = make_store();
        let wallet = WalletId::new();
        store.credit_rjoules(wallet, RJoule::new(100)).unwrap();
        let err = store.debit_rjoules(wallet, RJoule::new(500)).unwrap_err();
        assert!(matches!(err, WalletError::InsufficientBalance { .. }));
    }
    #[test]
    fn balance_never_negative() {
        let store = make_store();
        let wallet = WalletId::new();
        store.credit_rjoules(wallet, RJoule::new(50)).unwrap();
        // Debit exactly the balance
        let balance = store.debit_rjoules(wallet, RJoule::new(50)).unwrap();
        assert_eq!(balance.rjoules, 0);
        // Debit more should fail
        assert!(store.debit_rjoules(wallet, RJoule::new(1)).is_err());
    }
    #[test]
    fn transaction_ledger_is_append_only() {
        let store = make_store();
        let wallet = WalletId::new();
        store.credit_rjoules(wallet, RJoule::new(1000)).unwrap();
        let balance = store.get_balance(wallet).unwrap().unwrap();
        let tx = WalletTransaction {
            id: 0, // auto-increment, ignored on insert
            wallet_id: wallet,
            tx_type: TransactionType::Deposit {
                chain: ChainId::default(),
                privacy: PrivacyMode::default(),
                tx_hash: "test_tx".into(),
                amount_usdc_micro: 1_000_000,
            },
            rjoules_delta: 1000,
            balance_after: balance.rjoules,
            timestamp: chrono::Utc::now(),
        };
        store.record_transaction(&tx).unwrap();
        let txs = store.get_transactions(wallet, 10, 0).unwrap();
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].rjoules_delta, 1000);
    }
    #[test]
    fn deposit_reference_anti_replay() {
        let store = make_store();
        let wallet = WalletId::new();
        store.ensure_wallet(wallet).unwrap();
        let dep_ref = DepositReference {
            chain: ChainId::default(),
            reference: "test_ref_001".into(),
            wallet_id: wallet,
            nonce: [0u8; 16],
            expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
        };
        store.store_deposit_reference(&dep_ref).unwrap();
        // First consumption succeeds
        let result = store.consume_deposit_reference("test_ref_001").unwrap();
        assert_eq!(result, Some(wallet));
        // Second consumption fails (already spent)
        let result2 = store.consume_deposit_reference("test_ref_001").unwrap();
        assert_eq!(result2, None);
    }
    #[test]
    fn expired_deposit_reference_rejected() {
        let store = make_store();
        let wallet = WalletId::new();
        store.ensure_wallet(wallet).unwrap();
        let dep_ref = DepositReference {
            chain: ChainId::default(),
            reference: "expired_ref".into(),
            wallet_id: wallet,
            nonce: [0u8; 16],
            expires_at: chrono::Utc::now() - chrono::Duration::hours(1), // already expired
        };
        store.store_deposit_reference(&dep_ref).unwrap();
        let result = store.consume_deposit_reference("expired_ref").unwrap();
        assert_eq!(result, None);
    }
    #[test]
    fn api_key_store_and_retrieve_by_public_key() {
        let store = make_store();
        let wallet = WalletId::new();
        store.ensure_wallet(wallet).unwrap();
        let pubkey = Ed25519PublicKey([1u8; 32]);
        let cap = ApiKeyCapability {
            wallet_id: wallet,
            key_id: ApiKeyId::new(),
            public_key: pubkey,
            spending_limit_rj: RJoule::new(5000),
            spent_rj: RJoule::ZERO,
            scope: vec!["read-specs".to_string()],
            purpose: "test key".to_string(),
            rate_limit: None,
            expiry: None,
            issued_at: chrono::Utc::now(),
            privacy_mode: PrivacyMode::default(),
            preferred_chain: None,
        };
        store.store_api_key(&cap).unwrap();
        let retrieved = store.get_api_key_by_public_key(pubkey.as_bytes()).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().key_id, cap.key_id);
    }
    #[test]
    fn api_key_revocation_returns_unspent_rjoules() {
        let store = make_store();
        let wallet = WalletId::new();
        store.credit_rjoules(wallet, RJoule::new(10000)).unwrap();
        let cap = ApiKeyCapability {
            wallet_id: wallet,
            key_id: ApiKeyId::new(),
            public_key: Ed25519PublicKey([2u8; 32]),
            spending_limit_rj: RJoule::new(5000),
            spent_rj: RJoule::new(1200), // 3800 unspent
            scope: vec!["embed-corpus".to_string()],
            purpose: "revocation test".to_string(),
            rate_limit: None,
            expiry: None,
            issued_at: chrono::Utc::now(),
            privacy_mode: PrivacyMode::default(),
            preferred_chain: None,
        };
        let key_id = cap.key_id;
        store.store_api_key(&cap).unwrap();
        // Debit the wallet by the key's spending limit (simulating allocation)
        store.debit_rjoules(wallet, RJoule::new(5000)).unwrap();
        let before = store.get_balance(wallet).unwrap().unwrap();
        assert_eq!(before.rjoules, 5000); // 10000 - 5000
        store.revoke_api_key(key_id).unwrap();
        let after = store.get_balance(wallet).unwrap().unwrap();
        assert_eq!(after.rjoules, 8800); // 5000 + 3800 unspent returned
    }
    #[test]
    fn consume_encumbrance_updates_api_key_spent_rj() {
        let store = make_store();
        let wallet = WalletId::new();
        store.credit_rjoules(wallet, RJoule::new(10_000)).unwrap();
        let key_id = ApiKeyId::new();
        let cap = ApiKeyCapability {
            wallet_id: wallet,
            key_id,
            public_key: Ed25519PublicKey([7u8; 32]),
            spending_limit_rj: RJoule::new(5000),
            spent_rj: RJoule::ZERO,
            scope: vec!["read-specs".to_string()],
            purpose: "spend sync test".to_string(),
            rate_limit: None,
            expiry: None,
            issued_at: chrono::Utc::now(),
            privacy_mode: PrivacyMode::default(),
            preferred_chain: None,
        };
        store.store_api_key(&cap).unwrap();
        store
            .encumber_rjoules(wallet, key_id, RJoule::new(2000))
            .unwrap();
        store.consume_encumbrance(key_id, RJoule::new(300)).unwrap();
        store.consume_encumbrance(key_id, RJoule::new(250)).unwrap();
        let key = store.get_api_key(key_id).unwrap().unwrap();
        assert_eq!(
            key.spent_rj,
            RJoule::new(550),
            "spent_rj must track cumulative encumbrance consumption"
        );
    }
    #[test]
    fn failed_consume_does_not_increment_api_key_spent_rj() {
        let store = make_store();
        let wallet = WalletId::new();
        store.credit_rjoules(wallet, RJoule::new(10_000)).unwrap();
        let key_id = ApiKeyId::new();
        let cap = ApiKeyCapability {
            wallet_id: wallet,
            key_id,
            public_key: Ed25519PublicKey([8u8; 32]),
            spending_limit_rj: RJoule::new(5000),
            spent_rj: RJoule::ZERO,
            scope: vec!["read-specs".to_string()],
            purpose: "failed consume sync test".to_string(),
            rate_limit: None,
            expiry: None,
            issued_at: chrono::Utc::now(),
            privacy_mode: PrivacyMode::default(),
            preferred_chain: None,
        };
        store.store_api_key(&cap).unwrap();
        store
            .encumber_rjoules(wallet, key_id, RJoule::new(300))
            .unwrap();
        store.consume_encumbrance(key_id, RJoule::new(300)).unwrap();
        // Replay/second consume must fail because encumbrance is fully consumed.
        let second = store.consume_encumbrance(key_id, RJoule::new(1));
        assert!(
            second.is_err(),
            "second consume must fail after full consumption"
        );
        let key = store.get_api_key(key_id).unwrap().unwrap();
        assert_eq!(
            key.spent_rj,
            RJoule::new(300),
            "spent_rj must remain unchanged on failed consume"
        );
    }
    #[test]
    fn purge_expired_references_cleans_up() {
        let store = make_store();
        let wallet = WalletId::new();
        store.ensure_wallet(wallet).unwrap();
        // Store an expired reference
        let expired = DepositReference {
            chain: ChainId::default(),
            reference: "old_ref".into(),
            wallet_id: wallet,
            nonce: [0u8; 16],
            expires_at: chrono::Utc::now() - chrono::Duration::hours(1),
        };
        store.store_deposit_reference(&expired).unwrap();
        // Store a valid reference
        let valid = DepositReference {
            chain: ChainId::default(),
            reference: "new_ref".into(),
            wallet_id: wallet,
            nonce: [1u8; 16],
            expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
        };
        store.store_deposit_reference(&valid).unwrap();
        let purged = store.purge_expired_references().unwrap();
        assert_eq!(purged, 1);
        // Expired is gone
        assert_eq!(store.consume_deposit_reference("old_ref").unwrap(), None);
        // Valid still works
        assert_eq!(
            store.consume_deposit_reference("new_ref").unwrap(),
            Some(wallet)
        );
    }
    // Property test: for any sequence of credits and debits, the sum of all
    // transaction rjoules_delta values must equal the current wallet balance.
    #[test]
    fn balance_equals_sum_of_ledger_deltas() {
        let store = make_store();
        let wallet = WalletId::new();
        store.ensure_wallet(wallet).unwrap();
        // Perform a random-ish sequence of credits and debits.
        // Using fixed values for deterministic reproducibility.
        let operations: [(bool, u64); 12] = [
            (true, 5000),  // credit 5000
            (true, 3000),  // credit 3000
            (false, 1200), // debit 1200
            (true, 750),   // credit 750
            (false, 3000), // debit 3000
            (false, 500),  // debit 500
            (true, 10000), // credit 10000
            (false, 2000), // debit 2000
            (true, 150),   // credit 150
            (false, 8000), // debit 8000
            (false, 1500), // debit 1500
            (true, 2500),  // credit 2500
        ];
        let mut expected_sum: i64 = 0;
        for (is_credit, amount) in &operations {
            let rj = RJoule::new(*amount);
            if *is_credit {
                let balance = store.credit_rjoules(wallet, rj).unwrap();
                expected_sum += *amount as i64;
                // Record the transaction (as WalletManager does)
                store
                    .record_transaction(&WalletTransaction {
                        id: 0,
                        wallet_id: wallet,
                        tx_type: TransactionType::Deposit {
                            chain: ChainId::default(),
                            privacy: PrivacyMode::default(),
                            tx_hash: format!("test_tx_{}", expected_sum),
                            amount_usdc_micro: *amount * 1000,
                        },
                        rjoules_delta: *amount as i64,
                        balance_after: balance.rjoules,
                        timestamp: chrono::Utc::now(),
                    })
                    .unwrap();
            } else {
                // Only debit if we can afford it
                if store.get_balance(wallet).unwrap().unwrap().rjoules >= *amount {
                    let balance = store.debit_rjoules(wallet, rj).unwrap();
                    expected_sum -= *amount as i64;
                    store
                        .record_transaction(&WalletTransaction {
                            id: 0,
                            wallet_id: wallet,
                            tx_type: TransactionType::Withdrawal {
                                chain: ChainId::default(),
                                privacy: PrivacyMode::default(),
                                tx_hash: format!("test_tx_{}", expected_sum),
                                amount_usdc_micro: *amount * 1000,
                            },
                            rjoules_delta: -(*amount as i64),
                            balance_after: balance.rjoules,
                            timestamp: chrono::Utc::now(),
                        })
                        .unwrap();
                }
            }
        }
        // Verify: current balance == sum of all deltas
        let balance = store.get_balance(wallet).unwrap().unwrap();
        assert_eq!(
            balance.rjoules as i64, expected_sum,
            "MUST-10 VIOLATION: balance {} != sum of ledger deltas {}",
            balance.rjoules, expected_sum,
        );
        // Cross-verify via transaction ledger
        let txs = store.get_transactions(wallet, 100, 0).unwrap();
        let ledger_sum: i64 = txs.iter().map(|tx| tx.rjoules_delta).sum();
        assert_eq!(
            balance.rjoules as i64, ledger_sum,
            "MUST-10 VIOLATION: balance {} != ledger sum {}",
            balance.rjoules, ledger_sum,
        );
    }
    // ── Idempotency contract tests ──────────────────────────────────────
    //
    // Idempotency contract matrix (PR 2.5.1):
    //
    // | Operation                  | Idempotent? | Mechanism                          |
    // |----------------------------|:-----------:|------------------------------------|
    // | ensure_wallet               | ✅          | INSERT OR IGNORE                  |
    // | get_balance / can_afford    | ✅          | Read-only                         |
    // | get_transactions            | ✅          | Read-only                         |
    // | consume_deposit_reference   | ✅          | Atomic CAS (spent=0 → spent=1)    |
    // | release_encumbrance         | ✅          | Status guard (active only)        |
    // | revoke_api_key              | ✅          | Marks revoked (idempotent mark)   |
    // | credit_rjoules              | ❌          | No tx-hash dedup (GAP)            |
    // | debit_rjoules               | ❌          | No idempotency key (GAP)          |
    // | encumber_rjoules            | ⚡           | Key-scoped guard (not op-scoped)  |
    // | consume_encumbrance         | ❌          | Double-consumes while active (GAP)|
    // | store_api_key               | ❌          | Always creates new key (GAP)      |
    // | store_deposit_reference     | ❌          | Always inserts                    |
    //
    // GAP entries are documented below with regression-catching tests.
    #[test]
    fn ensure_wallet_is_idempotent() {
        let store = make_store();
        let wallet = WalletId::new();
        // First call creates
        store.ensure_wallet(wallet).unwrap();
        let b1 = store.get_balance(wallet).unwrap().unwrap();
        assert_eq!(b1.rjoules, 0);
        // Second call should be no-op (INSERT OR IGNORE)
        store.ensure_wallet(wallet).unwrap();
        let b2 = store.get_balance(wallet).unwrap().unwrap();
        assert_eq!(
            b2.rjoules, 0,
            "balance should not change on duplicate ensure"
        );
    }
    #[test]
    fn release_encumbrance_is_idempotent() {
        let store = make_store();
        let wallet = WalletId::new();
        store.credit_rjoules(wallet, RJoule::new(5000)).unwrap();
        // Create an API key first (encumbrance references api_keys table)
        let key_id = ApiKeyId::new();
        let cap = ApiKeyCapability {
            wallet_id: wallet,
            key_id,
            public_key: Ed25519PublicKey([9u8; 32]),
            spending_limit_rj: RJoule::new(5000),
            spent_rj: RJoule::ZERO,
            scope: vec!["test".to_string()],
            purpose: "idempotency test".to_string(),
            rate_limit: None,
            expiry: None,
            issued_at: chrono::Utc::now(),
            privacy_mode: PrivacyMode::default(),
            preferred_chain: None,
        };
        store.store_api_key(&cap).unwrap();
        store
            .encumber_rjoules(wallet, key_id, RJoule::new(1000))
            .unwrap();
        // Balance should be 4000 after encumbrance
        let after_encumber = store.get_balance(wallet).unwrap().unwrap();
        assert_eq!(after_encumber.rjoules, 4000);
        // First release returns funds
        store.release_encumbrance(key_id).unwrap();
        let after_first = store.get_balance(wallet).unwrap().unwrap();
        assert_eq!(
            after_first.rjoules, 5000,
            "first release should return funds"
        );
        // Second release is a no-op (explicitly documented as idempotent)
        store.release_encumbrance(key_id).unwrap();
        let after_second = store.get_balance(wallet).unwrap().unwrap();
        assert_eq!(
            after_second.rjoules, 5000,
            "second release must not double-credit (idempotency contract)"
        );
    }
    //
    // This test documents the CURRENT behavior. When a transaction-hash
    // deduplication mechanism is added, this test MUST be updated to verify
    // that duplicate credits are rejected.
    #[test]
    fn credit_rjoules_is_not_idempotent_documents_gap() {
        let store = make_store();
        let wallet = WalletId::new();
        // Credit once
        store.credit_rjoules(wallet, RJoule::new(1000)).unwrap();
        assert_eq!(store.get_balance(wallet).unwrap().unwrap().rjoules, 1000);
        // Credit again with same amount — currently doubles (GAP)
        store.credit_rjoules(wallet, RJoule::new(1000)).unwrap();
        assert_eq!(
            store.get_balance(wallet).unwrap().unwrap().rjoules,
            2000,
            "GAP: duplicate credit doubles balance — no tx-hash dedup exists"
        );
    }
    //
    // This test documents the CURRENT behavior. When an idempotency key
    // mechanism is added, this test MUST be updated to verify that duplicate
    // debits are rejected (or are safe).
    #[test]
    fn debit_rjoules_is_not_idempotent_documents_gap() {
        let store = make_store();
        let wallet = WalletId::new();
        store.credit_rjoules(wallet, RJoule::new(1000)).unwrap();
        // Debit once
        store.debit_rjoules(wallet, RJoule::new(300)).unwrap();
        assert_eq!(store.get_balance(wallet).unwrap().unwrap().rjoules, 700);
        // Debit again — currently succeeds and double-charges (GAP)
        store.debit_rjoules(wallet, RJoule::new(300)).unwrap();
        assert_eq!(
            store.get_balance(wallet).unwrap().unwrap().rjoules,
            400,
            "GAP: duplicate debit double-charges — no idempotency key exists"
        );
    }
    //
    // This is the same as the anti-replay test above but explicitly framed
    // as an idempotency contract test.
    #[test]
    fn consume_deposit_reference_is_idempotent() {
        let store = make_store();
        let wallet = WalletId::new();
        store.ensure_wallet(wallet).unwrap();
        let dep_ref = DepositReference {
            chain: ChainId::default(),
            reference: "idem_ref_001".into(),
            wallet_id: wallet,
            nonce: [0u8; 16],
            expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
        };
        store.store_deposit_reference(&dep_ref).unwrap();
        // First consumption succeeds
        let r1 = store.consume_deposit_reference("idem_ref_001").unwrap();
        assert_eq!(r1, Some(wallet));
        // Second consumption returns None (idempotent — already spent)
        let r2 = store.consume_deposit_reference("idem_ref_001").unwrap();
        assert_eq!(
            r2, None,
            "second consume must return None (idempotent via atomic CAS)"
        );
    }
}
