//! Consent Store — SQLite persistence for user consent records
//!
//! Persists consent records so they survive restarts, enforcing
//! user sovereignty (Principle 1.3) in the headless system.

use crate::Store;
use hkask_types::InfrastructureError;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

/// Consent store errors
#[derive(Debug, Error)]
pub enum ConsentStoreError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),

    #[error("Consent record not found for WebID: {0}")]
    NotFound(String),
}

impl_from_rusqlite!(ConsentStoreError, Infra);

impl_from_serde_json!(ConsentStoreError, Infra);

/// Persistent consent record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredConsentRecord {
    pub id: String,
    pub webid: String,
    pub granted_categories: HashSet<String>,
    pub granted_at: i64,
    pub revoked_at: Option<i64>,
    pub active: bool,
}

define_store!(ConsentStore);

impl ConsentStore {
    /// Initialize the consent_records table
    pub fn initialize_schema(&self) -> Result<(), ConsentStoreError> {
        let conn = self.lock_conn()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS consent_records (
                id TEXT PRIMARY KEY,
                webid TEXT NOT NULL,
                granted_categories TEXT NOT NULL,
                granted_at INTEGER NOT NULL,
                revoked_at INTEGER,
                active INTEGER NOT NULL DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_consent_webid ON consent_records(webid);
            CREATE INDEX IF NOT EXISTS idx_consent_active ON consent_records(active);
            ",
        )?;
        Ok(())
    }

    /// Store (upsert) a consent record for a WebID
    pub fn store(&self, record: &StoredConsentRecord) -> Result<(), ConsentStoreError> {
        let conn = self.lock_conn()?;
        let categories_json = serde_json::to_string(&record.granted_categories)?;
        let active_int = if record.active { 1 } else { 0 };

        conn.execute(
            "INSERT INTO consent_records (id, webid, granted_categories, granted_at, revoked_at, active)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(webid) DO UPDATE SET
                granted_categories = excluded.granted_categories,
                granted_at = excluded.granted_at,
                revoked_at = excluded.revoked_at,
                active = excluded.active",
            params![
                record.id,
                record.webid,
                categories_json,
                record.granted_at,
                record.revoked_at,
                active_int,
            ],
        )?;

        Ok(())
    }

    /// Get the active consent record for a WebID
    pub fn get(&self, webid: &str) -> Result<Option<StoredConsentRecord>, ConsentStoreError> {
        let conn = self.lock_conn()?;

        let mut stmt = conn.prepare(
            "SELECT id, webid, granted_categories, granted_at, revoked_at, active
             FROM consent_records WHERE webid = ?1",
        )?;

        let record = stmt
            .query_row(params![webid], |row| {
                let id: String = row.get(0)?;
                let webid: String = row.get(1)?;
                let categories_json: String = row.get(2)?;
                let granted_at: i64 = row.get(3)?;
                let revoked_at: Option<i64> = row.get(4)?;
                let active_int: i32 = row.get(5)?;

                let granted_categories: HashSet<String> = serde_json::from_str(&categories_json)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

                Ok(StoredConsentRecord {
                    id,
                    webid,
                    granted_categories,
                    granted_at,
                    revoked_at,
                    active: active_int != 0,
                })
            })
            .optional()?;

        Ok(record)
    }

    /// Delete consent record for a WebID
    pub fn delete(&self, webid: &str) -> Result<(), ConsentStoreError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "DELETE FROM consent_records WHERE webid = ?1",
            params![webid],
        )?;
        Ok(())
    }
}
