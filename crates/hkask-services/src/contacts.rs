//! Contact registry — agent-native contact management.
//! Each agent owns its own contacts, stored in the registry DB.
//! These are direct crate calls, not MCP tools.

use hkask_rsolidity::contract;

use hkask_storage::AgentRegistryStore;
use hkask_types::Contact;

use crate::ServiceError;

pub struct ContactService;

impl ContactService {
    /// Add a contact to an agent's registry.
    ///
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
            .map_err(|e| ServiceError::AgentRegistryStore {
                message: e.to_string(),
            })
    }

    /// Find contacts by name or relationship. Returns all matches.
    ///
    pub fn find(
        store: &AgentRegistryStore,
        agent_name: &str,
        query: &str,
    ) -> Result<Vec<Contact>, ServiceError> {
        store
            .find_contacts(agent_name, query)
            .map_err(|e| ServiceError::AgentRegistryStore {
                message: e.to_string(),
            })
    }

    /// List all contacts for an agent.
    ///
    pub fn list(
        store: &AgentRegistryStore,
        agent_name: &str,
    ) -> Result<Vec<Contact>, ServiceError> {
        store
            .list_contacts(agent_name)
            .map_err(|e| ServiceError::AgentRegistryStore {
                message: e.to_string(),
            })
    }
}
