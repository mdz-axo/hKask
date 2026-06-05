//! AgentRegistryStore — Persistent storage for registered agents
use crate::Store;
use hkask_types::ports::{AgentRegistrationPort, RegistryError};
use hkask_types::{AgentDefinition, AgentKind, InfrastructureError, RegisteredAgent};
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

impl From<serde_json::Error> for AgentRegistryError {
    fn from(e: serde_json::Error) -> Self {
        InfrastructureError::from(e).into()
    }
}

define_store!(AgentRegistryStore);

impl AgentRegistryStore {
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
            CREATE INDEX IF NOT EXISTS idx_agent_registry_kind ON agent_registry(agent_kind);",
        )?;
        Ok(())
    }

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
            .map_err(|_| AgentRegistryError::NotFound(name.to_string()))?;

        let definition: AgentDefinition = serde_json::from_str(&agent.0)?;
        Ok(RegisteredAgent {
            definition,
            token_hash: agent.1,
            registered_at: agent.2,
            source_yaml: agent.3,
        })
    }

    pub fn list(&self) -> Result<Vec<RegisteredAgent>, AgentRegistryError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT definition_json, token_hash, registered_at, source_yaml
             FROM agent_registry ORDER BY name",
        )?;

        let mapped: Vec<_> = stmt
            .query_map([], |row| {
                let definition_json: String = row.get(0)?;
                let token_hash: String = row.get(1)?;
                let registered_at: String = row.get(2)?;
                let source_yaml: String = row.get(3)?;
                Ok((definition_json, token_hash, registered_at, source_yaml))
            })?
            .collect();

        let mut agents = Vec::with_capacity(mapped.len());
        for row_result in mapped {
            match row_result {
                Ok((def_json, token_hash, registered_at, source_yaml)) => {
                    match serde_json::from_str::<AgentDefinition>(&def_json) {
                        Ok(definition) => agents.push(RegisteredAgent {
                            definition,
                            token_hash,
                            registered_at,
                            source_yaml,
                        }),
                        Err(e) => tracing::warn!(
                            target: "hkask.storage",
                            error = %e,
                            "Skipping agent with unparseable definition JSON"
                        ),
                    }
                }
                Err(e) => {
                    tracing::warn!(target: "hkask.storage", error = %e, "Skipping unreadable database row")
                }
            }
        }

        Ok(agents)
    }

    pub fn list_by_kind(
        &self,
        kind: AgentKind,
    ) -> Result<Vec<RegisteredAgent>, AgentRegistryError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT definition_json, token_hash, registered_at, source_yaml
             FROM agent_registry WHERE agent_kind = ?1 ORDER BY name",
        )?;

        let mapped: Vec<_> = stmt
            .query_map(rusqlite::params![kind.as_str()], |row| {
                let definition_json: String = row.get(0)?;
                let token_hash: String = row.get(1)?;
                let registered_at: String = row.get(2)?;
                let source_yaml: String = row.get(3)?;
                Ok((definition_json, token_hash, registered_at, source_yaml))
            })?
            .collect();

        let mut agents = Vec::with_capacity(mapped.len());
        for row_result in mapped {
            match row_result {
                Ok((def_json, token_hash, registered_at, source_yaml)) => {
                    match serde_json::from_str::<AgentDefinition>(&def_json) {
                        Ok(definition) => agents.push(RegisteredAgent {
                            definition,
                            token_hash,
                            registered_at,
                            source_yaml,
                        }),
                        Err(e) => tracing::warn!(
                            target: "hkask.storage",
                            error = %e,
                            "Skipping agent with unparseable definition JSON"
                        ),
                    }
                }
                Err(e) => {
                    tracing::warn!(target: "hkask.storage", error = %e, "Skipping unreadable database row")
                }
            }
        }

        Ok(agents)
    }

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
}

impl AgentRegistrationPort for AgentRegistryStore {
    fn register_agent(&self, agent: RegisteredAgent) -> Result<(), RegistryError> {
        AgentRegistryStore::insert(self, &agent).map_err(|e| match e {
            AgentRegistryError::NotFound(s) => RegistryError::NotFound(s),
            other => RegistryError::Other(other.to_string()),
        })
    }

    fn get_agent(&self, name: &str) -> Result<RegisteredAgent, RegistryError> {
        AgentRegistryStore::get(self, name).map_err(|e| match e {
            AgentRegistryError::NotFound(s) => RegistryError::NotFound(s),
            other => RegistryError::Other(other.to_string()),
        })
    }

    fn list_agents(&self) -> Result<Vec<RegisteredAgent>, RegistryError> {
        AgentRegistryStore::list(self).map_err(|e| RegistryError::Other(e.to_string()))
    }

    fn list_agents_by_kind(&self, kind: AgentKind) -> Result<Vec<RegisteredAgent>, RegistryError> {
        AgentRegistryStore::list_by_kind(self, kind)
            .map_err(|e| RegistryError::Other(e.to_string()))
    }

    fn remove_agent(&self, name: &str) -> Result<(), RegistryError> {
        AgentRegistryStore::remove(self, name).map_err(|e| match e {
            AgentRegistryError::NotFound(s) => RegistryError::NotFound(s),
            other => RegistryError::Other(other.to_string()),
        })
    }
}
