//! AgentRegistryStore — Persistent storage for registered agents
use hkask_database::driver::{query_map, query_row};
use hkask_database::value::DbValue;
use hkask_storage_core::{define_driver_store, impl_from_db_error};
use hkask_types::AgentKind;
use hkask_types::InfrastructureError;
use hkask_types::NotFound;
use hkask_types::agent_registry::{
    AgentDefinition, Contact, RegisteredAgent, ScheduledTask, UserProfile,
};
use thiserror::Error;
#[derive(Error, Debug)]
pub enum AgentRegistryError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),
    #[error("Agent not found: {0}")]
    NotFound(NotFound),
    #[error("Agent already registered: {0}")]
    AlreadyRegistered(String),
}
impl_from_db_error!(AgentRegistryError, Infra);
define_driver_store!(AgentRegistryStore);
impl AgentRegistryStore {
    /// Initialize the agent registry schema.
    ///
    /// expect: "The system provides durable storage for agent registry data"
    /// \[P3\] Motivating: Generative Space — agent registry schema
    /// post: agents, user_profiles, contacts, scheduled_tasks tables created
    fn init_schema(driver: &std::sync::Arc<dyn hkask_database::driver::DatabaseDriver>) {
        let _ = driver.execute_batch(
            "CREATE TABLE IF NOT EXISTS agent_registry (
                name TEXT PRIMARY KEY,
                agent_kind TEXT,
                definition_json TEXT NOT NULL,
                token_hash TEXT NOT NULL,
                registered_at TEXT NOT NULL,
                source_yaml TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_agent_registry_kind ON agent_registry(agent_kind);
            CREATE TABLE IF NOT EXISTS user_profile (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                profile_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS contacts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                agent_name TEXT NOT NULL,
                contact_name TEXT NOT NULL,
                relationship TEXT,
                notes TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_contacts_agent ON contacts(agent_name);
            CREATE TABLE IF NOT EXISTS scheduled_tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                agent_name TEXT NOT NULL,
                trigger_expr TEXT NOT NULL,
                action TEXT NOT NULL,
                params TEXT,
                next_run TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_scheduled_agent ON scheduled_tasks(agent_name);",
        );
    }
    /// Insert a registered agent.
    ///
    /// expect: "The system provides durable storage for agent registry data"
    /// \[P3\] Motivating: Generative Space — insert registered agent
    /// pre:  agent.name is non-empty
    /// post: agent inserted into agents table
    pub fn insert(&self, agent: &RegisteredAgent) -> Result<(), AgentRegistryError> {
        let definition_json = serde_json::to_string(&agent.definition)
            .map_err(|e| AgentRegistryError::Infra(InfrastructureError::from(e)))?;
        self.driver.execute(
            "INSERT OR REPLACE INTO agent_registry (name, definition_json, token_hash, registered_at, source_yaml)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            &[
                DbValue::Text(agent.definition.name.clone()),
                DbValue::Text(definition_json),
                DbValue::Text(agent.token_hash.clone()),
                DbValue::Text(agent.registered_at.clone()),
                DbValue::Text(agent.source_yaml.clone()),
            ],
        )?;
        Ok(())
    }
    /// Get an agent by name.
    ///
    /// expect: "The system provides durable storage for agent registry data"
    /// \[P3\] Motivating: Generative Space — get agent by name
    /// pre:  name is non-empty
    /// post: returns RegisteredAgent if found
    #[must_use = "result must be used"]
    pub fn get(&self, name: &str) -> Result<RegisteredAgent, AgentRegistryError> {
        let agent = query_row(
            &*self.driver,
            "SELECT definition_json, token_hash, registered_at, source_yaml
             FROM agent_registry WHERE name = ?1",
            &[DbValue::Text(name.to_string())],
            |row| {
                Ok((
                    row.get_str(0)?.to_string(),
                    row.get_str(1)?.to_string(),
                    row.get_str(2)?.to_string(),
                    row.get_str(3)?.to_string(),
                ))
            },
        )?
        .ok_or_else(|| {
            AgentRegistryError::NotFound(NotFound {
                entity_type: "agent".to_string(),
                id: name.to_string(),
            })
        })?;
        let definition: AgentDefinition = serde_json::from_str(&agent.0)
            .map_err(|e| AgentRegistryError::Infra(InfrastructureError::from(e)))?;
        Ok(RegisteredAgent {
            definition,
            token_hash: agent.1,
            registered_at: agent.2,
            source_yaml: agent.3,
        })
    }
    /// List all registered agents.
    ///
    /// expect: "The system provides durable storage for agent registry data"
    /// \[P3\] Motivating: Generative Space — list all agents
    /// post: returns Vec of all RegisteredAgent
    #[must_use = "result must be used"]
    pub fn list(&self) -> Result<Vec<RegisteredAgent>, AgentRegistryError> {
        Ok(query_map(
            &*self.driver,
            "SELECT definition_json, token_hash, registered_at, source_yaml
             FROM agent_registry ORDER BY name ",
            &[],
            |row| {
                let def_json: String = row.get_str(0)?.to_string();
                let token_hash: String = row.get_str(1)?.to_string();
                let registered_at: String = row.get_str(2)?.to_string();
                let source_yaml: String = row.get_str(3)?.to_string();
                let definition: AgentDefinition = serde_json::from_str(&def_json)
                    .map_err(|e| hkask_database::types::DbError::Database(e.to_string()))?;
                Ok(RegisteredAgent {
                    definition,
                    token_hash,
                    registered_at,
                    source_yaml,
                })
            },
        )?)
    }

    /// Remove an agent by name.
    ///
    /// expect: "The system provides durable storage for agent registry data"
    /// \[P3\] Motivating: Generative Space — remove agent
    /// pre:  name is non-empty
    /// post: agent deleted if existed
    pub fn remove(&self, name: &str) -> Result<(), AgentRegistryError> {
        let deleted = self.driver.execute(
            "DELETE FROM agent_registry WHERE name = ?1",
            &[DbValue::Text(name.to_string())],
        )?;
        if deleted == 0 {
            return Err(AgentRegistryError::NotFound(NotFound {
                entity_type: "agent".to_string(),
                id: name.to_string(),
            }));
        }
        Ok(())
    }
    /// Store the human user's profile. Replaces any existing profile (single-row table).
    /// Store a user profile.
    ///
    /// expect: "The system provides durable storage for agent registry data"
    /// \[P3\] Motivating: Generative Space — store user profile
    /// pre:  profile has valid fields
    /// post: profile upserted
    pub fn store_user_profile(&self, profile: &UserProfile) -> Result<(), AgentRegistryError> {
        let json = serde_json::to_string(profile)
            .map_err(|e| AgentRegistryError::Infra(InfrastructureError::from(e)))?;
        self.driver.execute(
            "INSERT OR REPLACE INTO user_profile (id, profile_json) VALUES (1, ?1)",
            &[DbValue::Text(json)],
        )?;
        Ok(())
    }
    /// Retrieve the human user's profile. Returns None if no profile has been stored.
    /// Get the user profile.
    ///
    /// expect: "The system provides durable storage for agent registry data"
    /// \[P3\] Motivating: Generative Space — get user profile
    /// post: returns Some(profile) if exists, None otherwise
    pub fn get_user_profile(&self) -> Result<Option<UserProfile>, AgentRegistryError> {
        let json: Option<String> = query_row(
            &*self.driver,
            "SELECT profile_json FROM user_profile WHERE id = 1",
            &[],
            |row| Ok(row.get_str(0)?.to_string()),
        )?;
        match json {
            Some(j) => Ok(Some(serde_json::from_str(&j).map_err(|e| {
                AgentRegistryError::Infra(InfrastructureError::from(e))
            })?)),
            None => Ok(None),
        }
    }
    /// Add a contact to an agent's contact registry.
    /// Add a contact.
    ///
    /// expect: "The system provides durable storage for agent registry data"
    /// \[P3\] Motivating: Generative Space — add contact
    /// pre:  contact has valid fields
    /// post: contact inserted
    pub fn add_contact(&self, contact: &Contact) -> Result<(), AgentRegistryError> {
        self.driver.execute(
            "INSERT INTO contacts (agent_name, contact_name, relationship, notes)
             VALUES (?1, ?2, ?3, ?4)",
            &[
                DbValue::Text(contact.agent_name.clone()),
                DbValue::Text(contact.contact_name.clone()),
                contact
                    .relationship
                    .as_ref()
                    .map_or(DbValue::Null, |r| DbValue::Text(r.clone())),
                contact
                    .notes
                    .as_ref()
                    .map_or(DbValue::Null, |n| DbValue::Text(n.clone())),
            ],
        )?;
        Ok(())
    }
    /// Find contacts for an agent by name or relationship.
    /// Returns all matching contacts.
    /// Find contacts matching criteria.
    ///
    /// expect: "The system provides durable storage for agent registry data"
    /// \[P3\] Motivating: Generative Space — find contacts
    /// post: returns Vec of matching contacts
    pub fn find_contacts(
        &self,
        agent_name: &str,
        query: &str,
    ) -> Result<Vec<Contact>, AgentRegistryError> {
        let pattern = format!("%{query}%");
        Ok(query_map(
            &*self.driver,
            "SELECT agent_name, contact_name, relationship, notes
             FROM contacts WHERE agent_name = ?1 AND (contact_name LIKE ?2 OR relationship LIKE ?2)",
            &[
                DbValue::Text(agent_name.to_string()),
                DbValue::Text(pattern),
            ],
            |row| {
                Ok(Contact {
                    agent_name: row.get_str(0)?.to_string(),
                    contact_name: row.get_str(1)?.to_string(),
                    relationship: match row.get(2)? { DbValue::Null => None, v => Some(v.as_text()?.to_string()) },
                    notes: match row.get(3)? { DbValue::Null => None, v => Some(v.as_text()?.to_string()) },
                })
            },
        )?)
    }
    /// List all contacts for an agent.
    /// List contacts for an agent.
    ///
    /// expect: "The system provides durable storage for agent registry data"
    /// \[P3\] Motivating: Generative Space — list contacts for agent
    /// pre:  agent_name is non-empty
    /// post: returns Vec of contacts
    pub fn list_contacts(&self, agent_name: &str) -> Result<Vec<Contact>, AgentRegistryError> {
        Ok(query_map(
            &*self.driver,
            "SELECT agent_name, contact_name, relationship, notes
             FROM contacts WHERE agent_name = ?1 ORDER BY contact_name ",
            &[DbValue::Text(agent_name.to_string())],
            |row| {
                Ok(Contact {
                    agent_name: row.get_str(0)?.to_string(),
                    contact_name: row.get_str(1)?.to_string(),
                    relationship: match row.get(2)? {
                        DbValue::Null => None,
                        v => Some(v.as_text()?.to_string()),
                    },
                    notes: match row.get(3)? {
                        DbValue::Null => None,
                        v => Some(v.as_text()?.to_string()),
                    },
                })
            },
        )?)
    }
    /// Add a scheduled task for an agent.
    /// Add a scheduled task.
    ///
    /// expect: "The system provides durable storage for agent registry data"
    /// \[P3\] Motivating: Generative Space — add scheduled task
    /// pre:  task has valid fields
    /// post: task inserted
    pub fn add_scheduled_task(&self, task: &ScheduledTask) -> Result<(), AgentRegistryError> {
        self.driver.execute(
            "INSERT INTO scheduled_tasks (agent_name, trigger_expr, action, params, next_run, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            &[
                DbValue::Text(task.agent_name.clone()),
                DbValue::Text(task.trigger.clone()),
                DbValue::Text(task.action.clone()),
                task.params.as_ref().map_or(DbValue::Null, |p| DbValue::Text(p.clone())),
                task.next_run.as_ref().map_or(DbValue::Null, |s| DbValue::Text(s.clone())),
                DbValue::Integer(if task.enabled { 1 } else { 0 }),
            ],
        )?;
        Ok(())
    }
    /// List all enabled scheduled tasks whose next_run is due (<= now).
    /// List tasks due for execution.
    ///
    /// expect: "The system provides durable storage for agent registry data"
    /// \[P3\] Motivating: Generative Space — list due tasks
    /// pre:  now is a valid timestamp
    /// post: returns Vec of due tasks
    pub fn list_due_tasks(&self, now: &str) -> Result<Vec<ScheduledTask>, AgentRegistryError> {
        Ok(query_map(
            &*self.driver,
            "SELECT agent_name, trigger_expr, action, params, next_run, enabled
             FROM scheduled_tasks WHERE enabled = 1 AND next_run <= ?1 ORDER BY next_run ",
            &[DbValue::Text(now.to_string())],
            |row| {
                Ok(ScheduledTask {
                    agent_name: row.get_str(0)?.to_string(),
                    trigger: row.get_str(1)?.to_string(),
                    action: row.get_str(2)?.to_string(),
                    params: match row.get(3)? {
                        DbValue::Null => None,
                        v => Some(v.as_text()?.to_string()),
                    },
                    next_run: Some(row.get_str(4)?.to_string()),
                    enabled: row.get_int(5)? != 0,
                })
            },
        )?)
    }
    /// List all scheduled tasks for an agent.
    /// List scheduled tasks for an agent.
    ///
    /// expect: "The system provides durable storage for agent registry data"
    /// \[P3\] Motivating: Generative Space — list tasks for agent
    /// pre:  agent_name is non-empty
    /// post: returns Vec of tasks
    pub fn list_scheduled_tasks(
        &self,
        agent_name: &str,
    ) -> Result<Vec<ScheduledTask>, AgentRegistryError> {
        Ok(query_map(
            &*self.driver,
            "SELECT agent_name, trigger_expr, action, params, next_run, enabled
             FROM scheduled_tasks WHERE agent_name = ?1 ORDER BY next_run ",
            &[DbValue::Text(agent_name.to_string())],
            |row| {
                Ok(ScheduledTask {
                    agent_name: row.get_str(0)?.to_string(),
                    trigger: row.get_str(1)?.to_string(),
                    action: row.get_str(2)?.to_string(),
                    params: match row.get(3)? {
                        DbValue::Null => None,
                        v => Some(v.as_text()?.to_string()),
                    },
                    next_run: Some(row.get_str(4)?.to_string()),
                    enabled: row.get_int(5)? != 0,
                })
            },
        )?)
    }
    /// Update the next_run time for a scheduled task (after it fires).
    /// Update the next run time for a task.
    ///
    /// expect: "The system provides durable storage for agent registry data"
    /// \[P3\] Motivating: Generative Space — update task next_run
    /// pre:  task_id is valid, next_run is valid
    /// post: next_run updated
    pub fn update_next_run(
        &self,
        agent_name: &str,
        trigger: &str,
        new_next_run: &str,
    ) -> Result<(), AgentRegistryError> {
        let updated = self.driver.execute(
            "UPDATE scheduled_tasks SET next_run = ?1 WHERE agent_name = ?2 AND trigger_expr = ?3",
            &[
                DbValue::Text(new_next_run.to_string()),
                DbValue::Text(agent_name.to_string()),
                DbValue::Text(trigger.to_string()),
            ],
        )?;
        if updated == 0 {
            return Err(AgentRegistryError::NotFound(NotFound {
                entity_type: "agent".to_string(),
                id: format!("Task {agent_name}/{trigger}"),
            }));
        }
        Ok(())
    }
}

// ── RegistryPort implementation ──────────────────────────────────────

impl hkask_ports::registry_port::RegistryPort for AgentRegistryStore {
    fn initialize_schema(&self) -> Result<(), InfrastructureError> {
        // Schema initialized by from_driver() via init_schema().
        Ok(())
    }

    fn list(&self) -> Result<Vec<RegisteredAgent>, InfrastructureError> {
        self.list()
            .map_err(|e| InfrastructureError::database(e.to_string()))
    }

    fn insert(&self, agent: &RegisteredAgent) -> Result<(), InfrastructureError> {
        self.insert(agent)
            .map_err(|e| InfrastructureError::database(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_database::sqlite::SqliteDriver;
    use hkask_ports::registry_port::RegistryPort;
    use std::sync::Arc;

    fn make_store() -> AgentRegistryStore {
        let pool = SqliteDriver::in_memory_pool().expect("in-memory SQLite pool");
        let driver = SqliteDriver::new(pool);
        let store = AgentRegistryStore::from_driver(Arc::new(driver));
        store.initialize_schema().unwrap();
        store
    }

    #[test]
    fn get_missing_agent_returns_not_found() {
        let store = make_store();
        let result = store.get("nonexistent");
        assert!(matches!(result, Err(AgentRegistryError::NotFound(_))));
    }

    #[test]
    fn remove_missing_agent_returns_not_found() {
        let store = make_store();
        let result = store.remove("nonexistent");
        assert!(matches!(result, Err(AgentRegistryError::NotFound(_))));
    }
}
