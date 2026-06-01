//! Agent Registry Adapter — Concrete persistence for agent registry

use hkask_storage::{AgentRegistryError, AgentRegistryStore};
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

    pub fn initialize_schema(&self) -> Result<(), AgentRegistryError> {
        self.store.initialize_schema()
    }

    pub fn insert(&self, agent: &RegisteredAgent) -> Result<(), AgentRegistryError> {
        self.store.insert(agent)
    }

    pub fn remove(&self, name: &str) -> Result<(), AgentRegistryError> {
        self.store.remove(name)
    }

    pub fn list(&self) -> Result<Vec<RegisteredAgent>, AgentRegistryError> {
        self.store.list()
    }

    pub fn get(&self, name: &str) -> Result<Option<RegisteredAgent>, AgentRegistryError> {
        match self.store.get(name) {
            Ok(agent) => Ok(Some(agent)),
            Err(AgentRegistryError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
