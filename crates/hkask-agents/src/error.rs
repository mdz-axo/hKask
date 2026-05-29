//! Port error types for type-safe error handling
//!
//! Each port has its own error type to provide specific error information
//! and enable proper error handling at call sites.

use thiserror::Error;

/// MCP runtime errors
#[derive(Debug, Error)]
pub enum McpError {
    #[error("Invalid capability token: {0}")]
    InvalidToken(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Tool invocation failed: {0}")]
    InvocationFailed(String),

    #[error("Capability denied: {0}")]
    CapabilityDenied(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),
}

/// Git CAS errors
///
/// Re-exported from `hkask_types::GitError` for backward compatibility.
/// The canonical definition lives in `hkask_types`.
pub use hkask_types::GitError;

/// Memory storage errors
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Invalid artifact type: {0}")]
    InvalidArtifactType(String),

    #[error("Capability denied: {0}")]
    CapabilityDenied(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Registry source errors
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("IO error: {0}")]
    Io(String),

    #[error("YAML parse error: {0}")]
    YamlParse(String),
}
