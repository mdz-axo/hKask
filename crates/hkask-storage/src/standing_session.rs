//! StandingSessionStore — Persistent storage for standing ensemble sessions
//!
//! Session lifecycle under master key rotation:
//! - Each session records the `key_version` under which it was created.
//! - On master key rotation, old sessions are sealed (read-only).
//! - Sealed sessions remain readable but cannot accept new messages —
//!   they are archival, consistent with the architecture's forward-only
//!   migration policy (no automatic re-encryption).

use crate::{Store, now_rfc3339};
use hkask_types::InfrastructureError;
use hkask_types::ports::git_cas::RepoId;
use hkask_types::ports::{MessageRecord, SessionRecord, SessionStoreError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StandingSessionError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),

    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Session is sealed (key version mismatch): {0}")]
    Sealed(String),
}

impl_from_rusqlite!(StandingSessionError, Infra);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSession {
    pub session_id: String,
    pub config_yaml: String,
    pub created_at: String,
    pub last_active: String,
    /// Key derivation version at session creation.
    /// Incremented when the master key is rotated.
    pub key_version: u32,
    /// `true` when the session's key version predates the current derivation
    /// context — sealed sessions are read-only archives.
    pub sealed: bool,
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

define_store_cas!(StandingSessionStore);

impl StandingSessionStore {
    /// Initialize the standing session tables.
    pub fn initialize_schema(&self) -> Result<(), StandingSessionError> {
        let conn = self.lock_conn()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS standing_sessions (
                session_id TEXT PRIMARY KEY,
                config_yaml TEXT NOT NULL,
                created_at TEXT NOT NULL,
                last_active TEXT NOT NULL,
                key_version INTEGER NOT NULL DEFAULT 1,
                sealed INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS session_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                from_webid TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                template_id TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_session_messages_session ON session_messages(session_id);",
        )?;
        Ok(())
    }

    pub fn save_stored_session(&self, session: &StoredSession) -> Result<(), StandingSessionError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO standing_sessions (session_id, config_yaml, created_at, last_active, key_version, sealed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                session.session_id,
                session.config_yaml,
                session.created_at,
                session.last_active,
                session.key_version as i32,
                session.sealed as i32,
            ],
        )?;
        Ok(())
    }

    /// Save session with CAS write-through: persists to SQLite, then writes to the Sessions repo.
    pub async fn save_stored_session_with_cas(
        &self,
        session: &StoredSession,
    ) -> Result<(), StandingSessionError> {
        self.save_stored_session(session)?;
        if let Some(port) = &self.cas_port {
            let bytes = serde_json::to_vec(session).map_err(|e| {
                StandingSessionError::Infra(InfrastructureError::Serialization(e.to_string()))
            })?;
            port.put_blob(&RepoId::Sessions, &bytes)
                .await
                .map_err(|e| StandingSessionError::Infra(InfrastructureError::Io(e.to_string())))?;
        }
        Ok(())
    }

    pub fn get_stored_session(
        &self,
        session_id: &str,
    ) -> Result<StoredSession, StandingSessionError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT session_id, config_yaml, created_at, last_active, key_version, sealed
             FROM standing_sessions WHERE session_id = ?1",
        )?;

        let session = stmt
            .query_row(rusqlite::params![session_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i32>(4)? as u32,
                    row.get::<_, i32>(5)? != 0,
                ))
            })
            .map_err(|_| StandingSessionError::NotFound(session_id.to_string()))?;

        Ok(StoredSession {
            session_id: session.0,
            config_yaml: session.1,
            created_at: session.2,
            last_active: session.3,
            key_version: session.4,
            sealed: session.5,
        })
    }

    pub fn save_stored_message(
        &self,
        message: &StoredMessage,
    ) -> Result<i64, StandingSessionError> {
        let conn = self.lock_conn()?;

        // Check if the session is sealed before writing.
        let sealed: bool = conn
            .query_row(
                "SELECT sealed FROM standing_sessions WHERE session_id = ?1",
                [&message.session_id],
                |row| row.get::<_, i32>(0).map(|s| s != 0),
            )
            .map_err(|_| StandingSessionError::NotFound(message.session_id.clone()))?;
        if sealed {
            return Err(StandingSessionError::Sealed(message.session_id.clone()));
        }

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

    pub fn get_stored_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredMessage>, StandingSessionError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, session_id, from_webid, content, timestamp, template_id
             FROM session_messages WHERE session_id = ?1 ORDER BY id ASC",
        )?;

        let messages = collect_rows!(
            stmt,
            rusqlite::params![session_id],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<StoredMessage> {
                Ok(StoredMessage {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    from_webid: row.get(2)?,
                    content: row.get(3)?,
                    timestamp: row.get(4)?,
                    template_id: row.get(5)?,
                })
            }
        );

        Ok(messages)
    }

    pub fn update_stored_last_active(&self, session_id: &str) -> Result<(), StandingSessionError> {
        let conn = self.lock_conn()?;
        let now = now_rfc3339();
        conn.execute(
            "UPDATE standing_sessions SET last_active = ?1 WHERE session_id = ?2",
            rusqlite::params![now, session_id],
        )?;
        Ok(())
    }

    /// Get the current key version — the highest version across all sessions.
    /// Returns 1 for a fresh database.
    pub fn current_key_version(&self) -> Result<u32, StandingSessionError> {
        let conn = self.lock_conn()?;
        let version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(key_version), 1) FROM standing_sessions",
                [],
                |row| row.get(0),
            )
            .unwrap_or(1);
        Ok(version as u32)
    }
}

// Public API methods (SessionRecord/MessageRecord types)

impl StandingSessionStore {
    pub fn save_session(&self, session: &SessionRecord) -> Result<(), SessionStoreError> {
        let stored = StoredSession {
            session_id: session.session_id.clone(),
            config_yaml: session.config_yaml.clone(),
            created_at: session.created_at.clone(),
            last_active: session.last_active.clone(),
            key_version: 1, // default for port-level saves
            sealed: false,
        };
        self.save_stored_session(&stored).map_err(|e| match e {
            StandingSessionError::NotFound(s) => SessionStoreError::NotFound(s),
            StandingSessionError::Sealed(s) => SessionStoreError::Sealed(s),
            StandingSessionError::Infra(ie) => SessionStoreError::Storage(ie.to_string()),
        })
    }

    pub fn get_session(&self, session_id: &str) -> Result<SessionRecord, SessionStoreError> {
        let stored = self.get_stored_session(session_id).map_err(|e| match e {
            StandingSessionError::NotFound(s) => SessionStoreError::NotFound(s),
            StandingSessionError::Sealed(s) => SessionStoreError::Sealed(s),
            StandingSessionError::Infra(ie) => SessionStoreError::Storage(ie.to_string()),
        })?;
        Ok(SessionRecord {
            session_id: stored.session_id,
            config_yaml: stored.config_yaml,
            created_at: stored.created_at,
            last_active: stored.last_active,
        })
    }

    pub fn save_message(&self, message: &MessageRecord) -> Result<i64, SessionStoreError> {
        let stored = StoredMessage {
            id: message.id,
            session_id: message.session_id.clone(),
            from_webid: message.from_webid.clone(),
            content: message.content.clone(),
            timestamp: message.timestamp.clone(),
            template_id: message.template_id.clone(),
        };
        self.save_stored_message(&stored).map_err(|e| match e {
            StandingSessionError::NotFound(s) => SessionStoreError::NotFound(s),
            StandingSessionError::Sealed(s) => SessionStoreError::Sealed(s),
            StandingSessionError::Infra(ie) => SessionStoreError::Storage(ie.to_string()),
        })
    }

    pub fn get_messages(&self, session_id: &str) -> Result<Vec<MessageRecord>, SessionStoreError> {
        let stored = self.get_stored_messages(session_id).map_err(|e| match e {
            StandingSessionError::NotFound(s) => SessionStoreError::NotFound(s),
            StandingSessionError::Sealed(s) => SessionStoreError::Sealed(s),
            StandingSessionError::Infra(ie) => SessionStoreError::Storage(ie.to_string()),
        })?;
        Ok(stored
            .into_iter()
            .map(|s| MessageRecord {
                id: s.id,
                session_id: s.session_id,
                from_webid: s.from_webid,
                content: s.content,
                timestamp: s.timestamp,
                template_id: s.template_id,
            })
            .collect())
    }

    pub fn update_last_active(&self, session_id: &str) -> Result<(), SessionStoreError> {
        self.update_stored_last_active(session_id)
            .map_err(|e| match e {
                StandingSessionError::NotFound(s) => SessionStoreError::NotFound(s),
                StandingSessionError::Sealed(s) => SessionStoreError::Sealed(s),
                StandingSessionError::Infra(ie) => SessionStoreError::Storage(ie.to_string()),
            })
    }
}

#[cfg(test)]
mod cas_tests {
    use super::*;
    use hkask_types::ports::git_cas::MockGitCas;
    use std::sync::Arc;

    /// Tracer bullet: save_stored_session_with_cas writes to SQLite and CAS Sessions repo.
    #[tokio::test]
    async fn save_stored_session_with_cas_writes_to_sessions_repo() {
        let db = crate::Database::in_memory().expect("in-memory db");
        let mock = Arc::new(MockGitCas::new());
        let store = StandingSessionStore::new(db.conn_arc()).with_cas(mock.clone());
        store.initialize_schema().expect("schema");

        let session = StoredSession {
            session_id: "sess-cas-1".to_string(),
            config_yaml: "agent: test".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            last_active: "2025-01-01T00:00:00Z".to_string(),
            key_version: 1,
            sealed: false,
        };
        store
            .save_stored_session_with_cas(&session)
            .await
            .expect("save_with_cas");

        let retrieved = store.get_stored_session("sess-cas-1").expect("get");
        assert_eq!(retrieved.session_id, "sess-cas-1");
    }

    /// Tracer bullet: save_stored_session_with_cas without CAS port still persists to SQLite.
    #[tokio::test]
    async fn save_stored_session_with_cas_without_cas_port_persists_sqlite() {
        let db = crate::Database::in_memory().expect("in-memory db");
        let store = StandingSessionStore::new(db.conn_arc());
        store.initialize_schema().expect("schema");

        let session = StoredSession {
            session_id: "sess-no-cas".to_string(),
            config_yaml: "agent: test".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            last_active: "2025-01-01T00:00:00Z".to_string(),
            key_version: 1,
            sealed: false,
        };
        store
            .save_stored_session_with_cas(&session)
            .await
            .expect("save_with_cas");

        let retrieved = store.get_stored_session("sess-no-cas").expect("get");
        assert_eq!(retrieved.session_id, "sess-no-cas");
    }
}
