//! Contact registry — agent-native contact management.
//! Each agent owns its own contacts, stored in the registry DB.
//! These are direct crate calls, not MCP tools.

use hkask_storage::AgentRegistryStore;
use hkask_types::Contact;

use crate::ServiceError;

pub struct ContactService;

impl ContactService {
    /// Add a contact to an agent's registry.
    ///
    /// REQ: P1-svc-contacts-122
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  store must be initialized; agent_name and contact_name must be non-empty
    /// post: contact is persisted to the registry store; Err(AgentRegistryStore) on store failure
    pub fn add(
        store: &AgentRegistryStore,
        agent_name: &str,
        contact_name: &str,
        relationship: Option<&str>,
        notes: Option<&str>,
    ) -> Result<(), ServiceError> {
        let contact = Contact {
            agent_name: agent_name.to_string(),
            contact_name: contact_name.to_string(),
            relationship: relationship.map(|s| s.to_string()),
            notes: notes.map(|s| s.to_string()),
        };
        store
            .add_contact(&contact)
            .map_err(|e| ServiceError::AgentRegistryStore { message: e.to_string() })
    }

    /// Find contacts by name or relationship. Returns all matches.
    ///
    /// REQ: P1-svc-contacts-123
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  store must be initialized; agent_name and query must be non-empty
    /// post: returns Vec<Contact> matching the query; empty Vec if no matches; Err(AgentRegistryStore) on store failure
    pub fn find(
        store: &AgentRegistryStore,
        agent_name: &str,
        query: &str,
    ) -> Result<Vec<Contact>, ServiceError> {
        store
            .find_contacts(agent_name, query)
            .map_err(|e| ServiceError::AgentRegistryStore { message: e.to_string() })
    }

    /// List all contacts for an agent.
    ///
    /// REQ: P1-svc-contacts-124
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  store must be initialized; agent_name must be non-empty
    /// post: returns Vec<Contact> for the agent; empty Vec if no contacts; Err(AgentRegistryStore) on store failure
    pub fn list(
        store: &AgentRegistryStore,
        agent_name: &str,
    ) -> Result<Vec<Contact>, ServiceError> {
        store
            .list_contacts(agent_name)
            .map_err(|e| ServiceError::AgentRegistryStore { message: e.to_string() })
    }
}
