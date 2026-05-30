//! StandingSessionStore — Persistent storage for standing ensemble sessions
//!
//! Session lifecycle under master key rotation:
//! - Each session records the `key_version` under which it was created.
//! - On master key rotation, old sessions are sealed (read-only) via
//!   `seal_sessions_before_version()`. New sessions use the new key version.
//! - Sealed sessions remain readable but cannot accept new messages —
//!   they are archival, consistent with the architecture's forward-only
//!   migration policy (no automatic re-encryption).

use hkask_types::InfrastructureError;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
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

impl From<rusqlite::Error> for StandingSessionError {
    fn from(e: rusqlite::Error) -> Self {
        StandingSessionError::Infra(InfrastructureError::Database(e.to_string()))
    }
}

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

#[derive(Clone)]
pub struct StandingSessionStore {
    conn: Arc<Mutex<Connection>>,
}

impl StandingSessionStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn initialize_schema(&self) -> Result<(), StandingSessionError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
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
                template_id TEXT,
                FOREIGN KEY (session_id) REFERENCES standing_sessions(session_id)
            );
            CREATE INDEX IF NOT EXISTS idx_session_messages_session ON session_messages(session_id);",
        )?;
        Ok(())
    }

    pub fn save_session(&self, session: &StoredSession) -> Result<(), StandingSessionError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
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

    pub fn get_session(&self, session_id: &str) -> Result<StoredSession, StandingSessionError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
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

    pub fn list_sessions(&self) -> Result<Vec<StoredSession>, StandingSessionError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT session_id, config_yaml, created_at, last_active, key_version, sealed
             FROM standing_sessions ORDER BY last_active DESC",
        )?;

        let sessions = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i32>(4)? as u32,
                    row.get::<_, i32>(5)? != 0,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(|s| StoredSession {
                session_id: s.0,
                config_yaml: s.1,
                created_at: s.2,
                last_active: s.3,
                key_version: s.4,
                sealed: s.5,
            })
            .collect();

        Ok(sessions)
    }

    pub fn save_message(&self, message: &StoredMessage) -> Result<i64, StandingSessionError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;

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

    pub fn get_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredMessage>, StandingSessionError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
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
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE standing_sessions SET last_active = ?1 WHERE session_id = ?2",
            rusqlite::params![now, session_id],
        )?;
        Ok(())
    }

    /// Seal all sessions created before the given key version.
    ///
    /// Called when the master key is rotated. Sealed sessions remain
    /// readable but cannot accept new messages — they are archival.
    /// Returns the number of sessions sealed.
    pub fn seal_sessions_before_version(
        &self,
        version: u32,
    ) -> Result<usize, StandingSessionError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let count = conn.execute(
            "UPDATE standing_sessions SET sealed = 1 WHERE key_version < ?1 AND sealed = 0",
            [version as i32],
        )?;
        tracing::info!(
            target: "cns.session.lifecycle",
            sealed_count = count,
            new_key_version = version,
            "Sealed sessions from previous key version"
        );
        Ok(count)
    }

    /// Get the current key version — the highest version across all sessions.
    /// Returns 1 for a fresh database.
    pub fn current_key_version(&self) -> Result<u32, StandingSessionError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(key_version), 1) FROM standing_sessions",
                [],
                |row| row.get(0),
            )
            .unwrap_or(1);
        Ok(version as u32)
    }

    pub fn delete_session(&self, session_id: &str) -> Result<(), StandingSessionError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
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
