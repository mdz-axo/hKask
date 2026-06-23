//! Wallet types for hKask — rJoule payments, multi-chain deposits, API key capabilities.
//!
//! # Epistemic frame (pragmatic-semantics)
//! - rJoule is an internal accounting unit `[OUGHT-DECL]` — not an on-chain token
//! - Every rJoule originates from a verified on-chain deposit `[IS-DECL]`
//! - API keys are Ed25519-signed OCAP capability tokens `[OUGHT-DECL]`

pub mod chain;
pub mod error;
pub mod keys;

pub use chain::*;
pub use error::*;
pub use keys::*;

// Re-exports from hkask_wallet_types (canonical source for these types)
pub use hkask_wallet_types::{
    ChainId, DepositAddress, DepositReference, PriceFeedConfig, PrivacyMode, RJoule,
    TransactionType, TxHash, WalletBalance, WalletConfig, WalletError, WalletTransaction,
};
