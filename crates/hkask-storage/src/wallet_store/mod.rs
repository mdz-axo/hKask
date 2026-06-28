//! WalletStore — SQLite-backed persistence for rJoule balances, transactions, API keys.
//!
//! # Schema (5 tables)
//! - `wallet_balances` — one row per wallet, current rJoule balance
//! - `wallet_transactions` — append-only ledger of all balance changes
//! - `api_keys` — issued Ed25519 capability tokens with spending limits
//! - `deposit_addresses` — derived deposit addresses per wallet per chain
//! - `deposit_references` — one-time shielded deposit references (anti-replay)

pub(crate) mod types;
pub(crate) use types::*;

pub mod api_key;
pub mod deposit;
pub mod encumbrance;
pub mod transaction;

use crate::define_store;
define_store!(WalletStore);

impl WalletStore {
    /// Enable SQLite WAL (Write-Ahead Logging) mode for better concurrency.
    pub fn enable_wal_mode(&self) -> Result<(), hkask_wallet_types::WalletError> {
        use crate::Store;
        let conn = self.lock_conn().map_err(|e| {
            hkask_wallet_types::WalletError::Storage(hkask_types::InfrastructureError {
                message: format!("Failed to lock connection: {e}"),
            })
        })?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| {
                hkask_wallet_types::WalletError::Storage(hkask_types::InfrastructureError {
                    message: format!("Failed to enable WAL: {e}"),
                })
            })?;
        Ok(())
    }
}
