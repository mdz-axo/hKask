//! Agent Registry Port — Hexagonal boundary for agent persistence

use hkask_types::RegisteredAgent;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentRegistryPortError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Agent not found: {0}")]
    NotFound(String),
    #[error("Schema error: {0}")]
    Schema(String),
}

/// Port trait for agent registry persistence
///
/// Implementations:
/// - `AgentRegistryStore` — Production adapter via SQLite
/// - Mock implementations for testing
pub trait AgentRegistryPort: Send + Sync {
    fn initialize_schema(&self) -> Result<(), AgentRegistryPortError>;

    fn insert(&self, agent: &RegisteredAgent) -> Result<(), AgentRegistryPortError>;

    fn remove(&self, name: &str) -> Result<(), AgentRegistryPortError>;

    fn list(&self) -> Result<Vec<RegisteredAgent>, AgentRegistryPortError>;

    fn get(&self, name: &str) -> Result<Option<RegisteredAgent>, AgentRegistryPortError>;
}
