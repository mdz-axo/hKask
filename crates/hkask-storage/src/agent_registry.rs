//! AgentRegistryStore — Persistent storage for registered agents
use crate::Store;
use hkask_types::{
    AgentDefinition, AgentKind, Contact, InfrastructureError, RegisteredAgent, ScheduledTask,
    UserProfile,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentRegistryError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),

    #[error("Agent not found: {0}")]
    NotFound(String),
    #[error("Agent already registered: {0}")]
    AlreadyRegistered(String),
}

impl_from_rusqlite!(AgentRegistryError, Infra);

impl_from_serde_json!(AgentRegistryError, Infra);

define_store!(AgentRegistryStore);

impl AgentRegistryStore {
    /// Initialize the agent registry schema.
    ///
    /// REQ: P3-sto-agent-registry-schema
    /// [P3] Motivating: Generative Space — agent registry schema
    /// post: agents, user_profiles, contacts, scheduled_tasks tables created
    pub fn initialize_schema(&self) -> Result<(), AgentRegistryError> {
        let conn = self.lock_conn()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS agent_registry (
                name TEXT PRIMARY KEY,
                agent_kind TEXT NOT NULL,
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
        )?;
        Ok(())
    }

    /// Insert a registered agent.
    ///
    /// REQ: P3-sto-agent-registry-insert
    /// [P3] Motivating: Generative Space — insert registered agent
    /// pre:  agent.name is non-empty
    /// post: agent inserted into agents table
    pub fn insert(&self, agent: &RegisteredAgent) -> Result<(), AgentRegistryError> {
        let conn = self.lock_conn()?;
        let definition_json = serde_json::to_string(&agent.definition)?;

        conn.execute(
            "INSERT OR REPLACE INTO agent_registry (name, agent_kind, definition_json, token_hash, registered_at, source_yaml)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                agent.definition.name,
                agent.definition.agent_kind.as_str(),
                definition_json,
                agent.token_hash,
                agent.registered_at,
                agent.source_yaml,
            ],
        )?;
        Ok(())
    }

    /// Get an agent by name.
    ///
    /// REQ: P3-sto-agent-registry-get
    /// [P3] Motivating: Generative Space — get agent by name
    /// pre:  name is non-empty
    /// post: returns RegisteredAgent if found
    pub fn get(&self, name: &str) -> Result<RegisteredAgent, AgentRegistryError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT definition_json, token_hash, registered_at, source_yaml
             FROM agent_registry WHERE name = ?1",
        )?;

        let agent = stmt
            .query_row(rusqlite::params![name], |row| {
                let definition_json: String = row.get(0)?;
                let token_hash: String = row.get(1)?;
                let registered_at: String = row.get(2)?;
                let source_yaml: String = row.get(3)?;
                Ok((definition_json, token_hash, registered_at, source_yaml))
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AgentRegistryError::NotFound(name.to_string())
                }
                other => AgentRegistryError::from(other),
            })?;

        let definition: AgentDefinition = serde_json::from_str(&agent.0)?;
        Ok(RegisteredAgent {
            definition,
            token_hash: agent.1,
            registered_at: agent.2,
            source_yaml: agent.3,
        })
    }

    /// List all registered agents.
    ///
    /// REQ: P3-sto-agent-registry-list
    /// [P3] Motivating: Generative Space — list all agents
    /// post: returns Vec of all RegisteredAgent
    pub fn list(&self) -> Result<Vec<RegisteredAgent>, AgentRegistryError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT definition_json, token_hash, registered_at, source_yaml
             FROM agent_registry ORDER BY name ",
        )?;

        let agents = collect_rows!(
            stmt,
            [],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<(String, String, String, String)> {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
            |(def_json, token_hash, registered_at, source_yaml): (
                String,
                String,
                String,
                String
            )| {
                serde_json::from_str::<AgentDefinition>(&def_json).map(|definition| {
                    RegisteredAgent {
                        definition,
                        token_hash,
                        registered_at,
                        source_yaml,
                    }
                })
            }
        );

        Ok(agents)
    }

    /// List agents by kind.
    ///
    /// REQ: P3-sto-agent-registry-list-by-kind
    /// [P3] Motivating: Generative Space — list agents by kind
    /// pre:  kind is a valid AgentKind
    /// post: returns Vec of agents matching kind
    pub fn list_by_kind(
        &self,
        kind: AgentKind,
    ) -> Result<Vec<RegisteredAgent>, AgentRegistryError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT definition_json, token_hash, registered_at, source_yaml
             FROM agent_registry WHERE agent_kind = ?1 ORDER BY name ",
        )?;

        let agents = collect_rows!(
            stmt,
            rusqlite::params![kind],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<(String, String, String, String)> {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
            |(def_json, token_hash, registered_at, source_yaml): (
                String,
                String,
                String,
                String
            )| {
                serde_json::from_str::<AgentDefinition>(&def_json).map(|definition| {
                    RegisteredAgent {
                        definition,
                        token_hash,
                        registered_at,
                        source_yaml,
                    }
                })
            }
        );

        Ok(agents)
    }

    /// Remove an agent by name.
    ///
    /// REQ: P3-sto-agent-registry-remove
    /// [P3] Motivating: Generative Space — remove agent
    /// pre:  name is non-empty
    /// post: agent deleted if existed
    pub fn remove(&self, name: &str) -> Result<(), AgentRegistryError> {
        let conn = self.lock_conn()?;
        let deleted = conn.execute(
            "DELETE FROM agent_registry WHERE name = ?1",
            rusqlite::params![name],
        )?;
        if deleted == 0 {
            return Err(AgentRegistryError::NotFound(name.to_string()));
        }
        Ok(())
    }

    /// Store the human user's profile. Replaces any existing profile (single-row table).
    /// Store a user profile.
    ///
    /// REQ: P3-sto-agent-registry-profile-store
    /// [P3] Motivating: Generative Space — store user profile
    /// pre:  profile has valid fields
    /// post: profile upserted
    pub fn store_user_profile(&self, profile: &UserProfile) -> Result<(), AgentRegistryError> {
        let conn = self.lock_conn()?;
        let json = serde_json::to_string(profile)?;
        conn.execute(
            "INSERT OR REPLACE INTO user_profile (id, profile_json) VALUES (1, ?1)",
            rusqlite::params![json],
        )?;
        Ok(())
    }

    /// Retrieve the human user's profile. Returns None if no profile has been stored.
    /// Get the user profile.
    ///
    /// REQ: P3-sto-agent-registry-profile-get
    /// [P3] Motivating: Generative Space — get user profile
    /// post: returns Some(profile) if exists, None otherwise
    pub fn get_user_profile(&self) -> Result<Option<UserProfile>, AgentRegistryError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT profile_json FROM user_profile WHERE id = 1")?;
        let result: Result<String, _> = stmt.query_row([], |row| row.get(0));
        match result {
            Ok(json) => Ok(Some(serde_json::from_str(&json)?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AgentRegistryError::from(e)),
        }
    }

    /// Add a contact to an agent's contact registry.
    /// Add a contact.
    ///
    /// REQ: P3-sto-agent-registry-contact-add
    /// [P3] Motivating: Generative Space — add contact
    /// pre:  contact has valid fields
    /// post: contact inserted
    pub fn add_contact(&self, contact: &Contact) -> Result<(), AgentRegistryError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO contacts (agent_name, contact_name, relationship, notes)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                contact.agent_name,
                contact.contact_name,
                contact.relationship,
                contact.notes,
            ],
        )?;
        Ok(())
    }

    /// Find contacts for an agent by name or relationship.
    /// Returns all matching contacts.
    /// Find contacts matching criteria.
    ///
    /// REQ: P3-sto-agent-registry-contact-find
    /// [P3] Motivating: Generative Space — find contacts
    /// post: returns Vec of matching contacts
    pub fn find_contacts(
        &self,
        agent_name: &str,
        query: &str,
    ) -> Result<Vec<Contact>, AgentRegistryError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT agent_name, contact_name, relationship, notes
             FROM contacts WHERE agent_name = ?1 AND (contact_name LIKE ?2 OR relationship LIKE ?2)",
        )?;
        let pattern = format!("%{query}%");
        let rows = stmt.query_map(rusqlite::params![agent_name, pattern], |row| {
            Ok(Contact {
                agent_name: row.get(0)?,
                contact_name: row.get(1)?,
                relationship: row.get(2)?,
                notes: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AgentRegistryError::from)
    }

    /// List all contacts for an agent.
    /// List contacts for an agent.
    ///
    /// REQ: P3-sto-agent-registry-contact-list
    /// [P3] Motivating: Generative Space — list contacts for agent
    /// pre:  agent_name is non-empty
    /// post: returns Vec of contacts
    pub fn list_contacts(&self, agent_name: &str) -> Result<Vec<Contact>, AgentRegistryError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT agent_name, contact_name, relationship, notes
             FROM contacts WHERE agent_name = ?1 ORDER BY contact_name ",
        )?;
        let rows = stmt.query_map(rusqlite::params![agent_name], |row| {
            Ok(Contact {
                agent_name: row.get(0)?,
                contact_name: row.get(1)?,
                relationship: row.get(2)?,
                notes: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AgentRegistryError::from)
    }

    /// Add a scheduled task for an agent.
    /// Add a scheduled task.
    ///
    /// REQ: P3-sto-agent-registry-task-add
    /// [P3] Motivating: Generative Space — add scheduled task
    /// pre:  task has valid fields
    /// post: task inserted
    pub fn add_scheduled_task(&self, task: &ScheduledTask) -> Result<(), AgentRegistryError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO scheduled_tasks (agent_name, trigger_expr, action, params, next_run, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                task.agent_name,
                task.trigger,
                task.action,
                task.params,
                task.next_run,
                task.enabled as i32,
            ],
        )?;
        Ok(())
    }

    /// List all enabled scheduled tasks whose next_run is due (<= now).
    /// List tasks due for execution.
    ///
    /// REQ: P3-sto-agent-registry-task-list-due
    /// [P3] Motivating: Generative Space — list due tasks
    /// pre:  now is a valid timestamp
    /// post: returns Vec of due tasks
    pub fn list_due_tasks(&self, now: &str) -> Result<Vec<ScheduledTask>, AgentRegistryError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT agent_name, trigger_expr, action, params, next_run, enabled
             FROM scheduled_tasks WHERE enabled = 1 AND next_run <= ?1 ORDER BY next_run ",
        )?;
        let rows = stmt.query_map(rusqlite::params![now], |row| {
            Ok(ScheduledTask {
                agent_name: row.get(0)?,
                trigger: row.get(1)?,
                action: row.get(2)?,
                params: row.get(3)?,
                next_run: row.get(4)?,
                enabled: row.get::<_, i32>(5)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AgentRegistryError::from)
    }

    /// List all scheduled tasks for an agent.
    /// List scheduled tasks for an agent.
    ///
    /// REQ: P3-sto-agent-registry-task-list-agent
    /// [P3] Motivating: Generative Space — list tasks for agent
    /// pre:  agent_name is non-empty
    /// post: returns Vec of tasks
    pub fn list_scheduled_tasks(
        &self,
        agent_name: &str,
    ) -> Result<Vec<ScheduledTask>, AgentRegistryError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT agent_name, trigger_expr, action, params, next_run, enabled
             FROM scheduled_tasks WHERE agent_name = ?1 ORDER BY next_run ",
        )?;
        let rows = stmt.query_map(rusqlite::params![agent_name], |row| {
            Ok(ScheduledTask {
                agent_name: row.get(0)?,
                trigger: row.get(1)?,
                action: row.get(2)?,
                params: row.get(3)?,
                next_run: row.get(4)?,
                enabled: row.get::<_, i32>(5)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AgentRegistryError::from)
    }

    /// Update the next_run time for a scheduled task (after it fires).
    /// Update the next run time for a task.
    ///
    /// REQ: P3-sto-agent-registry-task-update
    /// [P3] Motivating: Generative Space — update task next_run
    /// pre:  task_id is valid, next_run is valid
    /// post: next_run updated
    pub fn update_next_run(
        &self,
        agent_name: &str,
        trigger: &str,
        new_next_run: &str,
    ) -> Result<(), AgentRegistryError> {
        let conn = self.lock_conn()?;
        let updated = conn.execute(
            "UPDATE scheduled_tasks SET next_run = ?1 WHERE agent_name = ?2 AND trigger_expr = ?3",
            rusqlite::params![new_next_run, agent_name, trigger],
        )?;
        if updated == 0 {
            return Err(AgentRegistryError::NotFound(format!(
                "Task {agent_name}/{trigger}"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    fn make_store() -> AgentRegistryStore {
        let conn = Arc::new(Mutex::new(
            Connection::open_in_memory().expect("in-memory DB "),
        ));
        let store = AgentRegistryStore::new(conn);
        store.initialize_schema().expect("init schema ");
        store
    }

    // REQ: P3-sto-agent-registry-notfound-get-test — get on missing name returns NotFound
    #[test]
    fn get_missing_agent_returns_not_found() {
        let store = make_store();
        let result = store.get("no-such-agent ");
        assert!(matches!(result, Err(AgentRegistryError::NotFound(_))));
    }

    // REQ: P3-sto-agent-registry-notfound-remove-test — remove on missing name returns NotFound
    #[test]
    fn remove_missing_agent_returns_not_found() {
        let store = make_store();
        let result = store.remove("no-such-agent ");
        assert!(matches!(result, Err(AgentRegistryError::NotFound(_))));
    }
}
