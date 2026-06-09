//! Error types — Cross-cutting infrastructure
//
//! Infrastructure errors used across all loops. Not loop-specific.!
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

// InfrastructureError — Cross-Crate Foundation

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
#[non_exhaustive]
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

// McpErrorKind — Canonical MCP Error Taxonomy

/// Semantic classification of MCP tool errors.
///
/// Inspired by `stack-domain-types::ErrorKind` (ADR-T7) and gRPC status codes.
/// Every MCP error variant maps to one `McpErrorKind`, enabling:
/// - Retry logic (retry on `Timeout`/`Unavailable`, don't on `InvalidArgument`)
/// - User-facing error categorization
/// - CNS observability bucketing by error class
/// - OCAP policy decisions (distinguish `PermissionDenied` from `NotFound`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
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
}

impl McpErrorKind {
    /// Whether errors of this kind are retryable with backoff.
    pub fn is_retryable(self) -> bool {
        matches!(self, Self::Unavailable | Self::Timeout | Self::RateLimited)
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
        }
    }
}

// HkaskError — Unified Error Hierarchy

/// Core error types shared across hKask crates.
///
/// Infrastructure failures (Database, Serialization, LockPoisoned, I/O, NotFound)
/// are delegated to [`InfrastructureError`] via `#[from]`. Domain enums should
/// prefer composing from `InfrastructureError` directly; `HkaskError` remains
/// for code that needs a single flat type with domain-neutral categories.
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
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

    pub fn not_found_typed(entity_type: &'static str, id: impl Into<String>) -> Self {
        InfrastructureError::NotFound(format!("{} not found: {}", entity_type, id.into())).into()
    }

    pub fn capability_denied(reason: impl Into<String>) -> Self {
        HkaskError::CapabilityDenied(reason.into())
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        false
    }

    /// Check if error requires user intervention
    pub fn requires_intervention(&self) -> bool {
        matches!(self, Self::CapabilityDenied(_) | Self::PermissionDenied(_))
    }

    /// Convert to McpErrorKind for MCP dispatch
    pub fn to_mcp_kind(&self) -> McpErrorKind {
        match self {
            Self::Infra(e) => match e {
                InfrastructureError::Database(_) => McpErrorKind::Internal,
                InfrastructureError::Serialization(_) => McpErrorKind::Internal,
                InfrastructureError::LockPoisoned => McpErrorKind::Internal,
                InfrastructureError::NotFound(_) => McpErrorKind::NotFound,
                InfrastructureError::Io(_) => McpErrorKind::Unavailable,
            },
            Self::CapabilityDenied(_) | Self::PermissionDenied(_) | Self::InvalidToken(_) => {
                McpErrorKind::PermissionDenied
            }
        }
    }
}

// GitError — Git CAS errors

/// Git CAS errors for content-addressable storage operations
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum GitError {
    #[error("Crate not found: {0}")]
    CrateNotFound(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Git error: {0}")]
    Git(String),
}

// Canonical domain error types — shared across all crates.
// Domain enums delegate to these rather than duplicating variants.

/// A resource was not found. Canonical across 17+ crates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotFound {
    pub entity_type: &'static str,
    pub id: String,
}

impl std::fmt::Display for NotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} not found: {}", self.entity_type, self.id)
    }
}

/// Capability denied — shared across 5+ crates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDenied {
    pub reason: String,
}

impl std::fmt::Display for CapabilityDenied {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "capability denied: {}", self.reason)
    }
}

/// Embedding dimension mismatch — duplicated across 2 crates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DimensionMismatch {
    pub expected: usize,
    pub actual: usize,
}

impl std::fmt::Display for DimensionMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "dimension mismatch: expected {}, got {}",
            self.expected, self.actual
        )
    }
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
