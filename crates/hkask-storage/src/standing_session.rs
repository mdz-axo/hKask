//! StandingSessionStore — Persistent storage for standing ensemble sessions

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StandingSessionError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Session not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSession {
    pub session_id: String,
    pub config_yaml: String,
    pub created_at: String,
    pub last_active: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    pub id: i64,
    pub session_id: String,
    pub from_webid: String,
    pub content: String,
    pub timestamp: String,
    pub template_id: Option<String>,
}

#[derive(Clone)]
pub struct StandingSessionStore {
    conn: Arc<Mutex<Connection>>,
}

impl StandingSessionStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn initialize_schema(&self) -> Result<(), StandingSessionError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS standing_sessions (
                session_id TEXT PRIMARY KEY,
                config_yaml TEXT NOT NULL,
                created_at TEXT NOT NULL,
                last_active TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS session_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                from_webid TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                template_id TEXT,
                FOREIGN KEY (session_id) REFERENCES standing_sessions(session_id)
            );
            CREATE INDEX IF NOT EXISTS idx_session_messages_session ON session_messages(session_id);",
        )?;
        Ok(())
    }

    pub fn save_session(&self, session: &StoredSession) -> Result<(), StandingSessionError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO standing_sessions (session_id, config_yaml, created_at, last_active)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                session.session_id,
                session.config_yaml,
                session.created_at,
                session.last_active,
            ],
        )?;
        Ok(())
    }

    pub fn get_session(&self, session_id: &str) -> Result<StoredSession, StandingSessionError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT session_id, config_yaml, created_at, last_active
             FROM standing_sessions WHERE session_id = ?1",
        )?;

        let session = stmt
            .query_row(rusqlite::params![session_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|_| StandingSessionError::NotFound(session_id.to_string()))?;

        Ok(StoredSession {
            session_id: session.0,
            config_yaml: session.1,
            created_at: session.2,
            last_active: session.3,
        })
    }

    pub fn list_sessions(&self) -> Result<Vec<StoredSession>, StandingSessionError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT session_id, config_yaml, created_at, last_active
             FROM standing_sessions ORDER BY last_active DESC",
        )?;

        let sessions = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(|s| StoredSession {
                session_id: s.0,
                config_yaml: s.1,
                created_at: s.2,
                last_active: s.3,
            })
            .collect();

        Ok(sessions)
    }

    pub fn save_message(&self, message: &StoredMessage) -> Result<i64, StandingSessionError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO session_messages (session_id, from_webid, content, timestamp, template_id)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                message.session_id,
                message.from_webid,
                message.content,
                message.timestamp,
                message.template_id,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredMessage>, StandingSessionError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, from_webid, content, timestamp, template_id
             FROM session_messages WHERE session_id = ?1 ORDER BY id ASC",
        )?;

        let messages = stmt
            .query_map(rusqlite::params![session_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(|m| StoredMessage {
                id: m.0,
                session_id: m.1,
                from_webid: m.2,
                content: m.3,
                timestamp: m.4,
                template_id: m.5,
            })
            .collect();

        Ok(messages)
    }

    pub fn update_last_active(&self, session_id: &str) -> Result<(), StandingSessionError> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE standing_sessions SET last_active = ?1 WHERE session_id = ?2",
            rusqlite::params![now, session_id],
        )?;
        Ok(())
    }

    pub fn delete_session(&self, session_id: &str) -> Result<(), StandingSessionError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM session_messages WHERE session_id = ?1",
            rusqlite::params![session_id],
        )?;
        conn.execute(
            "DELETE FROM standing_sessions WHERE session_id = ?1",
            rusqlite::params![session_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> StandingSessionStore {
        let conn = Connection::open_in_memory().unwrap();
        let store = StandingSessionStore::new(Arc::new(Mutex::new(conn)));
        store.initialize_schema().unwrap();
        store
    }

    #[test]
    fn test_save_and_get_session() {
        let store = test_store();
        let session = StoredSession {
            session_id: "test-session".to_string(),
            config_yaml: "test: yaml".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            last_active: "2026-01-01T00:00:00Z".to_string(),
        };

        store.save_session(&session).unwrap();
        let retrieved = store.get_session("test-session").unwrap();

        assert_eq!(retrieved.session_id, "test-session");
        assert_eq!(retrieved.config_yaml, "test: yaml");
    }

    #[test]
    fn test_save_and_get_messages() {
        let store = test_store();
        let session = StoredSession {
            session_id: "test-session".to_string(),
            config_yaml: "test: yaml".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            last_active: "2026-01-01T00:00:00Z".to_string(),
        };
        store.save_session(&session).unwrap();

        let msg1 = StoredMessage {
            id: 0,
            session_id: "test-session".to_string(),
            from_webid: "webid:curator".to_string(),
            content: "Hello".to_string(),
            timestamp: "2026-01-01T00:00:01Z".to_string(),
            template_id: None,
        };
        let msg2 = StoredMessage {
            id: 0,
            session_id: "test-session".to_string(),
            from_webid: "webid:bot1".to_string(),
            content: "Hi".to_string(),
            timestamp: "2026-01-01T00:00:02Z".to_string(),
            template_id: Some("template:test".to_string()),
        };

        store.save_message(&msg1).unwrap();
        store.save_message(&msg2).unwrap();

        let messages = store.get_messages("test-session").unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].content, "Hi");
        assert!(messages[1].template_id.is_some());
    }

    #[test]
    fn test_list_sessions() {
        let store = test_store();
        let session1 = StoredSession {
            session_id: "session-1".to_string(),
            config_yaml: "test: 1".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            last_active: "2026-01-01T00:00:00Z".to_string(),
        };
        let session2 = StoredSession {
            session_id: "session-2".to_string(),
            config_yaml: "test: 2".to_string(),
            created_at: "2026-01-02T00:00:00Z".to_string(),
            last_active: "2026-01-02T00:00:00Z".to_string(),
        };

        store.save_session(&session1).unwrap();
        store.save_session(&session2).unwrap();

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_delete_session() {
        let store = test_store();
        let session = StoredSession {
            session_id: "test-session".to_string(),
            config_yaml: "test: yaml".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            last_active: "2026-01-01T00:00:00Z".to_string(),
        };
        store.save_session(&session).unwrap();

        let msg = StoredMessage {
            id: 0,
            session_id: "test-session".to_string(),
            from_webid: "webid:curator".to_string(),
            content: "Hello".to_string(),
            timestamp: "2026-01-01T00:00:01Z".to_string(),
            template_id: None,
        };
        store.save_message(&msg).unwrap();

        store.delete_session("test-session").unwrap();

        assert!(store.get_session("test-session").is_err());
        assert!(store.get_messages("test-session").unwrap().is_empty());
    }
}
