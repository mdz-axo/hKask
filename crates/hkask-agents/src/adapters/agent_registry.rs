//! Agent Registry Adapter — Concrete persistence for agent registry

use hkask_storage::AgentRegistryStore;
use hkask_types::RegisteredAgent;

/// Error type for agent registry operations
#[derive(Debug)]
pub enum AgentRegistryError {
    /// Storage-level error
    Storage(String),
    /// Agent not found
    NotFound(String),
    /// Schema initialization error
    Schema(String),
}

impl std::fmt::Display for AgentRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(e) => write!(f, "Storage error: {e}"),
            Self::NotFound(e) => write!(f, "Agent not found: {e}"),
            Self::Schema(e) => write!(f, "Schema error: {e}"),
        }
    }
}

impl std::error::Error for AgentRegistryError {}

pub struct AgentRegistryAdapter {
    store: AgentRegistryStore,
}

impl AgentRegistryAdapter {
    pub fn new(store: AgentRegistryStore) -> Self {
        Self { store }
    }

    pub fn inner(&self) -> &AgentRegistryStore {
        &self.store
    }

    pub fn initialize_schema(&self) -> Result<(), AgentRegistryError> {
        self.store
            .initialize_schema()
            .map_err(|e| AgentRegistryError::Schema(e.to_string()))
    }

    pub fn insert(&self, agent: &RegisteredAgent) -> Result<(), AgentRegistryError> {
        self.store
            .insert(agent)
            .map_err(|e| AgentRegistryError::Storage(e.to_string()))
    }

    pub fn remove(&self, name: &str) -> Result<(), AgentRegistryError> {
        self.store
            .remove(name)
            .map_err(|e| AgentRegistryError::Storage(e.to_string()))
    }

    pub fn list(&self) -> Result<Vec<RegisteredAgent>, AgentRegistryError> {
        self.store
            .list()
            .map_err(|e| AgentRegistryError::Storage(e.to_string()))
    }

    pub fn get(&self, name: &str) -> Result<Option<RegisteredAgent>, AgentRegistryError> {
        match self.store.get(name) {
            Ok(agent) => Ok(Some(agent)),
            Err(hkask_storage::AgentRegistryError::NotFound(_)) => Ok(None),
            Err(e) => Err(AgentRegistryError::Storage(e.to_string())),
        }
    }
}
