//! Error types for hKask operations
//!
//! Domain-specific error types with recovery semantics.
//! Includes the canonical `McpErrorKind` taxonomy for MCP tool dispatch
//! classification — every MCP error variant maps to a kind so the dispatch
//! layer can reason about failures without parsing message strings.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// =============================================================================
// McpErrorKind — Canonical MCP Error Taxonomy
// =============================================================================

/// Semantic classification of MCP tool errors.
///
/// Inspired by `stack-domain-types::ErrorKind` (ADR-T7) and gRPC status codes.
/// Every MCP error variant maps to one `McpErrorKind`, enabling:
/// - Retry logic (retry on `Timeout`/`Unavailable`, don't on `InvalidArgument`)
/// - User-facing error categorization
/// - CNS observability bucketing by error class
/// - OCAP policy decisions (distinguish `PermissionDenied` from `NotFound`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum McpErrorKind {
    /// Internal server error (bug, unexpected state).
    Internal,
    /// External service unavailable (network, upstream down).
    Unavailable,
    /// Operation timed out.
    Timeout,
    /// Resource or tool not found.
    NotFound,
    /// Invalid arguments or schema validation failure.
    InvalidArgument,
    /// OCAP capability denied or insufficient permissions.
    PermissionDenied,
    /// Rate limit exceeded (retryable after backoff).
    RateLimited,
    /// Precondition not met (server not initialized, feature disabled).
    FailedPrecondition,
    /// Resource already exists (idempotent rejection).
    AlreadyExists,
    /// Data corruption or loss (deserialization failure).
    DataLoss,
    /// Operation cancelled by caller or supervisor.
    Cancelled,
}

impl McpErrorKind {
    /// Whether errors of this kind are retryable with backoff.
    pub fn is_retryable(self) -> bool {
        matches!(
            self,
            Self::Unavailable | Self::Timeout | Self::RateLimited | Self::Cancelled
        )
    }

    /// Whether this error requires user/admin intervention.
    pub fn requires_intervention(self) -> bool {
        matches!(self, Self::PermissionDenied | Self::FailedPrecondition)
    }
}

impl std::fmt::Display for McpErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal => write!(f, "internal"),
            Self::Unavailable => write!(f, "unavailable"),
            Self::Timeout => write!(f, "timeout"),
            Self::NotFound => write!(f, "not_found"),
            Self::InvalidArgument => write!(f, "invalid_argument"),
            Self::PermissionDenied => write!(f, "permission_denied"),
            Self::RateLimited => write!(f, "rate_limited"),
            Self::FailedPrecondition => write!(f, "failed_precondition"),
            Self::AlreadyExists => write!(f, "already_exists"),
            Self::DataLoss => write!(f, "data_loss"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

// =============================================================================
// GitArchivalError
// =============================================================================
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

/// AuthorizationError — Canonical authorization failure for hKask
///
/// Common authorization error variants shared across subsystems.
/// Domain-specific errors should use dedicated types (WebIdAuthError,
/// CapabilityAuthError, StepAuthError).
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthorizationError {
    #[error("Capability not found")]
    CapabilityNotFound,

    #[error("Capability expired")]
    CapabilityExpired,

    #[error("Unauthorized operation: {0}")]
    Unauthorized(String),

    #[error("Insufficient permissions: requested {requested}, granted {granted}")]
    InsufficientPermissions { requested: String, granted: String },
}
