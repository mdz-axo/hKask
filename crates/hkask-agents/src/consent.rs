//! Consent Manager — User consent tracking for sovereignty boundaries
//!
//! Manages explicit user consent for data access:
//! - Grant consent for specific data categories
//! - Revoke consent
//! - Audit consent history
//! - Check consent status
//!
//! Consent records are persisted via `ConsentStore` (SQLite-backed),
//! so they survive restarts — enforcing user sovereignty (Principle 1.3).

use hkask_storage::{ConsentStore, Store, StoredConsentRecord, read_rwlock, write_rwlock};
use hkask_types::DataCategory;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use tracing::debug;
use uuid::Uuid;

use crate::sovereignty::SovereigntyConsent;

/// Consent manager errors
#[derive(Debug, Error)]
pub enum ConsentError {
    #[error("Consent store error: {0}")]
    Store(#[from] hkask_storage::ConsentStoreError),

    #[error("Consent not found for WebID: {0}")]
    ConsentNotFound(String),

    #[error(transparent)]
    Infra(#[from] hkask_types::InfrastructureError),
}

/// Consent record (in-memory cache entry)
#[derive(Debug, Clone)]
pub(crate) struct ConsentRecord {
    pub(crate) webid: String,
    pub(crate) granted_categories: HashSet<String>,
    pub(crate) granted_at: i64,
    pub(crate) revoked_at: Option<i64>,
    pub(crate) active: bool,
}

impl ConsentRecord {
    pub fn new(webid: &str) -> Self {
        Self {
            webid: webid.to_string(),
            granted_categories: HashSet::new(),
            granted_at: chrono::Utc::now().timestamp(),
            revoked_at: None,
            active: true,
        }
    }

    pub fn grant(&mut self, category: &str) {
        self.granted_categories.insert(category.to_string());
        self.active = true;
        self.revoked_at = None;
    }

    pub fn revoke(&mut self) {
        self.revoked_at = Some(chrono::Utc::now().timestamp());
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active && self.revoked_at.is_none()
    }

    pub fn has_category(&self, category: &str) -> bool {
        self.active && self.granted_categories.contains(category)
    }
}

impl From<StoredConsentRecord> for ConsentRecord {
    fn from(stored: StoredConsentRecord) -> Self {
        Self {
            webid: stored.webid,
            granted_categories: stored.granted_categories,
            granted_at: stored.granted_at,
            revoked_at: stored.revoked_at,
            active: stored.active,
        }
    }
}

impl ConsentRecord {
    /// Convert to a `StoredConsentRecord` for persistence
    fn to_stored(&self) -> StoredConsentRecord {
        StoredConsentRecord {
            id: format!("cr_{}", Uuid::new_v4().simple()),
            webid: self.webid.clone(),
            granted_categories: self.granted_categories.clone(),
            granted_at: self.granted_at,
            revoked_at: self.revoked_at,
            active: self.active,
        }
    }
}

/// Consent manager with persistent storage
///
/// Uses a `ConsentStore` for persistence and an in-memory cache for
/// fast reads. Writes go to both the store and the cache; reads
/// check the cache first (loaded eagerly from the store on startup).
pub struct ConsentManager {
    store: ConsentStore,
    cache: Arc<RwLock<Vec<ConsentRecord>>>,
}

impl ConsentManager {
    /// Create a new consent manager backed by the given store
    pub fn new(store: ConsentStore) -> Self {
        let manager = Self {
            store,
            cache: Arc::new(RwLock::new(Vec::new())),
        };
        // Load existing records from the store into the cache
        if let Err(e) = manager.load_from_store() {
            tracing::warn!("Failed to load consent records from store: {}", e);
        }
        manager
    }

    /// Load all active consent records from the store into the in-memory cache
    fn load_from_store(&self) -> Result<(), ConsentError> {
        let records = {
            let conn_lock = self.store.lock_conn()?;

            let mut stmt = conn_lock
                .prepare(
                    "SELECT id, webid, granted_categories, granted_at, revoked_at, active
                     FROM consent_records WHERE active = 1",
                )
                .map_err(|e| ConsentError::Store(hkask_storage::ConsentStoreError::from(e)))?;

            stmt.query_map([], |row| {
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
            .map_err(|e| ConsentError::Store(hkask_storage::ConsentStoreError::from(e)))?
            .filter_map(|r| r.ok())
            .map(ConsentRecord::from)
            .collect::<Vec<_>>()
        };

        let mut cache = write_rwlock(&self.cache)?;
        *cache = records;
        Ok(())
    }

    /// Persist a consent record to the store
    fn persist(&self, record: &ConsentRecord) -> Result<(), ConsentError> {
        let stored = record.to_stored();
        self.store.store(&stored)?;
        Ok(())
    }

    /// Grant consent for a data category
    pub fn grant_consent(&self, webid: &str, category: &DataCategory) -> Result<(), ConsentError> {
        let mut cache = write_rwlock(&self.cache)?;

        // Find or create consent record
        let record = cache.iter_mut().find(|r| r.webid == webid);

        if let Some(record) = record {
            record.grant(category.as_str());
            self.persist(record)?;
        } else {
            let mut new_record = ConsentRecord::new(webid);
            new_record.grant(category.as_str());
            self.persist(&new_record)?;
            cache.push(new_record);
        }

        debug!(
            "Granted consent for WebID: {} category: {}",
            webid,
            category.as_str()
        );
        Ok(())
    }

    /// Revoke all consent for a WebID
    pub fn revoke_consent(&self, webid: &str) -> Result<(), ConsentError> {
        let mut cache = write_rwlock(&self.cache)?;

        if let Some(record) = cache.iter_mut().find(|r| r.webid == webid) {
            record.revoke();
            self.persist(record)?;
            debug!("Revoked consent for WebID: {}", webid);
            Ok(())
        } else {
            Err(ConsentError::ConsentNotFound(webid.to_string()))
        }
    }

    /// Check if consent is granted for a data category
    pub fn has_consent(&self, webid: &str, category: &DataCategory) -> Result<bool, ConsentError> {
        let cache = read_rwlock(&self.cache)?;

        Ok(cache
            .iter()
            .find(|r| r.webid == webid)
            .map(|r| r.has_category(category.as_str()))
            .unwrap_or(false))
    }

    /// Get all granted categories for a WebID
    pub fn get_granted_categories(&self, webid: &str) -> Result<Vec<String>, ConsentError> {
        let cache = read_rwlock(&self.cache)?;

        Ok(cache
            .iter()
            .find(|r| r.webid == webid && r.is_active())
            .map(|r| r.granted_categories.iter().cloned().collect())
            .unwrap_or_default())
    }
}

impl SovereigntyConsent for ConsentManager {
    fn has_consent(&self, webid: &str, category: &DataCategory) -> bool {
        // Translate storage errors into "deny by default" — sovereignty must
        // fail closed, never open. The Magna Carta's "Maximum" default
        // resistance is enforced by this conservative translation.
        ConsentManager::has_consent(self, webid, category).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::Database;
    use hkask_types::DataCategory;

    fn test_manager() -> ConsentManager {
        let db = Database::in_memory().expect("in-memory db");
        let store = ConsentStore::new(db.conn_arc());
        store.initialize_schema().expect("consent schema");
        ConsentManager::new(store)
    }

    /// Property: a freshly-constructed `ConsentManager` reports no consent
    /// for any (webid, category). This is the fail-closed default.
    #[test]
    fn fresh_consent_manager_denies_everything() {
        let mgr = test_manager();
        let webid = "hkask:test:user";
        assert!(
            !mgr.has_consent(webid, &DataCategory::EpisodicMemory)
                .unwrap()
        );
        assert!(
            !mgr.has_consent(webid, &DataCategory::SemanticMemory)
                .unwrap()
        );
    }

    /// Property: after `grant_consent`, the `SovereigntyConsent::has_consent`
    /// lookup reflects the grant. This is the wired-up Magna Carta consent
    /// tracking.
    #[test]
    fn grant_consent_is_observed_by_sovereignty_lookup() {
        let mgr = test_manager();
        let webid = "hkask:test:user";

        // Before grant: no consent.
        assert!(
            !mgr.has_consent(webid, &DataCategory::EpisodicMemory)
                .unwrap()
        );

        // After grant: consent is observed.
        mgr.grant_consent(webid, &DataCategory::EpisodicMemory)
            .expect("grant");
        assert!(
            mgr.has_consent(webid, &DataCategory::EpisodicMemory)
                .unwrap()
        );
        // Other categories remain unconsented.
        assert!(
            !mgr.has_consent(webid, &DataCategory::SemanticMemory)
                .unwrap()
        );
    }

    /// Property: after `revoke_consent`, the lookup reverts to denied.
    #[test]
    fn revoke_consent_revokes_lookup() {
        let mgr = test_manager();
        let webid = "hkask:test:user";
        mgr.grant_consent(webid, &DataCategory::SemanticMemory)
            .expect("grant");
        assert!(
            mgr.has_consent(webid, &DataCategory::SemanticMemory)
                .unwrap()
        );
        mgr.revoke_consent(webid).expect("revoke");
        assert!(
            !mgr.has_consent(webid, &DataCategory::SemanticMemory)
                .unwrap()
        );
    }

    /// Property: the `SovereigntyConsent` adapter (deny-on-error) returns
    /// `false` for any unconsented (webid, category) pair, matching the
    /// charter's "deny by default" requirement.
    #[test]
    fn sovereignty_consent_adapter_denies_unconsented() {
        let mgr = test_manager();
        let webid = "hkask:test:user";
        mgr.grant_consent(webid, &DataCategory::EpisodicMemory)
            .expect("grant");
        // Disambiguate the trait method from the inherent one (which
        // returns Result).
        let sovereignty_has = |cat: &DataCategory| {
            <ConsentManager as SovereigntyConsent>::has_consent(&mgr, webid, cat)
        };
        // Granted category: consent reported.
        assert!(sovereignty_has(&DataCategory::EpisodicMemory));
        // Ungranted category: consent denied.
        assert!(!sovereignty_has(&DataCategory::PersonalContext));
    }
}
