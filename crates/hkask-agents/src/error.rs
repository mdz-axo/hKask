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

    #[error("Capability denied: agent lacks permission for {action} on {resource}")]
    CapabilityDenied { resource: String, action: String },

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Tool invocation failed: {0}")]
    InvocationFailed(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("No MCP runtime wired: {0}")]
    NoRuntime(String),
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
    Storage(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Query error: {0}")]
    Query(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Capability denied: {0}")]
    CapabilityDenied(String),
}

impl From<hkask_storage::DatabaseError> for MemoryError {
    fn from(e: hkask_storage::DatabaseError) -> Self {
        MemoryError::Storage(Box::new(e))
    }
}

impl From<hkask_memory::EpisodicMemoryError> for MemoryError {
    fn from(e: hkask_memory::EpisodicMemoryError) -> Self {
        match &e {
            hkask_memory::EpisodicMemoryError::Triple(_) => MemoryError::Storage(Box::new(e)),
            hkask_memory::EpisodicMemoryError::InvalidVisibility(_)
            | hkask_memory::EpisodicMemoryError::MissingPerspective => {
                MemoryError::Query(Box::new(e))
            }
        }
    }
}

impl From<hkask_memory::SemanticMemoryError> for MemoryError {
    fn from(e: hkask_memory::SemanticMemoryError) -> Self {
        match &e {
            hkask_memory::SemanticMemoryError::Triple(_)
            | hkask_memory::SemanticMemoryError::Embedding(_) => MemoryError::Storage(Box::new(e)),
            hkask_memory::SemanticMemoryError::InvalidVisibility(_)
            | hkask_memory::SemanticMemoryError::NoEmbeddingsForCentroid(_)
            | hkask_memory::SemanticMemoryError::HasPerspective => MemoryError::Query(Box::new(e)),
        }
    }
}

/// Registry source errors
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("IO error: {0}")]
    Io(String),
}
