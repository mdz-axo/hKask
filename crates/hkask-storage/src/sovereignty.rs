//! Sovereignty Boundary Store — SQLite persistence for user sovereignty boundaries
//
//! Persists user-configured sovereignty boundaries including:
//! - Sovereign data categories (require explicit consent)
//! - Shared data categories (require consent)
//! - Public data categories (always accessible)
//! - Affirmative consent requirements
use crate::Store;
use hkask_types::InfrastructureError;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;
/// Sovereignty boundary store errors
#[derive(Debug, Error)]
pub enum SovereigntyStoreError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),
    #[error("UUID parse error: {0}")]
    UuidParse(String),
}
impl_from_rusqlite!(SovereigntyStoreError, Infra);
impl_from_serde_json!(SovereigntyStoreError, Infra);
/// Stored sovereignty boundary entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereigntyBoundaryEntry {
    pub id: String,
    pub webid: String,
    pub sovereign_categories: Vec<String>,
    pub shared_categories: Vec<String>,
    pub public_categories: Vec<String>,
    pub requires_affirmative_consent: String,
    pub created_at: i64,
    pub updated_at: i64,
}
define_store!(SovereigntyBoundaryStore);
impl SovereigntyBoundaryStore {
    /// Initialize the database schema
    ///
    /// Creates the `sovereignty_boundaries` table if it doesn't exist and
    /// applies any pending migrations (column renames, column drops from
    /// Magna Carta refactoring).
    /// Initialize the sovereignty boundary store schema.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P1\] Motivating: User Sovereignty — schema for sovereignty boundaries
    /// post: sovereignty_boundaries table created if not exists
    pub fn initialize_schema(&self) -> Result<(), SovereigntyStoreError> {
        let conn = self.lock_conn()?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sovereignty_boundaries (
                id TEXT PRIMARY KEY,
                webid TEXT NOT NULL UNIQUE,
                sovereign_categories TEXT NOT NULL,
                shared_categories TEXT NOT NULL,
                public_categories TEXT NOT NULL,
                requires_affirmative_consent TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sovereignty_webid ON sovereignty_boundaries(webid);
            CREATE INDEX IF NOT EXISTS idx_sovereignty_updated ON sovereignty_boundaries(updated_at);
            ",
        )?;
        // Migrations for Magna Carta refactoring
        self.migrate_resistance_column(&conn)?;
        self.migrate_drop_kill_zone(&conn)?;
        Ok(())
    }
    /// Migrate the `resistance` column to `requires_affirmative_consent`.
    ///
    /// The Magna Carta renamed the `resistance` column (legacy "Acquisition
    /// Resistance" naming) to `requires_affirmative_consent`. If the old column
    /// is present, this rebuilds the table with the new schema.
    fn migrate_resistance_column(
        &self,
        conn: &rusqlite::Connection,
    ) -> Result<(), SovereigntyStoreError> {
        let has_old_column = conn
            .prepare("SELECT resistance FROM sovereignty_boundaries LIMIT 0")
            .is_ok();
        if !has_old_column {
            return Ok(());
        }
        tracing::info!(
            target: "hkask.storage.migration",
            "Migrating sovereignty_boundaries: resistance → requires_affirmative_consent"
        );
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sovereignty_boundaries_new (
                id TEXT PRIMARY KEY,
                webid TEXT NOT NULL UNIQUE,
                sovereign_categories TEXT NOT NULL,
                shared_categories TEXT NOT NULL,
                public_categories TEXT NOT NULL,
                requires_affirmative_consent TEXT NOT NULL DEFAULT 'true',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            INSERT OR IGNORE INTO sovereignty_boundaries_new
                (id, webid, sovereign_categories, shared_categories, public_categories,
                 requires_affirmative_consent, created_at, updated_at)
            SELECT id, webid, sovereign_categories, shared_categories, public_categories,
                   'true', created_at, updated_at
            FROM sovereignty_boundaries;
            DROP TABLE sovereignty_boundaries;
            ALTER TABLE sovereignty_boundaries_new RENAME TO sovereignty_boundaries;
            CREATE INDEX IF NOT EXISTS idx_sovereignty_webid ON sovereignty_boundaries(webid);
            CREATE INDEX IF NOT EXISTS idx_sovereignty_updated ON sovereignty_boundaries(updated_at);
            ",
        )?;
        Ok(())
    }
    /// Drop the `kill_zone_threshold` column (kill-zone was removed from Magna Carta).
    ///
    /// If the column is present, this rebuilds the table without it.
    fn migrate_drop_kill_zone(
        &self,
        conn: &rusqlite::Connection,
    ) -> Result<(), SovereigntyStoreError> {
        let has_column = conn
            .prepare("SELECT kill_zone_threshold FROM sovereignty_boundaries LIMIT 0")
            .is_ok();
        if !has_column {
            return Ok(());
        }
        tracing::info!(
            target: "hkask.storage.migration",
            "Migrating sovereignty_boundaries: dropping kill_zone_threshold column"
        );
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sovereignty_boundaries_new (
                id TEXT PRIMARY KEY,
                webid TEXT NOT NULL UNIQUE,
                sovereign_categories TEXT NOT NULL,
                shared_categories TEXT NOT NULL,
                public_categories TEXT NOT NULL,
                requires_affirmative_consent TEXT NOT NULL DEFAULT 'true',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            INSERT OR IGNORE INTO sovereignty_boundaries_new
                (id, webid, sovereign_categories, shared_categories, public_categories,
                 requires_affirmative_consent, created_at, updated_at)
            SELECT id, webid, sovereign_categories, shared_categories, public_categories,
                   requires_affirmative_consent, created_at, updated_at
            FROM sovereignty_boundaries;
            DROP TABLE sovereignty_boundaries;
            ALTER TABLE sovereignty_boundaries_new RENAME TO sovereignty_boundaries;
            CREATE INDEX IF NOT EXISTS idx_sovereignty_webid ON sovereignty_boundaries(webid);
            CREATE INDEX IF NOT EXISTS idx_sovereignty_updated ON sovereignty_boundaries(updated_at);
            ",
        )?;
        Ok(())
    }
    /// Store sovereignty boundary for a WebID
    /// Store a sovereignty boundary entry.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P1\] Motivating: User Sovereignty — store a sovereignty boundary entry
    /// pre:  entry.webid is non-empty
    /// post: entry inserted or replaced
    pub fn store(&self, entry: &SovereigntyBoundaryEntry) -> Result<(), SovereigntyStoreError> {
        let conn = self.lock_conn()?;
        let sovereign_json = serde_json::to_string(&entry.sovereign_categories)?;
        let shared_json = serde_json::to_string(&entry.shared_categories)?;
        let public_json = serde_json::to_string(&entry.public_categories)?;
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO sovereignty_boundaries
             (id, webid, sovereign_categories, shared_categories, public_categories,
              requires_affirmative_consent, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(webid) DO UPDATE SET
                sovereign_categories = excluded.sovereign_categories,
                shared_categories = excluded.shared_categories,
                public_categories = excluded.public_categories,
                requires_affirmative_consent = excluded.requires_affirmative_consent,
                updated_at = excluded.updated_at",
            params![
                entry.id,
                entry.webid,
                sovereign_json,
                shared_json,
                public_json,
                entry.requires_affirmative_consent,
                entry.created_at,
                now
            ],
        )?;
        Ok(())
    }
    /// Get sovereignty boundary for a WebID
    /// Get sovereignty boundary entries for a WebID.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P1\] Motivating: User Sovereignty — get boundaries for a WebID
    /// pre:  webid is non-empty
    /// post: returns Vec of entries for this WebID
    pub fn get(
        &self,
        webid: &str,
    ) -> Result<Option<SovereigntyBoundaryEntry>, SovereigntyStoreError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, webid, sovereign_categories, shared_categories, public_categories,
                    requires_affirmative_consent, created_at, updated_at
             FROM sovereignty_boundaries WHERE webid = ?1",
        )?;
        let entry = stmt
            .query_row(params![webid], |row| {
                let id: String = row.get(0)?;
                let webid: String = row.get(1)?;
                let sovereign_json: String = row.get(2)?;
                let shared_json: String = row.get(3)?;
                let public_json: String = row.get(4)?;
                let requires_affirmative_consent: String = row.get(5)?;
                let created_at: i64 = row.get(6)?;
                let updated_at: i64 = row.get(7)?;
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
                    requires_affirmative_consent,
                    created_at,
                    updated_at,
                })
            })
            .optional()?;
        Ok(entry)
    }
    /// Delete sovereignty boundary for a WebID
    /// Delete sovereignty boundary entries for a WebID.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P1\] Motivating: User Sovereignty — delete boundaries for a WebID
    /// pre:  webid is non-empty
    /// post: entries deleted for this WebID
    pub fn delete(&self, webid: &str) -> Result<(), SovereigntyStoreError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "DELETE FROM sovereignty_boundaries WHERE webid = ?1",
            params![webid],
        )?;
        Ok(())
    }
}
