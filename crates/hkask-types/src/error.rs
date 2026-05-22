//! Error types for hKask operations
//!
//! Domain-specific error types with recovery semantics.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Git archival operation errors
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitArchivalError {
    /// Adapter not configured or found
    #[error("Adapter not configured: {0}")]
    AdapterNotFound(String),

    /// Repository not found
    #[error("Repository not found: {owner}/{repo}")]
    RepositoryNotFound { owner: String, repo: String },

    /// Commit operation failed
    #[error("Commit failed: {0}")]
    CommitFailed(String),

    /// Network error during archival
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Capability token missing or insufficient
    #[error("Capability denied: {0}")]
    CapabilityDenied(String),

    /// Sovereignty boundary violation
    #[error("Sovereignty denied: {0}")]
    SovereigntyDenied(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded. Retry after {retry_after} seconds")]
    RateLimitExceeded { retry_after: u64 },

    /// Invalid archival path
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Serialization error
    #[error("Serialization failed: {0}")]
    SerializationError(String),
}

impl GitArchivalError {
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(self, Self::NetworkError(_) | Self::RateLimitExceeded { .. })
    }

    /// Check if error requires user intervention
    pub fn requires_user_intervention(&self) -> bool {
        matches!(
            self,
            Self::CapabilityDenied(_)
                | Self::SovereigntyDenied(_)
                | Self::RepositoryNotFound { .. }
        )
    }
}

/// Archival operation result
pub type ArchivalResult<T> = Result<T, GitArchivalError>;
