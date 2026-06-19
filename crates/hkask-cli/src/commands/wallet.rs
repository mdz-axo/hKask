//! Wallet command handlers for `kask wallet`
//!
//! Implements CLI display logic for wallet operations: balance, deposits,
//! withdrawals, API key management, and transaction history.


use crate::cli::{KeyAction, WalletAction};
use hkask_services::WalletService;
use hkask_storage::WalletStore;
use hkask_storage::database::in_memory_db;
use hkask_types::wallet::{ChainId, PrivacyMode, RJoule, WalletConfig, WalletId};
use hkask_wallet::{ApiKeyIssuer, StaticPriceFeed, WalletManager};
use std::str::FromStr;
use std::sync::Arc;

/// Run a wallet subcommand. Builds a standalone WalletService for CLI use.
