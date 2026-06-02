//! Sovereignty Boundary Store — SQLite persistence for user sovereignty boundaries
//!
//! Persists user-configured sovereignty boundaries including:
//! - Sovereign data categories (require explicit consent)
//! - Shared data categories (require consent)
//! - Public data categories (always accessible)
//! - Acquisition resistance settings
//! - Kill-zone detector thresholds

use hkask_types::InfrastructureError;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Sovereignty boundary store errors
#[derive(Debug, Error)]
pub enum SovereigntyStoreError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),

    #[error("UUID parse error: {0}")]
    UuidParse(String),
}

impl From<rusqlite::Error> for SovereigntyStoreError {
    fn from(e: rusqlite::Error) -> Self {
        SovereigntyStoreError::Infra(InfrastructureError::Database(e.to_string()))
    }
}

impl From<serde_json::Error> for SovereigntyStoreError {
    fn from(e: serde_json::Error) -> Self {
        InfrastructureError::from(e).into()
    }
}

/// Stored sovereignty boundary entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SovereigntyBoundaryEntry {
    pub id: String,
    pub webid: String,
    pub sovereign_categories: Vec<String>,
    pub shared_categories: Vec<String>,
    pub public_categories: Vec<String>,
    pub resistance: String,
    pub kill_zone_threshold: f32,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Sovereignty Boundary Store
#[derive(Clone)]
pub struct SovereigntyBoundaryStore {
    conn: Arc<Mutex<Connection>>,
}

impl SovereigntyBoundaryStore {
    /// Create new store with a shared encrypted connection
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Initialize the database schema
    pub fn initialize_schema(&self) -> Result<(), SovereigntyStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sovereignty_boundaries (
                id TEXT PRIMARY KEY,
                webid TEXT NOT NULL UNIQUE,
                sovereign_categories TEXT NOT NULL,
                shared_categories TEXT NOT NULL,
                public_categories TEXT NOT NULL,
                resistance TEXT NOT NULL,
                kill_zone_threshold REAL NOT NULL DEFAULT 0.2,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sovereignty_webid ON sovereignty_boundaries(webid);
            CREATE INDEX IF NOT EXISTS idx_sovereignty_updated ON sovereignty_boundaries(updated_at);
            ",
        )?;
        Ok(())
    }

    /// Store sovereignty boundary for a WebID
    pub fn store(&self, entry: &SovereigntyBoundaryEntry) -> Result<(), SovereigntyStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let sovereign_json = serde_json::to_string(&entry.sovereign_categories)?;
        let shared_json = serde_json::to_string(&entry.shared_categories)?;
        let public_json = serde_json::to_string(&entry.public_categories)?;
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO sovereignty_boundaries
             (id, webid, sovereign_categories, shared_categories, public_categories,
              resistance, kill_zone_threshold, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(webid) DO UPDATE SET
                sovereign_categories = excluded.sovereign_categories,
                shared_categories = excluded.shared_categories,
                public_categories = excluded.public_categories,
                resistance = excluded.resistance,
                kill_zone_threshold = excluded.kill_zone_threshold,
                updated_at = excluded.updated_at",
            params![
                entry.id,
                entry.webid,
                sovereign_json,
                shared_json,
                public_json,
                entry.resistance,
                entry.kill_zone_threshold,
                entry.created_at,
                now
            ],
        )?;

        Ok(())
    }

    /// Get sovereignty boundary for a WebID
    pub fn get(
        &self,
        webid: &str,
    ) -> Result<Option<SovereigntyBoundaryEntry>, SovereigntyStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, webid, sovereign_categories, shared_categories, public_categories,
                    resistance, kill_zone_threshold, created_at, updated_at
             FROM sovereignty_boundaries WHERE webid = ?1",
        )?;

        let entry = stmt
            .query_row(params![webid], |row| {
                let id: String = row.get(0)?;
                let webid: String = row.get(1)?;
                let sovereign_json: String = row.get(2)?;
                let shared_json: String = row.get(3)?;
                let public_json: String = row.get(4)?;
                let resistance: String = row.get(5)?;
                let kill_zone_threshold: f32 = row.get(6)?;
                let created_at: i64 = row.get(7)?;
                let updated_at: i64 = row.get(8)?;

                let sovereign_categories: Vec<String> = serde_json::from_str(&sovereign_json)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
                let shared_categories: Vec<String> = serde_json::from_str(&shared_json)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
                let public_categories: Vec<String> = serde_json::from_str(&public_json)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

                Ok(SovereigntyBoundaryEntry {
                    id,
                    webid,
                    sovereign_categories,
                    shared_categories,
                    public_categories,
                    resistance,
                    kill_zone_threshold,
                    created_at,
                    updated_at,
                })
            })
            .optional()?;

        Ok(entry)
    }

    /// Delete sovereignty boundary for a WebID
    pub fn delete(&self, webid: &str) -> Result<(), SovereigntyStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        conn.execute(
            "DELETE FROM sovereignty_boundaries WHERE webid = ?1",
            params![webid],
        )?;
        Ok(())
    }
}
