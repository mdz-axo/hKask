//! WalletStore — SQLite-backed persistence for rJoule balances, transactions, API keys.
//!
//! # Schema (5 tables)
//! - `wallet_balances` — one row per wallet, current rJoule balance
//! - `wallet_transactions` — append-only ledger of all balance changes
//! - `api_keys` — issued Ed25519 capability tokens with spending limits
//! - `deposit_addresses` — derived deposit addresses per wallet per chain
//! - `deposit_references` — one-time shielded deposit references (anti-replay)

pub mod api_keys;
pub mod balances;
pub mod deposits;
pub mod encumbrances;
pub mod transactions;
#[cfg(test)]
mod tests;

use crate::Store;
use hkask_types::InfrastructureError;
use hkask_wallet_types::WalletError;

define_store!(WalletStore);

// ── WalletStore cross-cutting methods ──────────────────────────────────────────

impl WalletStore {
    /// Enable SQLite WAL (Write-Ahead Logging) mode for better concurrency.
    ///
    /// WAL mode allows concurrent reads while a write is in progress,
    /// significantly improving throughput under multi-agent API key spend loads.
    /// Without WAL, all operations serialize on the connection mutex.
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// post: journal_mode set to WAL
    /// post: synchronous set to NORMAL (balance durability vs performance)
    ///
    /// Call once after store creation, before any wallet operations.
    /// Enable WAL mode for better concurrency.
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — enable WAL for wallet concurrency
    /// \[P7\] Constraining: Evolutionary Architecture — WAL mode emerged from multi-agent load
    /// post: journal_mode set to WAL, synchronous set to NORMAL
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
}
