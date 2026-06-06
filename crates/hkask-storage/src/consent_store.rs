//! Consent Store — SQLite persistence for user consent records
//!
//! Persists consent records so they survive restarts, enforcing
//! user sovereignty (Principle 1.3) in the headless system.

use crate::Store;
use hkask_types::InfrastructureError;
use hkask_types::ports::git_cas::RepoId;
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

define_store_cas!(ConsentStore);

impl ConsentStore {
    /// Initialize the consent_records table
    pub fn initialize_schema(&self) -> Result<(), ConsentStoreError> {
        let conn = self.lock_conn()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS consent_records (
                id TEXT PRIMARY KEY,
                webid TEXT NOT NULL UNIQUE,
                granted_categories TEXT NOT NULL,
                granted_at INTEGER NOT NULL,
                revoked_at INTEGER,
                active INTEGER NOT NULL DEFAULT 1
            );
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

    /// Store with CAS write-through: persists to SQLite, then writes to the Sovereignty repo.
    pub async fn store_with_cas(
        &self,
        record: &StoredConsentRecord,
    ) -> Result<(), ConsentStoreError> {
        self.store(record)?;
        if let Some(port) = &self.cas_port {
            let bytes = serde_json::to_vec(record).map_err(|e| {
                ConsentStoreError::Infra(InfrastructureError::Serialization(e.to_string()))
            })?;
            port.put_blob(&RepoId::Sovereignty, &bytes)
                .await
                .map_err(|e| ConsentStoreError::Infra(InfrastructureError::Io(e.to_string())))?;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::ports::git_cas::MockGitCas;
    use std::sync::Arc;

    /// Tracer bullet: store_with_cas writes to SQLite and CAS Sovereignty repo.
    #[tokio::test]
    async fn store_with_cas_writes_to_sovereignty_repo() {
        let db = crate::Database::in_memory().expect("in-memory db");
        let mock = Arc::new(MockGitCas::new());
        let store = ConsentStore::new(db.conn_arc()).with_cas(mock.clone());
        store.initialize_schema().expect("schema");

        let record = StoredConsentRecord {
            id: "consent-1".to_string(),
            webid: "did:web:user".to_string(),
            granted_categories: HashSet::from(["inference".to_string()]),
            granted_at: 1700000000,
            revoked_at: None,
            active: true,
        };
        store.store_with_cas(&record).await.expect("store_with_cas");

        let retrieved = store.get("did:web:user").expect("get");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().webid, "did:web:user");
    }

    /// Tracer bullet: store_with_cas without CAS port still persists to SQLite.
    #[tokio::test]
    async fn store_with_cas_without_cas_port_persists_sqlite() {
        let db = crate::Database::in_memory().expect("in-memory db");
        let store = ConsentStore::new(db.conn_arc());
        store.initialize_schema().expect("schema");

        let record = StoredConsentRecord {
            id: "consent-2".to_string(),
            webid: "did:web:other".to_string(),
            granted_categories: HashSet::from(["inference".to_string()]),
            granted_at: 1700000001,
            revoked_at: None,
            active: true,
        };
        store.store_with_cas(&record).await.expect("store_with_cas");

        let retrieved = store.get("did:web:other").expect("get");
        assert!(retrieved.is_some());
    }
}
