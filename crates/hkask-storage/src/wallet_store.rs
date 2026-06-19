//! WalletStore — SQLite-backed persistence for rJoule balances, transactions, API keys.
//!
//! # Schema (5 tables)
//! - `wallet_balances` — one row per wallet, current rJoule balance
//! - `wallet_transactions` — append-only ledger of all balance changes
//! - `api_keys` — issued Ed25519 capability tokens with spending limits
//! - `deposit_addresses` — derived deposit addresses per wallet per chain
//! - `deposit_references` — one-time shielded deposit references (anti-replay)
use crate::Store;
use hkask_rsolidity as rs;
use hkask_types::time::now_rfc3339;
use hkask_types::{
    ApiKeyCapability, ApiKeyId, ChainId, DepositAddress, DepositReference, Ed25519PublicKey,
    Encumbrance, EncumbranceStatus, InfrastructureError, PrivacyMode, RJoule, RateLimitConfig,
    TransactionType, WalletBalance, WalletError, WalletId, WalletTransaction,
};
use rusqlite::OptionalExtension;
use std::str::FromStr;
define_store!(WalletStore);
// ── Row types for query mapping ────────────────────────────────────────────────
struct WalletBalanceRow {
    wallet_id: String,
    balance_rj: i64,
    usdc_equivalent_micro: i64,
}
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
struct ApiKeyRow {
    key_id: String,
    wallet_id: String,
    public_key: Vec<u8>,
    spending_limit_rj: i64,
    spent_rj: i64,
    scope: String,
    purpose: String,
    rate_limit_json: Option<String>,
    privacy_mode: String,
    preferred_chain: Option<String>,
    expires_at: Option<String>,
    issued_at: String,
}
struct DepositAddressRow {
    chain: String,
    address: String,
    privacy_mode: String,
}
// ── WalletStore implementation ──────────────────────────────────────────────────
impl WalletStore {
    /// Enable SQLite WAL (Write-Ahead Logging) mode for better concurrency.
    ///
    /// WAL mode allows concurrent reads while a write is in progress,
    /// significantly improving throughput under multi-agent API key spend loads.
    /// Without WAL, all operations serialize on the connection mutex.
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// post: journal_mode set to WAL
    /// post: synchronous set to NORMAL (balance durability vs performance)
    ///
    /// Call once after store creation, before any wallet operations.
    /// Enable WAL mode for better concurrency.
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — enable WAL for wallet concurrency
    /// \[P7\] Constraining: Evolutionary Architecture — WAL mode emerged from multi-agent load
    /// post: journal_mode set to WAL, synchronous set to NORMAL
    #[rs::contract(id = "P3-sto-wallet-wal-mode", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-wal-mode", principle = "P3")]
    pub fn enable_wal_mode(&self) -> Result<(), WalletError> {
        let conn = self.lock_conn()?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; \
             PRAGMA synchronous=NORMAL; \
             PRAGMA busy_timeout=5000;",
        )
        .map_err(|e| WalletError::Infra(InfrastructureError::Database(e.to_string())))?;
        tracing::info!(target: "hkask.storage", "WalletStore WAL mode enabled");
        Ok(())
    }
    // ── Balance ──────────────────────────────────────────────────────────────
    /// Get the current balance for a wallet, or None if the wallet doesn't exist.
    /// Get wallet balance.
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — get wallet balance
    /// pre:  wallet_id is valid
    /// post: returns Some(WalletBalance) if wallet exists, None otherwise
    #[rs::contract(id = "P3-sto-wallet-balance-get", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-balance-get", principle = "P3")]
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
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — idempotently ensure wallet row
    /// pre:  wallet_id is valid
    /// post: wallet row exists (created if missing)
    #[rs::contract(id = "P3-sto-wallet-ensure", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-ensure", principle = "P3")]
    pub fn ensure_wallet(&self, wallet_id: WalletId) -> Result<(), WalletError> {
        let conn = self.lock_conn()?;
        self.ensure_wallet_with_conn(&conn, wallet_id)
    }
    /// List all wallet IDs in the system.
    /// List all wallet IDs.
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P8\] Motivating: Semantic Grounding — list wallet IDs
    /// post: returns Vec of all WalletId
    #[rs::contract(id = "P3-sto-wallet-list-ids", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-list-ids", principle = "P3")]
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
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — credit rJoules
    /// pre:  wallet_id exists, amount > 0
    /// post: balance increased by amount, transaction recorded
    #[rs::contract(id = "P3-sto-wallet-credit", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-credit", principle = "P3")]
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
    /// Debit rJoules from a wallet.
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — debit rJoules
    /// pre:  wallet_id exists, amount > 0, balance >= amount
    /// post: balance decreased by amount, transaction recorded
    /// post: returns Err if insufficient balance
    #[rs::contract(id = "P3-sto-wallet-debit", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-debit", principle = "P3")]
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
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — record wallet transaction
    /// pre:  tx has valid wallet_id and rjoules_delta
    /// post: transaction inserted into ledger
    #[rs::contract(id = "P3-sto-wallet-tx-record", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-tx-record", principle = "P3")]
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
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — list transactions
    /// pre:  wallet_id is valid
    /// post: returns Vec of transactions, optionally limited
    #[rs::contract(id = "P3-sto-wallet-tx-list", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-tx-list", principle = "P3")]
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
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P4\] Motivating: Clear Boundaries — anti-replay hash check
    /// pre:  tx_hash is non-empty
    /// post: returns true if hash exists (anti-replay)
    #[rs::contract(id = "P3-sto-wallet-tx-hash-exists", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-tx-hash-exists", principle = "P3")]
    pub fn transaction_exists_by_hash(&self, tx_hash: &str) -> Result<bool, WalletError> {
        let conn = self.lock_conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM wallet_transactions WHERE on_chain_tx_hash = ?1",
            rusqlite::params![tx_hash],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
    // ── API Keys ─────────────────────────────────────────────────────────────
    /// Store a newly issued API key capability.
    /// Store an API key capability.
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — store API key capability
    /// pre:  capability has valid key_id and wallet_id
    /// post: API key stored
    #[rs::contract(id = "P3-sto-wallet-api-key-store", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-api-key-store", principle = "P3")]
    pub fn store_api_key(&self, capability: &ApiKeyCapability) -> Result<(), WalletError> {
        let conn = self.lock_conn()?;
        let scope_json =
            serde_json::to_string(&capability.scope).unwrap_or_else(|_| "[]".to_string());
        let rate_limit_json = capability
            .rate_limit
            .as_ref()
            .and_then(|rl| serde_json::to_string(rl).ok());
        conn.execute(
            "INSERT INTO api_keys (key_id, wallet_id, public_key, spending_limit_rj, spent_rj, scope, purpose, rate_limit_json, privacy_mode, preferred_chain, expires_at, issued_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                capability.key_id.to_string(),
                capability.wallet_id.to_string(),
                capability.public_key.as_bytes(),
                capability.spending_limit_rj.as_u64() as i64,
                capability.spent_rj.as_u64() as i64,
                scope_json,
                capability.purpose,
                rate_limit_json,
                capability.privacy_mode.to_string(),
                capability.preferred_chain.map(|c| c.to_string()),
                capability.expiry.map(|e| e.to_rfc3339()),
                capability.issued_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }
    /// Look up an API key by its ID.
    /// Get an API key by key ID.
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — get API key by ID
    /// pre:  key_id is valid
    /// post: returns Some(capability) if found, None otherwise
    #[rs::contract(id = "P3-sto-wallet-api-key-get", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-api-key-get", principle = "P3")]
    pub fn get_api_key(&self, key_id: ApiKeyId) -> Result<Option<ApiKeyCapability>, WalletError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT key_id, wallet_id, public_key, spending_limit_rj, spent_rj, scope, purpose, rate_limit_json, privacy_mode, preferred_chain, expires_at, issued_at, revoked_at FROM api_keys WHERE key_id = ?1",
        )?;
        let rows: Vec<ApiKeyCapability> = collect_rows_strict!(
            stmt,
            rusqlite::params![key_id.to_string()],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<ApiKeyRow> {
                Ok(ApiKeyRow {
                    key_id: row.get(0)?,
                    wallet_id: row.get(1)?,
                    public_key: row.get(2)?,
                    spending_limit_rj: row.get(3)?,
                    spent_rj: row.get(4)?,
                    scope: row.get(5)?,
                    purpose: row.get(6)?,
                    rate_limit_json: row.get(7)?,
                    privacy_mode: row.get(8)?,
                    preferred_chain: row.get(9)?,
                    expires_at: row.get(10)?,
                    issued_at: row.get(11)?,
                })
            },
            |r: ApiKeyRow| -> Result<ApiKeyCapability, WalletError> {
                row_to_api_key_capability(r)
            }
        );
        Ok(rows.into_iter().next())
    }
    /// Look up an API key by its Ed25519 public key (for Bearer token auth).
    /// Get an API key by public key.
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — get API key by public key
    /// pre:  public_key is valid
    /// post: returns Some(capability) if found, None otherwise
    #[rs::contract(id = "P3-sto-wallet-api-key-by-pubkey", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-api-key-by-pubkey", principle = "P3")]
    pub fn get_api_key_by_public_key(
        &self,
        public_key: &[u8],
    ) -> Result<Option<ApiKeyCapability>, WalletError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT key_id, wallet_id, public_key, spending_limit_rj, spent_rj, scope, purpose, rate_limit_json, privacy_mode, preferred_chain, expires_at, issued_at, revoked_at FROM api_keys WHERE public_key = ?1 AND revoked_at IS NULL",
        )?;
        let rows: Vec<ApiKeyCapability> = collect_rows_strict!(
            stmt,
            rusqlite::params![public_key],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<ApiKeyRow> {
                Ok(ApiKeyRow {
                    key_id: row.get(0)?,
                    wallet_id: row.get(1)?,
                    public_key: row.get(2)?,
                    spending_limit_rj: row.get(3)?,
                    spent_rj: row.get(4)?,
                    scope: row.get(5)?,
                    purpose: row.get(6)?,
                    rate_limit_json: row.get(7)?,
                    privacy_mode: row.get(8)?,
                    preferred_chain: row.get(9)?,
                    expires_at: row.get(10)?,
                    issued_at: row.get(11)?,
                })
            },
            |r: ApiKeyRow| -> Result<ApiKeyCapability, WalletError> {
                row_to_api_key_capability(r)
            }
        );
        Ok(rows.into_iter().next())
    }
    /// List all active (non-revoked) API keys for a wallet.
    /// List API keys for a wallet.
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — list API keys
    /// pre:  wallet_id is valid
    /// post: returns Vec of API key capabilities
    #[rs::contract(id = "P3-sto-wallet-api-key-list", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-api-key-list", principle = "P3")]
    pub fn list_api_keys(&self, wallet_id: WalletId) -> Result<Vec<ApiKeyCapability>, WalletError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT key_id, wallet_id, public_key, spending_limit_rj, spent_rj, scope, purpose, rate_limit_json, privacy_mode, preferred_chain, expires_at, issued_at, revoked_at FROM api_keys WHERE wallet_id = ?1 AND revoked_at IS NULL ORDER BY issued_at DESC",
        )?;
        let rows: Vec<ApiKeyCapability> = collect_rows_strict!(
            stmt,
            rusqlite::params![wallet_id.to_string()],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<ApiKeyRow> {
                Ok(ApiKeyRow {
                    key_id: row.get(0)?,
                    wallet_id: row.get(1)?,
                    public_key: row.get(2)?,
                    spending_limit_rj: row.get(3)?,
                    spent_rj: row.get(4)?,
                    scope: row.get(5)?,
                    purpose: row.get(6)?,
                    rate_limit_json: row.get(7)?,
                    privacy_mode: row.get(8)?,
                    preferred_chain: row.get(9)?,
                    expires_at: row.get(10)?,
                    issued_at: row.get(11)?,
                })
            },
            |r: ApiKeyRow| -> Result<ApiKeyCapability, WalletError> {
                row_to_api_key_capability(r)
            }
        );
        Ok(rows)
    }
    /// Revoke an API key. Returns unspent rJoules to the wallet.
    /// Idempotent — revoking an already-revoked key is a no-op.
    /// Revoke an API key.
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — revoke API key
    /// pre:  key_id is valid
    /// post: API key revoked, unspent rJ returned to wallet
    #[rs::contract(id = "P3-sto-wallet-api-key-revoke", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-api-key-revoke", principle = "P3")]
    pub fn revoke_api_key(&self, key_id: ApiKeyId) -> Result<(), WalletError> {
        let conn = self.lock_conn()?;
        let now = now_rfc3339();
        let rows = conn.execute(
            "UPDATE api_keys SET revoked_at = ?1 WHERE key_id = ?2 AND revoked_at IS NULL",
            rusqlite::params![now, key_id.to_string()],
        )?;
        if rows == 0 {
            return Ok(()); // already revoked or doesn't exist — no-op
        }
        // Return unspent rJoules to wallet
        let (wallet_id_str, spent, limit): (String, i64, i64) = conn.query_row(
            "SELECT wallet_id, spent_rj, spending_limit_rj FROM api_keys WHERE key_id = ?1",
            rusqlite::params![key_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let unspent = limit - spent;
        if unspent > 0 {
            conn.execute(
                "UPDATE wallet_balances SET balance_rj = balance_rj + ?1, updated_at = ?2 WHERE wallet_id = ?3",
                rusqlite::params![unspent, now, wallet_id_str],
            )?;
        }
        Ok(())
    }
    /// Update the spent_rj counter on an API key (called after each tool invocation).
    /// Update spent rJoules for an API key.
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — update spent rJ for key
    /// pre:  key_id is valid
    /// post: spent_rj updated
    #[rs::contract(id = "P3-sto-wallet-spent-rj-update", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-spent-rj-update", principle = "P3")]
    pub fn update_spent_rj(&self, key_id: ApiKeyId, spent: RJoule) -> Result<(), WalletError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "UPDATE api_keys SET spent_rj = ?1 WHERE key_id = ?2",
            rusqlite::params![spent.as_u64() as i64, key_id.to_string()],
        )?;
        Ok(())
    }
    // ── Deposit Addresses ────────────────────────────────────────────────────
    /// Store a derived deposit address for a wallet.
    /// Store a deposit address.
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — store deposit address
    /// pre:  address has valid wallet_id and chain
    /// post: deposit address stored
    #[rs::contract(id = "P3-sto-wallet-address-store", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-address-store", principle = "P3")]
    pub fn store_deposit_address(
        &self,
        wallet_id: WalletId,
        chain: ChainId,
        address: &str,
        index: u64,
        privacy: PrivacyMode,
    ) -> Result<(), WalletError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO deposit_addresses (wallet_id, chain, address, derivation_index, privacy_mode) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                wallet_id.to_string(),
                chain.to_string(),
                address,
                index as i64,
                privacy.to_string(),
            ],
        )?;
        Ok(())
    }
    /// Get all deposit addresses for a wallet.
    /// Get deposit addresses for a wallet.
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — list deposit addresses
    /// pre:  wallet_id is valid
    /// post: returns Vec of deposit addresses
    #[rs::contract(id = "P3-sto-wallet-address-list", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-address-list", principle = "P3")]
    pub fn get_deposit_addresses(
        &self,
        wallet_id: WalletId,
    ) -> Result<Vec<DepositAddress>, WalletError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT wallet_id, chain, address, derivation_index, privacy_mode FROM deposit_addresses WHERE wallet_id = ?1 ORDER BY derivation_index",
        )?;
        let rows: Vec<DepositAddress> = collect_rows_strict!(
            stmt,
            rusqlite::params![wallet_id.to_string()],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<DepositAddressRow> {
                Ok(DepositAddressRow {
                    chain: row.get(1)?,
                    address: row.get(2)?,
                    privacy_mode: row.get(4)?,
                })
            },
            |r: DepositAddressRow| -> Result<DepositAddress, WalletError> {
                Ok(DepositAddress {
                    address: r.address,
                    chain: ChainId::from_str(&r.chain)
                        .map_err(|e| WalletError::Infra(InfrastructureError::Database(e)))?,
                    privacy_mode: PrivacyMode::from_str(&r.privacy_mode)
                        .map_err(|e| WalletError::Infra(InfrastructureError::Database(e)))?,
                })
            }
        );
        Ok(rows)
    }
    /// Resolve which wallet owns a deposit address (reverse lookup).
    ///
    /// Used by the deposit monitor to credit incoming transfers to the
    /// correct wallet in a multi-wallet setup.
    /// Resolve wallet for a deposit address.
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — resolve wallet for address
    /// pre:  chain is valid, address is non-empty
    /// post: returns Some(WalletId) if found, None otherwise
    #[rs::contract(id = "P3-sto-wallet-address-resolve", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-address-resolve", principle = "P3")]
    pub fn resolve_wallet_for_address(
        &self,
        address: &str,
    ) -> Result<Option<WalletId>, WalletError> {
        let conn = self.lock_conn()?;
        let mut stmt =
            conn.prepare("SELECT wallet_id FROM deposit_addresses WHERE address = ?1")?;
        let wallet_id_str: Option<String> = stmt
            .query_row(rusqlite::params![address], |row| row.get(0))
            .optional()
            .map_err(|e| WalletError::Infra(InfrastructureError::Database(e.to_string())))?;
        match wallet_id_str {
            Some(s) => Ok(Some(WalletId::from_str(&s)?)),
            None => Ok(None),
        }
    }
    // ── Deposit References ───────────────────────────────────────────────────
    /// Store a one-time shielded deposit reference.
    /// Store a deposit reference for anti-replay.
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — store deposit reference
    /// pre:  reference has valid fields
    /// post: deposit reference stored
    #[rs::contract(id = "P3-sto-wallet-reference-store", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-reference-store", principle = "P3")]
    pub fn store_deposit_reference(&self, reference: &DepositReference) -> Result<(), WalletError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO deposit_references (reference, wallet_id, chain, expires_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                reference.reference,
                reference.wallet_id.to_string(),
                reference.chain.to_string(),
                reference.expires_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }
    /// Consume a deposit reference — atomically marks it spent and returns the wallet_id.
    /// Returns None if the reference doesn't exist, is already spent, or has expired.
    /// Consume a deposit reference (anti-replay).
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — consume deposit reference
    /// pre:  reference is valid and not expired
    /// post: reference consumed, wallet credited
    /// post: returns Err if already consumed or expired
    #[rs::contract(id = "P3-sto-wallet-reference-consume", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-reference-consume", principle = "P3")]
    pub fn consume_deposit_reference(
        &self,
        reference: &str,
    ) -> Result<Option<WalletId>, WalletError> {
        let conn = self.lock_conn()?;
        let now = now_rfc3339();
        // Atomic check-and-set: only consume if not already spent and not expired
        let rows = conn.execute(
            "UPDATE deposit_references SET spent = 1 WHERE reference = ?1 AND spent = 0 AND expires_at > ?2",
            rusqlite::params![reference, now],
        )?;
        if rows == 0 {
            return Ok(None); // not found, already spent, or expired
        }
        let wallet_id_str: String = conn.query_row(
            "SELECT wallet_id FROM deposit_references WHERE reference = ?1",
            rusqlite::params![reference],
            |row| row.get(0),
        )?;
        Ok(Some(WalletId::from_str(&wallet_id_str)?))
    }
    /// Purge expired deposit references. Returns count of purged rows.
    /// Purge expired deposit references.
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — purge expired references
    /// post: expired references deleted
    /// post: returns count of deleted references
    #[rs::contract(id = "P3-sto-wallet-reference-purge", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-reference-purge", principle = "P3")]
    pub fn purge_expired_references(&self) -> Result<u64, WalletError> {
        let conn = self.lock_conn()?;
        let now = now_rfc3339();
        let rows = conn.execute(
            "DELETE FROM deposit_references WHERE expires_at <= ?1",
            rusqlite::params![now],
        )?;
        Ok(rows as u64)
    }
    // ── Encumbrance methods ──────────────────────────────────────────────────
    /// Lock rJoules from a wallet for an API key's use.
    ///
    /// Debits the wallet balance by `amount_rj` and creates an active
    /// encumbrance row. Returns an error if the key already has an active
    /// encumbrance or the wallet has insufficient balance.
    /// Encumber rJoules for an API key (lock funds for spending).
    ///
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — encumber rJoules for key
    /// pre:  wallet_id exists, key_id is valid, amount > 0, balance >= amount
    /// post: rJoules encumbered, balance decreased
    #[rs::contract(id = "P3-sto-wallet-encumber", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-encumber", principle = "P3")]
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
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — release encumbrance
    /// pre:  key_id has active encumbrance
    /// post: encumbrance released, unspent rJ returned to wallet
    #[rs::contract(id = "P3-sto-wallet-encumbrance-release", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-encumbrance-release", principle = "P3")]
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
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — consume from encumbrance
    /// pre:  key_id has active encumbrance with sufficient remaining
    /// post: consumed_rj increased, api_keys.spent_rj synced
    /// post: returns Err if insufficient or not active
    #[rs::contract(id = "P3-sto-wallet-encumbrance-consume", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-encumbrance-consume", principle = "P3")]
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
    /// expect: "The system provides durable storage for wallet data" [P3]
    /// \[P3\] Motivating: Generative Space — get encumbrance
    /// pre:  key_id is valid
    /// post: returns Some(Encumbrance) if found, None otherwise
    #[rs::contract(id = "P3-sto-wallet-encumbrance-get", principle = "P3")]
    #[rs::contract(id = "P3-sto-wallet-encumbrance-get", principle = "P3")]
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
        TransactionType::Deposit {
            chain,
            privacy,
            tx_hash,
            ..
        } => (
            "deposit",
            Some(privacy.to_string()),
            Some(chain.to_string()),
            Some(tx_hash.clone()),
            None,
            None,
            None,
        ),
        TransactionType::Withdrawal {
            chain,
            privacy,
            tx_hash,
            ..
        } => (
            "withdrawal",
            Some(privacy.to_string()),
            Some(chain.to_string()),
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
            chain: ChainId::from_str(r.chain.as_deref().unwrap_or("solana"))
                .map_err(|e| WalletError::Infra(InfrastructureError::Database(e)))?,
            privacy: PrivacyMode::from_str(r.tx_subtype.as_deref().unwrap_or("transparent"))
                .map_err(|e| WalletError::Infra(InfrastructureError::Database(e)))?,
            tx_hash: r.on_chain_tx_hash.unwrap_or_default(),
            amount_usdc_micro: 0, // reconstructed from amount_rj / config
        },
        "withdrawal" => TransactionType::Withdrawal {
            chain: ChainId::from_str(r.chain.as_deref().unwrap_or("solana"))
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
            chain: ChainId::from_str(r.chain.as_deref().unwrap_or("solana"))
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
        privacy_mode: PrivacyMode::from_str(&r.privacy_mode)
            .map_err(|e| WalletError::Infra(InfrastructureError::Database(e)))?,
        preferred_chain: r
            .preferred_chain
            .map(|c| ChainId::from_str(&c))
            .transpose()
            .map_err(|e| WalletError::Infra(InfrastructureError::Database(e)))?,
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
    // contract: P3-sto-wallet-wal-test
    // expect: "Storage operation works correctly under test conditions" [P3]
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
    // contract: P1-sto-wallet-store-test
    // expect: "Storage operation works correctly under test conditions" [P1]
    #[test]
    fn credit_rjoules_increases_balance() {
        let store = make_store();
        let wallet = WalletId::new();
        let balance = store.credit_rjoules(wallet, RJoule::new(1000)).unwrap();
        assert_eq!(balance.rjoules, 1000);
    }
    // contract: P1-sto-wallet-store-test
    // expect: "Storage operation works correctly under test conditions" [P1]
    #[test]
    fn debit_rjoules_decreases_balance() {
        let store = make_store();
        let wallet = WalletId::new();
        store.credit_rjoules(wallet, RJoule::new(1000)).unwrap();
        let balance = store.debit_rjoules(wallet, RJoule::new(300)).unwrap();
        assert_eq!(balance.rjoules, 700);
    }
    // contract: P1-sto-wallet-store-test
    // expect: "Storage operation works correctly under test conditions" [P1]
    #[test]
    fn debit_rjoules_rejects_insufficient_balance() {
        let store = make_store();
        let wallet = WalletId::new();
        store.credit_rjoules(wallet, RJoule::new(100)).unwrap();
        let err = store.debit_rjoules(wallet, RJoule::new(500)).unwrap_err();
        assert!(matches!(err, WalletError::InsufficientBalance { .. }));
    }
    // contract: P1-sto-wallet-store-test
    // expect: "Storage operation works correctly under test conditions" [P1]
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
    // contract: P1-sto-wallet-store-test
    // expect: "Storage operation works correctly under test conditions" [P1]
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
                chain: ChainId::Solana,
                privacy: PrivacyMode::Transparent,
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
    // contract: P1-sto-wallet-store-test
    // expect: "Storage operation works correctly under test conditions" [P1]
    #[test]
    fn deposit_reference_anti_replay() {
        let store = make_store();
        let wallet = WalletId::new();
        store.ensure_wallet(wallet).unwrap();
        let dep_ref = DepositReference {
            reference: "test_ref_001".into(),
            wallet_id: wallet,
            chain: ChainId::Solana,
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
    // contract: P1-sto-wallet-store-test
    // expect: "Storage operation works correctly under test conditions" [P1]
    #[test]
    fn expired_deposit_reference_rejected() {
        let store = make_store();
        let wallet = WalletId::new();
        store.ensure_wallet(wallet).unwrap();
        let dep_ref = DepositReference {
            reference: "expired_ref".into(),
            wallet_id: wallet,
            chain: ChainId::Solana,
            nonce: [0u8; 16],
            expires_at: chrono::Utc::now() - chrono::Duration::hours(1), // already expired
        };
        store.store_deposit_reference(&dep_ref).unwrap();
        let result = store.consume_deposit_reference("expired_ref").unwrap();
        assert_eq!(result, None);
    }
    // contract: P1-sto-wallet-store-test
    // expect: "Storage operation works correctly under test conditions" [P1]
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
            privacy_mode: PrivacyMode::Transparent,
            preferred_chain: None,
        };
        store.store_api_key(&cap).unwrap();
        let retrieved = store.get_api_key_by_public_key(pubkey.as_bytes()).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().key_id, cap.key_id);
    }
    // contract: P1-sto-wallet-store-test
    // expect: "Storage operation works correctly under test conditions" [P1]
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
            privacy_mode: PrivacyMode::Transparent,
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
    // contract: P1-sto-wallet-spend-sync-test
    // expect: "Storage operation works correctly under test conditions" [P1]
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
            privacy_mode: PrivacyMode::Transparent,
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
    // contract: P1-sto-wallet-spend-sync-drift-test
    // expect: "Storage operation works correctly under test conditions" [P1]
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
            privacy_mode: PrivacyMode::Transparent,
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
    // contract: P1-sto-wallet-store-test
    // expect: "Storage operation works correctly under test conditions" [P1]
    #[test]
    fn purge_expired_references_cleans_up() {
        let store = make_store();
        let wallet = WalletId::new();
        store.ensure_wallet(wallet).unwrap();
        // Store an expired reference
        let expired = DepositReference {
            reference: "old_ref".into(),
            wallet_id: wallet,
            chain: ChainId::Solana,
            nonce: [0u8; 16],
            expires_at: chrono::Utc::now() - chrono::Duration::hours(1),
        };
        store.store_deposit_reference(&expired).unwrap();
        // Store a valid reference
        let valid = DepositReference {
            reference: "new_ref".into(),
            wallet_id: wallet,
            chain: ChainId::Solana,
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
    // contract: P1-sto-wallet-balance-conservation-test
    // expect: "Storage operation works correctly under test conditions" [P1]
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
                            chain: ChainId::Solana,
                            privacy: PrivacyMode::Transparent,
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
                                chain: ChainId::Solana,
                                privacy: PrivacyMode::Transparent,
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
    // contract: P3-sto-wallet-ensure-idempotent-test
    // expect: "Storage operation works correctly under test conditions" [P3]
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
    // contract: P3-sto-wallet-release-idempotent-test
    // expect: "Storage operation works correctly under test conditions" [P3]
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
            privacy_mode: PrivacyMode::Transparent,
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
    // contract: P3-sto-wallet-credit-not-idempotent-test
    // expect: "Storage operation works correctly under test conditions" [P3]
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
    // contract: P3-sto-wallet-debit-not-idempotent-test
    // expect: "Storage operation works correctly under test conditions" [P3]
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
    // contract: P3-sto-wallet-consume-reference-idempotent-test
    // expect: "Storage operation works correctly under test conditions" [P3]
    //
    // This is the same as the anti-replay test above but explicitly framed
    // as an idempotency contract test.
    #[test]
    fn consume_deposit_reference_is_idempotent() {
        let store = make_store();
        let wallet = WalletId::new();
        store.ensure_wallet(wallet).unwrap();
        let dep_ref = DepositReference {
            reference: "idem_ref_001".into(),
            wallet_id: wallet,
            chain: ChainId::Solana,
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
