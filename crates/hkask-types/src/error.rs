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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = GitArchivalError::AdapterNotFound("git".to_string());
        assert_eq!(err.to_string(), "Adapter not configured: git");
    }

    #[test]
    fn test_error_recovery_classification() {
        let network_err = GitArchivalError::NetworkError("timeout".to_string());
        assert!(network_err.is_recoverable());
        assert!(!network_err.requires_user_intervention());

        let cap_err = GitArchivalError::CapabilityDenied("missing token".to_string());
        assert!(!cap_err.is_recoverable());
        assert!(cap_err.requires_user_intervention());
    }

    #[test]
    fn test_error_serialization() {
        let err = GitArchivalError::RepositoryNotFound {
            owner: "test".to_string(),
            repo: "repo".to_string(),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("Repository not found"));
    }
}
