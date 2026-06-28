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
