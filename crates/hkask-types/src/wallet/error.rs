//! Wallet error domain — typed errors with context.

use chrono::{DateTime, Utc};

use super::chain::ChainId;
use super::types::RJoule;
use crate::error::InfrastructureError;
use crate::id::ApiKeyId;

/// Wallet-specific error domain.
///
/// # Design principles (rust-expertise §7) `[OUGHT-DECL]`
/// - Typed errors for library code (`thiserror`)
/// - Each variant carries context, not just a name
/// - Never discard errors silently
#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("infrastructure error: {0}")]
    Infra(InfrastructureError),

    #[error("insufficient rJoule balance: have {have}, need {need}")]
    InsufficientBalance { have: RJoule, need: RJoule },

    #[error("API key {key_id} spending limit exceeded: {spent} / {limit}")]
    SpendingLimitExceeded {
        key_id: ApiKeyId,
        spent: RJoule,
        limit: RJoule,
    },

    #[error("API key {key_id} expired at {expiry}")]
    KeyExpired {
        key_id: ApiKeyId,
        expiry: DateTime<Utc>,
    },

    #[error("API key {key_id} has been revoked")]
    KeyRevoked { key_id: ApiKeyId },

    #[error("chain {chain} is not enabled for this wallet")]
    ChainNotEnabled { chain: ChainId },

    #[error("privacy layer unavailable for chain {chain}")]
    PrivacyUnavailable { chain: ChainId },

    #[error("deposit reference {reference} not found or expired")]
    DepositReferenceInvalid { reference: String },

    #[error("chain error ({chain}): {message}")]
    ChainError { chain: ChainId, message: String },

    #[error("privacy layer error: {message}")]
    PrivacyError { message: String },

    #[error("API key {key_id} already has an active encumbrance")]
    EncumbranceAlreadyExists { key_id: ApiKeyId },

    #[error("no active encumbrance found for API key {key_id}")]
    EncumbranceNotFound { key_id: ApiKeyId },

    #[error(
        "encumbrance for key {key_id} has insufficient remaining: have {remaining}, need {need}"
    )]
    EncumbranceInsufficient {
        key_id: ApiKeyId,
        remaining: RJoule,
        need: RJoule,
    },
}

impl From<InfrastructureError> for WalletError {
    fn from(e: InfrastructureError) -> Self {
        WalletError::Infra(e)
    }
}

#[cfg(feature = "sql")]
impl From<rusqlite::Error> for WalletError {
    fn from(e: rusqlite::Error) -> Self {
        WalletError::Infra(InfrastructureError::Database(e.to_string()))
    }
}

impl From<uuid::Error> for WalletError {
    fn from(e: uuid::Error) -> Self {
        WalletError::Infra(InfrastructureError::Database(e.to_string()))
    }
}
