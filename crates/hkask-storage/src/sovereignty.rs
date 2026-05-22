//! Sovereignty Boundary Store — SQLite persistence for user sovereignty boundaries
//!
//! Persists user-configured sovereignty boundaries including:
//! - Sovereign data categories (require explicit consent)
//! - Shared data categories (require consent)
//! - Public data categories (always accessible)
//! - Acquisition resistance settings
//! - Kill-zone detector thresholds

use hkask_types::{AcquisitionResistance, DataCategory, KillZoneDetector, SovereigntyId, UserSovereigntyState};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use thiserror::Error;
use tracing::debug;

/// Sovereignty boundary store errors
#[derive(Debug, Error)]
pub enum SovereigntyStoreError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Sovereignty boundary not found for WebID: {0}")]
    NotFound(String),

    #[error("Invalid data category: {0}")]
    InvalidCategory(String),

    #[error("UUID parse error: {0}")]
    UuidParse(String),
}

/// Stored sovereignty boundary entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereigntyBoundaryEntry {
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

impl SovereigntyBoundaryEntry {
    /// Create from sovereignty state
    pub fn from_state(webid: &str, state: &UserSovereigntyState) -> Self {
        let now = chrono::Utc::now().timestamp();

        Self {
            id: state.boundary.id.0.to_string(),
            webid: webid.to_string(),
            sovereign_categories: state.boundary.sovereign_data.iter().map(|c| c.as_str().to_string()).collect(),
            shared_categories: state.boundary.shared_data.iter().map(|c| c.as_str().to_string()).collect(),
            public_categories: state.boundary.public_data.iter().map(|c| c.as_str().to_string()).collect(),
            resistance: format!("{:?}", state.boundary.resistance),
            kill_zone_threshold: state.detector.threshold,
            created_at: now,
            updated_at: now,
        }
    }

    /// Convert to sovereignty state
    pub fn to_state(&self) -> Result<UserSovereigntyState, SovereigntyStoreError> {
        let sovereignty_id = SovereigntyId(
            uuid::Uuid::parse_str(&self.id)
                .map_err(|e| SovereigntyStoreError::UuidParse(e.to_string()))?,
        );

        let resistance = match self.resistance.as_str() {
            "None" => AcquisitionResistance::None,
            "Low" => AcquisitionResistance::Low,
            "Medium" => AcquisitionResistance::Medium,
            "High" => AcquisitionResistance::High,
            "Maximum" => AcquisitionResistance::Maximum,
            _ => AcquisitionResistance::Maximum,
        };

        let parse_categories = |categories: &Vec<String>| -> Result<HashSet<DataCategory>, SovereigntyStoreError> {
            categories.iter().map(|s| {
                match s.as_str() {
                    "episodic_memory" => Ok(DataCategory::EpisodicMemory),
                    "semantic_memory" => Ok(DataCategory::SemanticMemory),
                    "personal_context" => Ok(DataCategory::PersonalContext),
                    "capability_tokens" => Ok(DataCategory::CapabilityTokens),
                    "ocap_boundaries" => Ok(DataCategory::OcapBoundaries),
                    "template_invocations" => Ok(DataCategory::TemplateInvocations),
                    "hlexicon_terms" => Ok(DataCategory::HLexiconTerms),
                    "template_registry" => Ok(DataCategory::TemplateRegistry),
                    other => Err(SovereigntyStoreError::InvalidCategory(other.to_string())),
                }
            }).collect()
        };

        let boundary = hkask_types::DataSovereigntyBoundary {
            id: sovereignty_id,
            sovereign_data: parse_categories(&self.sovereign_categories)?,
            shared_data: parse_categories(&self.shared_categories)?,
            public_data: parse_categories(&self.public_categories)?,
            resistance,
        };

        let detector = KillZoneDetector {
            vc_investment: 0.0,
            threshold: self.kill_zone_threshold,
            kill_zone_active: false,
            acquisition_attempt: false,
        };

        Ok(UserSovereigntyState {
            boundary,
            detector,
            explicit_consent: false,
            last_check: chrono::Utc::now(),
        })
    }
}

/// Sovereignty boundary statistics
#[derive(Debug, Clone)]
pub struct SovereigntyStoreStats {
    pub total_boundaries: usize,
    pub sovereign_boundaries: usize,
}

/// Sovereignty Boundary Store
pub struct SovereigntyBoundaryStore {
    conn: Connection,
}

impl SovereigntyBoundaryStore {
    /// Create new store at path
    pub fn new(path: &Path) -> Result<Self, SovereigntyStoreError> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.initialize_schema()?;
        Ok(store)
    }

    /// Create in-memory store
    pub fn in_memory() -> Result<Self, SovereigntyStoreError> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.initialize_schema()?;
        Ok(store)
    }

    fn initialize_schema(&self) -> Result<(), SovereigntyStoreError> {
        self.conn.execute_batch(
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
        let sovereign_json = serde_json::to_string(&entry.sovereign_categories)?;
        let shared_json = serde_json::to_string(&entry.shared_categories)?;
        let public_json = serde_json::to_string(&entry.public_categories)?;
        let now = chrono::Utc::now().timestamp();

        self.conn.execute(
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

        debug!("Stored sovereignty boundary for WebID: {}", entry.webid);
        Ok(())
    }

    /// Get sovereignty boundary for a WebID
    pub fn get(&self, webid: &str) -> Result<Option<SovereigntyBoundaryEntry>, SovereigntyStoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, webid, sovereign_categories, shared_categories, public_categories,
                    resistance, kill_zone_threshold, created_at, updated_at
             FROM sovereignty_boundaries WHERE webid = ?1"
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
        self.conn.execute(
            "DELETE FROM sovereignty_boundaries WHERE webid = ?1",
            params![webid],
        )?;
        debug!("Deleted sovereignty boundary for WebID: {}", webid);
        Ok(())
    }

    /// List all sovereignty boundaries
    pub fn list_all(&self) -> Result<Vec<SovereigntyBoundaryEntry>, SovereigntyStoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, webid, sovereign_categories, shared_categories, public_categories,
                    resistance, kill_zone_threshold, created_at, updated_at
             FROM sovereignty_boundaries ORDER BY created_at DESC"
        )?;

        let entries = stmt
            .query_map([], |row| {
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
            })?
            .filter_map(|result| result.ok())
            .collect();

        Ok(entries)
    }

    /// Update kill-zone threshold for a WebID
    pub fn update_kill_zone_threshold(
        &self,
        webid: &str,
        threshold: f32,
    ) -> Result<(), SovereigntyStoreError> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "UPDATE sovereignty_boundaries SET kill_zone_threshold = ?1, updated_at = ?2 WHERE webid = ?3",
            params![threshold, now, webid],
        )?;
        debug!("Updated kill-zone threshold for WebID: {} to {}", webid, threshold);
        Ok(())
    }

    /// Update acquisition resistance for a WebID
    pub fn update_resistance(
        &self,
        webid: &str,
        resistance: AcquisitionResistance,
    ) -> Result<(), SovereigntyStoreError> {
        let now = chrono::Utc::now().timestamp();
        let resistance_str = format!("{:?}", resistance);
        self.conn.execute(
            "UPDATE sovereignty_boundaries SET resistance = ?1, updated_at = ?2 WHERE webid = ?3",
            params![resistance_str, now, webid],
        )?;
        debug!("Updated resistance for WebID: {} to {:?}", webid, resistance);
        Ok(())
    }

    /// Get store statistics
    pub fn stats(&self) -> Result<SovereigntyStoreStats, SovereigntyStoreError> {
        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM sovereignty_boundaries", [], |row| {
                row.get(0)
            })?;

        let sovereign: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sovereignty_boundaries WHERE resistance = 'Maximum'",
            [],
            |row| row.get(0),
        )?;

        Ok(SovereigntyStoreStats {
            total_boundaries: total as usize,
            sovereign_boundaries: sovereign as usize,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sovereignty_store_in_memory() {
        let store = SovereigntyBoundaryStore::in_memory().unwrap();
        let stats = store.stats().unwrap();
        assert_eq!(stats.total_boundaries, 0);
    }

    #[test]
    fn test_sovereignty_store_roundtrip() {
        let store = SovereigntyBoundaryStore::in_memory().unwrap();
        let webid = "did:web:test.example.com:user1";

        let state = UserSovereigntyState::new();
        let entry = SovereigntyBoundaryEntry::from_state(webid, &state);

        store.store(&entry).unwrap();

        let retrieved = store.get(webid).unwrap();
        assert!(retrieved.is_some());

        let retrieved_entry = retrieved.unwrap();
        assert_eq!(retrieved_entry.webid, webid);

        let retrieved_state = retrieved_entry.to_state().unwrap();
        assert_eq!(
            retrieved_state.boundary.sovereign_data,
            state.boundary.sovereign_data
        );
    }

    #[test]
    fn test_sovereignty_store_update_threshold() {
        let store = SovereigntyBoundaryStore::in_memory().unwrap();
        let webid = "did:web:test.example.com:user2";

        let state = UserSovereigntyState::new();
        let entry = SovereigntyBoundaryEntry::from_state(webid, &state);
        store.store(&entry).unwrap();

        store.update_kill_zone_threshold(webid, 0.5).unwrap();

        let retrieved = store.get(webid).unwrap().unwrap();
        assert_eq!(retrieved.kill_zone_threshold, 0.5);
    }

    #[test]
    fn test_sovereignty_store_delete() {
        let store = SovereigntyBoundaryStore::in_memory().unwrap();
        let webid = "did:web:test.example.com:user3";

        let state = UserSovereigntyState::new();
        let entry = SovereigntyBoundaryEntry::from_state(webid, &state);
        store.store(&entry).unwrap();

        store.delete(webid).unwrap();

        let retrieved = store.get(webid).unwrap();
        assert!(retrieved.is_none());
    }
}
