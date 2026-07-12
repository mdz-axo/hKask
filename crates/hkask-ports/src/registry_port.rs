//! RegistryPort — trait boundary for agent registry storage.
//!
//! Decouples agent pods from the concrete `AgentRegistryStore` in hkask-storage.

use hkask_types::InfrastructureError;
use hkask_types::agent_registry::RegisteredAgent;

/// Port trait for agent registry persistence.
pub trait RegistryPort: Send + Sync {
    /// Initialize the registry schema.
    fn initialize_schema(&self) -> Result<(), InfrastructureError>;

    /// List all registered agents.
    fn list(&self) -> Result<Vec<RegisteredAgent>, InfrastructureError>;

    /// Insert or update a registered agent.
    fn insert(&self, agent: &RegisteredAgent) -> Result<(), InfrastructureError>;
}
