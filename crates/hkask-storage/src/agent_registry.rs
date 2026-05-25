//! AgentRegistryStore — Persistent storage for registered agents

use hkask_types::{AgentDefinition, AgentKind, RegisteredAgent};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentRegistryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Agent not found: {0}")]
    NotFound(String),
    #[error("Agent already registered: {0}")]
    AlreadyRegistered(String),
}

#[derive(Clone)]
pub struct AgentRegistryStore {
    conn: Arc<Mutex<Connection>>,
}

impl AgentRegistryStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn initialize_schema(&self) -> Result<(), AgentRegistryError> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT definition_json, token_hash, registered_at, source_yaml
             FROM agent_registry ORDER BY name",
        )?;

        let agents = stmt
            .query_map([], |row| {
                let definition_json: String = row.get(0)?;
                let token_hash: String = row.get(1)?;
                let registered_at: String = row.get(2)?;
                let source_yaml: String = row.get(3)?;
                Ok((definition_json, token_hash, registered_at, source_yaml))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(def_json, token_hash, registered_at, source_yaml)| {
                let definition: AgentDefinition = serde_json::from_str(&def_json).ok()?;
                Some(RegisteredAgent {
                    definition,
                    token_hash,
                    registered_at,
                    source_yaml,
                })
            })
            .collect();

        Ok(agents)
    }

    pub fn list_by_kind(
        &self,
        kind: AgentKind,
    ) -> Result<Vec<RegisteredAgent>, AgentRegistryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT definition_json, token_hash, registered_at, source_yaml
             FROM agent_registry WHERE agent_kind = ?1 ORDER BY name",
        )?;

        let agents = stmt
            .query_map(rusqlite::params![kind.as_str()], |row| {
                let definition_json: String = row.get(0)?;
                let token_hash: String = row.get(1)?;
                let registered_at: String = row.get(2)?;
                let source_yaml: String = row.get(3)?;
                Ok((definition_json, token_hash, registered_at, source_yaml))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(def_json, token_hash, registered_at, source_yaml)| {
                let definition: AgentDefinition = serde_json::from_str(&def_json).ok()?;
                Some(RegisteredAgent {
                    definition,
                    token_hash,
                    registered_at,
                    source_yaml,
                })
            })
            .collect();

        Ok(agents)
    }

    pub fn exists(&self, name: &str) -> Result<bool, AgentRegistryError> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM agent_registry WHERE name = ?1",
            rusqlite::params![name],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn remove(&self, name: &str) -> Result<(), AgentRegistryError> {
        let conn = self.conn.lock().unwrap();
        let deleted = conn.execute(
            "DELETE FROM agent_registry WHERE name = ?1",
            rusqlite::params![name],
        )?;
        if deleted == 0 {
            return Err(AgentRegistryError::NotFound(name.to_string()));
        }
        Ok(())
    }

    pub fn count(&self) -> Result<usize, AgentRegistryError> {
        let conn = self.conn.lock().unwrap();
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM agent_registry", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use hkask_types::Charter;

    fn test_store() -> AgentRegistryStore {
        let conn = Connection::open_in_memory().unwrap();
        let store = AgentRegistryStore::new(Arc::new(Mutex::new(conn)));
        store.initialize_schema().unwrap();
        store
    }

    fn test_agent(name: &str, kind: AgentKind) -> RegisteredAgent {
        RegisteredAgent {
            definition: AgentDefinition {
                name: name.to_string(),
                agent_kind: kind,
                binding_contract: true,
                editor: "admin".to_string(),
                charter: Some(Charter {
                    description: format!("{} charter", name),
                    archetype: "Test".to_string(),
                    visibility: "public".to_string(),
                }),
                capabilities: vec!["tool:cns:emit".to_string()],
                rights: vec![],
                responsibilities: vec![],
                reporting: None,
                standing_session: None,
                persona: None,
                depends_on: vec![],
                readiness_probe: None,
                process_manifest: None,
            },
            token_hash: "test-hash".to_string(),
            registered_at: Utc::now().to_rfc3339(),
            source_yaml: format!("registry/bots/{}.yaml", name),
        }
    }

    #[test]
    fn test_insert_and_get() {
        let store = test_store();
        let agent = test_agent("test-bot", AgentKind::Bot);
        store.insert(&agent).unwrap();

        let retrieved = store.get("test-bot").unwrap();
        assert_eq!(retrieved.definition.name, "test-bot");
        assert_eq!(retrieved.definition.agent_kind, AgentKind::Bot);
    }

    #[test]
    fn test_list() {
        let store = test_store();
        store.insert(&test_agent("bot-a", AgentKind::Bot)).unwrap();
        store.insert(&test_agent("bot-b", AgentKind::Bot)).unwrap();
        store
            .insert(&test_agent("curator", AgentKind::Replicant))
            .unwrap();

        let all = store.list().unwrap();
        assert_eq!(all.len(), 3);

        let bots = store.list_by_kind(AgentKind::Bot).unwrap();
        assert_eq!(bots.len(), 2);

        let replicants = store.list_by_kind(AgentKind::Replicant).unwrap();
        assert_eq!(replicants.len(), 1);
    }

    #[test]
    fn test_exists_and_remove() {
        let store = test_store();
        let agent = test_agent("removable", AgentKind::Bot);
        store.insert(&agent).unwrap();

        assert!(store.exists("removable").unwrap());
        store.remove("removable").unwrap();
        assert!(!store.exists("removable").unwrap());
    }

    #[test]
    fn test_count() {
        let store = test_store();
        assert_eq!(store.count().unwrap(), 0);
        store.insert(&test_agent("a", AgentKind::Bot)).unwrap();
        store.insert(&test_agent("b", AgentKind::Bot)).unwrap();
        assert_eq!(store.count().unwrap(), 2);
    }

    #[test]
    fn test_upsert() {
        let store = test_store();
        let mut agent = test_agent("upsert-bot", AgentKind::Bot);
        store.insert(&agent).unwrap();

        agent
            .definition
            .capabilities
            .push("tool:memory:recall".to_string());
        store.insert(&agent).unwrap();

        let retrieved = store.get("upsert-bot").unwrap();
        assert_eq!(retrieved.definition.capabilities.len(), 2);
        assert_eq!(store.count().unwrap(), 1);
    }
}
