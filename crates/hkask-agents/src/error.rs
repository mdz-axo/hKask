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

/// Shared core error variants used across multiple agent domains.
///
/// Consolidates `Infra(#[from] InfrastructureError)`, `NoSnapshot`,
/// and `Acp` delegation that were previously duplicated across
/// `MemoryError`, `EscalationError`, `MetacognitionError`, and `A2AError`.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Infrastructure failure (DB, IO, serialization, etc.)
    #[error(transparent)]
    Infra(#[from] hkask_types::InfrastructureError),

    /// No health snapshot available for metacognition cycle
    #[error("No snapshot available for metacognition cycle")]
    NoSnapshot,

    /// ACP protocol failure
    #[error(transparent)]
    Acp(#[from] crate::a2a::A2AError),
}

impl From<rusqlite::Error> for CoreError {
    fn from(e: rusqlite::Error) -> Self {
        CoreError::Infra(hkask_types::InfrastructureError::Database(e.to_string()))
    }
}

/// Memory storage errors
///
/// Composes from `CoreError` for infrastructure transport-layer failures
/// and adds a `CapabilityDenied` variant for OCAP visibility/perspective constraints.
///
/// Matches the pattern used by `ConsentError`, `GoalRepositoryError`,
/// `AgentRegistryError`, and other domain error types.
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error(transparent)]
    Core(#[from] CoreError),

    #[error("Capability denied: {action} on {resource}")]
    CapabilityDenied { resource: String, action: String },
}

impl From<hkask_types::InfrastructureError> for MemoryError {
    fn from(e: hkask_types::InfrastructureError) -> Self {
        MemoryError::Core(CoreError::Infra(e))
    }
}

impl From<hkask_storage::DatabaseError> for MemoryError {
    fn from(e: hkask_storage::DatabaseError) -> Self {
        MemoryError::Core(CoreError::Infra(
            hkask_types::InfrastructureError::Database(e.to_string()),
        ))
    }
}

impl From<hkask_memory::EpisodicMemoryError> for MemoryError {
    fn from(e: hkask_memory::EpisodicMemoryError) -> Self {
        match e {
            hkask_memory::EpisodicMemoryError::Triple(inner) => inner.into(),
            hkask_memory::EpisodicMemoryError::InvalidVisibility(msg) => {
                MemoryError::CapabilityDenied {
                    resource: "episodic_memory".into(),
                    action: msg,
                }
            }
            hkask_memory::EpisodicMemoryError::MissingPerspective => {
                MemoryError::CapabilityDenied {
                    resource: "episodic_memory".into(),
                    action: "requires a perspective (agent WebID)".into(),
                }
            }
        }
    }
}

impl From<hkask_memory::SemanticMemoryError> for MemoryError {
    fn from(e: hkask_memory::SemanticMemoryError) -> Self {
        match e {
            hkask_memory::SemanticMemoryError::Triple(inner) => inner.into(),
            hkask_memory::SemanticMemoryError::Embedding(inner) => inner.into(),
            hkask_memory::SemanticMemoryError::InvalidVisibility(msg) => {
                MemoryError::CapabilityDenied { resource: "semantic_memory".into(), action: msg }
            }
            hkask_memory::SemanticMemoryError::NoEmbeddingsForCentroid(msg) => {
                MemoryError::Core(CoreError::Infra(hkask_types::InfrastructureError::NotFound(msg)))
            }
            hkask_memory::SemanticMemoryError::HasPerspective => MemoryError::CapabilityDenied {
                resource: "semantic_memory".into(),
                action: "requires no perspective (use consolidation bridge for episodic→semantic promotion)".into(),
            },
        }
    }
}

impl From<hkask_storage::TripleError> for MemoryError {
    fn from(e: hkask_storage::TripleError) -> Self {
        match e {
            hkask_storage::TripleError::Infra(inner) => MemoryError::Core(CoreError::Infra(inner)),
            hkask_storage::TripleError::NotFound => MemoryError::Core(CoreError::Infra(
                hkask_types::InfrastructureError::NotFound("triple".into()),
            )),
        }
    }
}

impl From<hkask_storage::EmbeddingError> for MemoryError {
    fn from(e: hkask_storage::EmbeddingError) -> Self {
        match e {
            hkask_storage::EmbeddingError::Infrastructure(inner) => {
                MemoryError::Core(CoreError::Infra(inner))
            }
            hkask_storage::EmbeddingError::NotFound(msg) => MemoryError::Core(CoreError::Infra(
                hkask_types::InfrastructureError::NotFound(msg),
            )),
            hkask_storage::EmbeddingError::DimensionMismatch { .. } => {
                MemoryError::Core(CoreError::Infra(
                    hkask_types::InfrastructureError::Serialization(e.to_string()),
                ))
            }
            hkask_storage::EmbeddingError::Storage(_) => MemoryError::Core(CoreError::Infra(
                hkask_types::InfrastructureError::Database(e.to_string()),
            )),
            hkask_storage::EmbeddingError::Decode(msg) => MemoryError::Core(CoreError::Infra(
                hkask_types::InfrastructureError::Serialization(msg),
            )),
        }
    }
}

/// Registry source errors
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("IO error: {0}")]
    Io(String),
}
