//! AgentRegistryStore — Persistent storage for registered agents
use crate::Store;
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

impl_from_serde_json!(AgentRegistryError, Infra);

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

    pub fn list(&self) -> Result<Vec<RegisteredAgent>, AgentRegistryError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT definition_json, token_hash, registered_at, source_yaml
             FROM agent_registry ORDER BY name",
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

    pub fn list_by_kind(
        &self,
        kind: AgentKind,
    ) -> Result<Vec<RegisteredAgent>, AgentRegistryError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT definition_json, token_hash, registered_at, source_yaml
             FROM agent_registry WHERE agent_kind = ?1 ORDER BY name",
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    fn make_store() -> AgentRegistryStore {
        let conn = Arc::new(Mutex::new(
            Connection::open_in_memory().expect("in-memory DB"),
        ));
        let store = AgentRegistryStore::new(conn);
        store.initialize_schema().expect("init schema");
        store
    }

    // REQ: agent-registry-notfound-001 — get on missing name returns NotFound
    //
    // Before fix, any rusqlite error was mapped to NotFound. Now only
    // QueryReturnedNoRows maps to NotFound; other errors map to Infra.
    #[test]
    fn get_missing_agent_returns_not_found() {
        let store = make_store();
        let result = store.get("no-such-agent");
        assert!(
            matches!(result, Err(AgentRegistryError::NotFound(_))),
            "expected NotFound, got {:?}",
            result
        );
    }

    // REQ: agent-registry-notfound-002 — remove on missing name returns NotFound
    #[test]
    fn remove_missing_agent_returns_not_found() {
        let store = make_store();
        let result = store.remove("no-such-agent");
        assert!(
            matches!(result, Err(AgentRegistryError::NotFound(_))),
            "expected NotFound, got {:?}",
            result
        );
    }
}
