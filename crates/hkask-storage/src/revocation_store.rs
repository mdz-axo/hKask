//! Revocation Store — SQLite persistence for capability revocations
//!
//! Persists revoked capability/delegation IDs so revocation survives process restarts.
//! Per ADR-022 §T12: "Persistent revocation with SQLite backend."
//!
//! # Schema
//!
//! ```sql
//! CREATE TABLE IF NOT EXISTS revocations (
//!     id TEXT PRIMARY KEY,
//!     revoked_at TEXT NOT NULL,
//!     reason TEXT NOT NULL,
//!     revoked_by TEXT NOT NULL
//! );
//! ```

use hkask_types::InfrastructureError;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Revocation store errors
#[derive(Debug, Error)]
pub enum RevocationError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),
}

impl From<rusqlite::Error> for RevocationError {
    fn from(e: rusqlite::Error) -> Self {
        RevocationError::Infra(InfrastructureError::Database(e.to_string()))
    }
}

/// A persisted revocation record
#[derive(Debug, Clone)]
pub struct RevocationRecord {
    /// The revoked capability or delegation ID
    pub id: String,
    /// ISO 8601 timestamp of revocation
    pub revoked_at: String,
    /// Human-readable reason for revocation
    pub reason: String,
    /// WebID of the authority that performed the revocation
    pub revoked_by: String,
}

/// SQLite-backed revocation store
///
/// Thread-safe via `Arc<Mutex<Connection>>`.
/// Shares the database connection with other stores.
pub struct RevocationStore {
    conn: Arc<Mutex<Connection>>,
}

impl RevocationStore {
    /// Create a new revocation store using the shared database connection.
    ///
    /// Initializes the `revocations` table if it does not exist.
    pub fn new(conn: Arc<Mutex<Connection>>) -> Result<Self, RevocationError> {
        let store = Self { conn };
        store.initialize_schema()?;
        Ok(store)
    }

    fn initialize_schema(&self) -> Result<(), RevocationError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS revocations (
                id TEXT PRIMARY KEY,
                revoked_at TEXT NOT NULL,
                reason TEXT NOT NULL,
                revoked_by TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_revocations_id ON revocations(id);",
        )?;
        Ok(())
    }

    /// Revoke a capability or delegation by ID.
    ///
    /// Returns `true` if the revocation was newly inserted,
    /// `false` if it was already revoked.
    pub fn revoke(
        &self,
        id: &str,
        reason: &str,
        revoked_by: &str,
    ) -> Result<bool, RevocationError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;

        // Check if already revoked
        let already_revoked: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM revocations WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if already_revoked {
            return Ok(false);
        }

        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO revocations (id, revoked_at, reason, revoked_by) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, now, reason, revoked_by],
        )?;
        Ok(true)
    }

    /// Check whether a capability or delegation ID has been revoked.
    pub fn is_revoked(&self, id: &str) -> Result<bool, RevocationError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM revocations WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Un-revoke a previously revoked ID.
    ///
    /// Returns `true` if the revocation was removed, `false` if it wasn't found.
    pub fn unrevoke(&self, id: &str) -> Result<bool, RevocationError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let deleted = conn.execute(
            "DELETE FROM revocations WHERE id = ?1",
            rusqlite::params![id],
        )?;
        Ok(deleted > 0)
    }

    /// List all active revocations.
    pub fn list_active(&self) -> Result<Vec<RevocationRecord>, RevocationError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, revoked_at, reason, revoked_by FROM revocations ORDER BY revoked_at DESC",
        )?;

        let entries = stmt
            .query_map([], |row| {
                Ok(RevocationRecord {
                    id: row.get(0)?,
                    revoked_at: row.get(1)?,
                    reason: row.get(2)?,
                    revoked_by: row.get(3)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Count total revocations.
    pub fn count(&self) -> Result<usize, RevocationError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM revocations", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Get the revocation record for a given ID, if it exists.
    pub fn get(&self, id: &str) -> Result<Option<RevocationRecord>, RevocationError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let mut stmt = conn
            .prepare("SELECT id, revoked_at, reason, revoked_by FROM revocations WHERE id = ?1")?;

        let record = stmt
            .query_row(rusqlite::params![id], |row| {
                Ok(RevocationRecord {
                    id: row.get(0)?,
                    revoked_at: row.get(1)?,
                    reason: row.get(2)?,
                    revoked_by: row.get(3)?,
                })
            })
            .ok();

        Ok(record)
    }
}
