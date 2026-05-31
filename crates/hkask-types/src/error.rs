//! Error types for hKask operations
//!
//! Layered error architecture (Miller separation):
//! 1. `InfrastructureError` — cross-cutting transport errors (Database, Serialization,
//!    LockPoisoned, Io). No domain semantics. Passes through crate boundaries.
//! 2. `HkaskError` — the legacy consolidation type; delegates to `InfrastructureError`
//!    for generic categories and adds domain-neutral variants.
//! 3. Domain enums (e.g. `GoalRepositoryError`, `EmbeddingError`) — compose from
//!    `InfrastructureError` via `#[from]` and add only authority-bearing,
//!    recovery-path-significant variants.
//!
//! Rule: if a variant name appears in 3+ crates with identical semantics, it
//! belongs in `InfrastructureError`. If it carries domain-specific recovery
//! semantics, it stays in the domain enum.

use serde::{Deserialize, Serialize};
use std::sync::PoisonError;
use thiserror::Error;

// =============================================================================
// InfrastructureError — Cross-Crate Foundation
// =============================================================================

/// Generic infrastructure errors shared by every crate.
///
/// These are transport-layer failures — they carry no domain semantics.
/// Every domain enum may compose from this via `#[from]` to eliminate
/// the 87× repetition of `Database(String)` / `Serialization(String)` /
/// `LockPoisoned(String)` spread across the codebase.
///
/// Design constraint (C5): every variant is a distinct recovery category —
/// no catch-all, no `Other(String)`, no `Internal(String)`.
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InfrastructureError {
    #[error("database: {0}")]
    Database(String),

    #[error("serialization: {0}")]
    Serialization(String),

    #[error("lock poisoned")]
    LockPoisoned,

    #[error("not found: {0}")]
    NotFound(String),

    #[error("io: {0}")]
    Io(String),
}

// From impls for the canonical error sources.
// Note: no From<rusqlite::Error> here — hkask-types does not depend on rusqlite.
// Downstream crates should wrap rusqlite errors into InfrastructureError::Database(String).
impl From<serde_json::Error> for InfrastructureError {
    fn from(e: serde_json::Error) -> Self {
        InfrastructureError::Serialization(e.to_string())
    }
}

impl From<std::io::Error> for InfrastructureError {
    fn from(e: std::io::Error) -> Self {
        InfrastructureError::Io(e.to_string())
    }
}

impl<T> From<PoisonError<T>> for InfrastructureError {
    fn from(_: PoisonError<T>) -> Self {
        InfrastructureError::LockPoisoned
    }
}

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
    ///
    /// This is an external API boundary rate limiter — it protects MCP servers
    /// from external client DoS, distinct from internal energy budget tracking.
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
        matches!(self, Self::NetworkError(_))
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

// =============================================================================
// HkaskError — Unified Error Hierarchy
// =============================================================================

/// Core error types shared across hKask crates.
///
/// Infrastructure failures (Database, Serialization, LockPoisoned, I/O, NotFound)
/// are delegated to [`InfrastructureError`] via `#[from]`. Domain enums should
/// prefer composing from `InfrastructureError` directly; `HkaskError` remains
/// for code that needs a single flat type with domain-neutral categories.
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HkaskError {
    /// Infrastructure transport failure (Database, Serialization, etc.)
    #[error(transparent)]
    Infra(#[from] InfrastructureError),

    #[error("capability denied: {0}")]
    CapabilityDenied(String),

    #[error("invalid token: {0}")]
    InvalidToken(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("network: {0}")]
    Network(String),

    #[error("configuration: {0}")]
    Config(String),

    #[error("validation: {0}")]
    Validation(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),
}

impl HkaskError {
    pub fn database(msg: impl Into<String>) -> Self {
        InfrastructureError::Database(msg.into()).into()
    }

    pub fn serialization(msg: impl Into<String>) -> Self {
        InfrastructureError::Serialization(msg.into()).into()
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        InfrastructureError::NotFound(msg.into()).into()
    }

    pub fn io_error(msg: impl Into<String>) -> Self {
        InfrastructureError::Io(msg.into()).into()
    }

    pub fn lock_poisoned() -> Self {
        InfrastructureError::LockPoisoned.into()
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Network(_))
    }

    /// Check if error requires user intervention
    pub fn requires_intervention(&self) -> bool {
        matches!(
            self,
            Self::CapabilityDenied(_) | Self::PermissionDenied(_) | Self::Config(_)
        )
    }

    /// Convert to McpErrorKind for MCP dispatch
    pub fn to_mcp_kind(&self) -> McpErrorKind {
        match self {
            Self::Infra(e) => match e {
                InfrastructureError::Database(_) => McpErrorKind::Internal,
                InfrastructureError::Serialization(_) => McpErrorKind::DataLoss,
                InfrastructureError::LockPoisoned => McpErrorKind::Internal,
                InfrastructureError::NotFound(_) => McpErrorKind::NotFound,
                InfrastructureError::Io(_) => McpErrorKind::Unavailable,
            },
            Self::CapabilityDenied(_) | Self::PermissionDenied(_) | Self::InvalidToken(_) => {
                McpErrorKind::PermissionDenied
            }
            Self::Network(_) => McpErrorKind::Unavailable,
            Self::Config(_) | Self::Validation(_) | Self::InvalidInput(_) => {
                McpErrorKind::InvalidArgument
            }
        }
    }
}

// =============================================================================
// GitError — Git CAS errors
// =============================================================================

/// Git CAS errors for content-addressable storage operations
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitError {
    #[error("Crate not found: {0}")]
    CrateNotFound(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Parse error: {0}")]
    Parse(String),
}

// Conversions from common error types
impl From<std::io::Error> for HkaskError {
    fn from(err: std::io::Error) -> Self {
        InfrastructureError::Io(err.to_string()).into()
    }
}

impl From<serde_json::Error> for HkaskError {
    fn from(err: serde_json::Error) -> Self {
        InfrastructureError::Serialization(err.to_string()).into()
    }
}
