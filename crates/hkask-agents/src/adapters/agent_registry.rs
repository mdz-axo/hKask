//! Agent Registry Adapter — Bridges AgentRegistryStore to AgentRegistryPort

use crate::ports::{AgentRegistryPort, AgentRegistryPortError};
use hkask_storage::AgentRegistryStore;
use hkask_types::RegisteredAgent;

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
}

impl AgentRegistryPort for AgentRegistryAdapter {
    fn initialize_schema(&self) -> Result<(), AgentRegistryPortError> {
        self.store
            .initialize_schema()
            .map_err(|e| AgentRegistryPortError::Schema(e.to_string()))
    }

    fn insert(&self, agent: &RegisteredAgent) -> Result<(), AgentRegistryPortError> {
        self.store
            .insert(agent)
            .map_err(|e| AgentRegistryPortError::Storage(e.to_string()))
    }

    fn remove(&self, name: &str) -> Result<(), AgentRegistryPortError> {
        self.store
            .remove(name)
            .map_err(|e| AgentRegistryPortError::Storage(e.to_string()))
    }

    fn list(&self) -> Result<Vec<RegisteredAgent>, AgentRegistryPortError> {
        self.store
            .list()
            .map_err(|e| AgentRegistryPortError::Storage(e.to_string()))
    }

    fn get(&self, name: &str) -> Result<Option<RegisteredAgent>, AgentRegistryPortError> {
        match self.store.get(name) {
            Ok(agent) => Ok(Some(agent)),
            Err(hkask_storage::AgentRegistryError::NotFound(_)) => Ok(None),
            Err(e) => Err(AgentRegistryPortError::Storage(e.to_string())),
        }
    }
}
