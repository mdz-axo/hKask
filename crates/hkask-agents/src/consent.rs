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
use hkask_types::WebID;
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span, SpanNamespace};
use hkask_types::sovereignty::DataCategory;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use tracing::{debug, warn};

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
    /// REQ: P2-agt-consent-record-new
    /// \[P2\] Motivating: Affirmative Consent — consent record starts empty and active
    /// \[P1\] Constraining: User Sovereignty — record is bound to user WebID
    /// pre:  `webid` is a non-empty string.
    /// post: Returns a new `ConsentRecord` with empty granted categories,
    ///       `active = true`, `revoked_at = None`, and `granted_at` set to
    ///       the current UTC timestamp.
    pub fn new(webid: &str) -> Self {
        Self {
            webid: webid.to_string(),
            granted_categories: HashSet::new(),
            granted_at: chrono::Utc::now().timestamp(),
            revoked_at: None,
            active: true,
        }
    }

    /// REQ: P2-agt-consent-record-grant
    /// \[P2\] Motivating: Affirmative Consent — explicit grant adds a data category
    /// pre:  `category` is a non-empty string.
    /// post: `category` is added to `granted_categories`; `active` is set
    ///       to `true`; `revoked_at` is cleared to `None`.
    pub fn grant(&mut self, category: &str) {
        self.granted_categories.insert(category.to_string());
        self.active = true;
        self.revoked_at = None;
    }

    /// REQ: P2-agt-consent-record-revoke
    /// \[P2\] Motivating: Affirmative Consent — revocation terminates consent
    /// pre:  (none — revoke is always valid).
    /// post: `revoked_at` is set to the current UTC timestamp;
    ///       `active` is set to `false`.
    pub fn revoke(&mut self) {
        self.revoked_at = Some(chrono::Utc::now().timestamp());
        self.active = false;
    }

    /// REQ: P2-agt-consent-record-is-active
    /// \[P2\] Motivating: Affirmative Consent — active iff not revoked
    /// pre:  (none).
    /// post: Returns `true` iff `active == true` AND `revoked_at` is `None`.
    pub fn is_active(&self) -> bool {
        self.active && self.revoked_at.is_none()
    }

    /// REQ: P2-agt-consent-record-has-category
    /// \[P2\] Motivating: Affirmative Consent — category check enforces scoped grant
    /// pre:  `category` is a non-empty string.
    /// post: Returns `true` iff the record is active AND `category` is
    ///       present in `granted_categories`.
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
    /// Convert to a `StoredConsentRecord` for persistence.
    /// Uses a stable id derived from the webid to enable upserts
    /// rather than generating a new UUID per call.
    fn to_stored(&self) -> StoredConsentRecord {
        StoredConsentRecord {
            id: format!("cr_{}", self.webid),
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
    /// Optional CNS event sink for observability of consent denials.
    /// When set, a `cns.consent.denied` ν-event is emitted every time
    /// `has_consent` returns false, closing the observability loop
    /// on the Prohibition gate (Magna Carta P2).
    event_sink: Option<Arc<dyn NuEventSink>>,
}

impl ConsentManager {
    /// Create a new consent manager backed by the given store.
    ///
    /// REQ: P2-agt-consent-manager-new
    /// \[P2\] Motivating: Affirmative Consent — manager caches active consent records
    /// pre:  `store` is a valid, initialized `ConsentStore`.
    /// post: Returns a `ConsentManager` with an empty in-memory cache;
    ///       eagerly loads active records from the store into the cache;
    ///       logs a warning if the load fails (cache remains empty).
    pub fn new(store: ConsentStore) -> Self {
        let manager = Self {
            store,
            cache: Arc::new(RwLock::new(Vec::new())),
            event_sink: None,
        };
        // Load existing records from the store into the cache
        if let Err(e) = manager.load_from_store() {
            tracing::warn!("Failed to load consent records from store: {}", e);
        }
        manager
    }

    /// Set a CNS event sink for consent denial observability.
    ///
    /// When set, every `has_consent` denial produces a `cns.consent.denied`
    /// ν-event. This provides observability without opening a feedback path
    /// (the denial remains terminal — this is a Prohibition, not a Guardrail).
    /// # REQ: OPEN_QUESTIONS §2.2 — consent denial CNS instrumentation.
    ///
    /// REQ: P2-agt-consent-manager-with-sink
    /// \[P9\] Motivating: Homeostatic Self-Regulation — CNS instrumentation for denials (observability only, no feedback)
    /// pre:  `sink` is a valid `Arc<dyn NuEventSink>`.
    /// post: Returns `self` with `event_sink` set to `Some(sink)`.
    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
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

    /// Grant consent for a data category.
    ///
    /// REQ: P2-agt-consent-manager-grant
    /// \[P2\] Motivating: Affirmative Consent — persist a scoped grant
    /// pre:  `webid` is a non-empty string; `category` is a valid
    ///       `DataCategory` variant.
    /// post: If a record exists for `webid`, the category is granted and
    ///       persisted; otherwise a new record is created, granted, and
    ///       persisted. Returns `Ok(())` on success.
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

    /// Revoke all consent for a WebID.
    ///
    /// REQ: P2-agt-consent-manager-revoke
    /// \[P2\] Motivating: Affirmative Consent — revoke all consent for a WebID
    /// pre:  `webid` is a non-empty string.
    /// post: If a record exists for `webid`, it is revoked and persisted;
    ///       returns `Ok(())`. If no record exists, returns
    ///       `Err(ConsentError::ConsentNotFound)`.
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

    /// Check if consent is granted for a data category.
    ///
    /// Emits a `cns.consent.denied` ν-event when consent is denied,
    /// providing observability without opening a feedback path.
    ///
    /// REQ: P2-agt-consent-manager-check
    /// \[P2\] Motivating: Affirmative Consent — terminal deny unless active grant exists
    /// \[P1\] Constraining: User Sovereignty — check is per-user/data-category
    /// pre:  `webid` is a non-empty string; `category` is a valid
    ///       `DataCategory` variant.
    /// post: Returns `Ok(true)` if an active record for `webid` has the
    ///       category granted; `Ok(false)` otherwise (including when no
    ///       record exists). Emits a denial ν-event on `false`.
    pub fn has_consent(&self, webid: &str, category: &DataCategory) -> Result<bool, ConsentError> {
        let cache = read_rwlock(&self.cache)?;

        let granted = cache
            .iter()
            .find(|r| r.webid == webid)
            .map(|r| r.has_category(category.as_str()))
            .unwrap_or(false);

        if !granted {
            self.emit_consent_denied(webid, category);
        }

        Ok(granted)
    }

    /// Emit a `cns.consent.denied` ν-event for observability.
    ///
    /// This is a Prohibition-gate observation, not a regulatory loop signal.
    /// The denial is terminal — the event records the fact for audit.
    fn emit_consent_denied(&self, webid: &str, category: &DataCategory) {
        if let Some(ref sink) = self.event_sink {
            let event = NuEvent::new(
                WebID::new(),
                Span::new(SpanNamespace::new("cns.consent"), "denied"),
                Phase::Compare,
                serde_json::json!({
                    "webid": webid,
                    "category": category.as_str(),
                }),
                0,
            );
            if let Err(e) = sink.persist(&event) {
                warn!(
                    target: "cns.consent",
                    error = %e,
                    webid = %webid,
                    category = %category.as_str(),
                    "Failed to persist consent denial event"
                );
            }
        }
    }

    /// Get all granted categories for a WebID.
    ///
    /// REQ: P2-agt-consent-manager-granted-categories
    /// \[P2\] Motivating: Affirmative Consent — list granted categories for disclosure
    /// pre:  `webid` is a non-empty string.
    /// post: Returns `Ok(Vec<String>)` containing all granted category
    ///       names for an active record; returns `Ok(vec![])` if no active
    ///       record exists for `webid`.
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
        // fail-closed default deny is enforced by this conservative translation.
        ConsentManager::has_consent(self, webid, category).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: P2-agt-consent-record-new-test — new ConsentRecord starts active with empty grants
    #[test]
    fn consent_record_new_has_correct_defaults() {
        let record = ConsentRecord::new("user:alice");
        assert_eq!(record.webid, "user:alice");
        assert!(record.granted_categories.is_empty());
        assert!(record.is_active());
        assert!(record.revoked_at.is_none());
        assert!(record.granted_at > 0);
    }

    // REQ: P2-agt-consent-record-grant-test — grant() adds category, sets active, clears revoked_at
    #[test]
    fn consent_record_grant_adds_category_and_activates() {
        let mut record = ConsentRecord::new("user:alice");
        // First revoke to set inactive state, then grant to verify reactivation
        record.revoke();
        assert!(!record.is_active());

        record.grant("episodic_memory");
        assert!(record.is_active());
        assert!(record.revoked_at.is_none());
        assert!(record.has_category("episodic_memory"));
    }

    // REQ: P2-agt-consent-record-revoke-test — revoke() sets revoked_at and deactivates
    #[test]
    fn consent_record_revoke_sets_inactive() {
        let mut record = ConsentRecord::new("user:alice");
        record.grant("episodic_memory");
        assert!(record.is_active());

        record.revoke();
        assert!(!record.is_active());
        assert!(record.revoked_at.is_some());
        // After revoke, previously granted categories should not be accessible
        assert!(!record.has_category("episodic_memory"));
    }

    // REQ: P2-agt-consent-record-has-category-test — has_category() only true when active and granted
    #[test]
    fn consent_record_has_category_only_when_active_and_granted() {
        let mut record = ConsentRecord::new("user:alice");
        // Not granted yet
        assert!(!record.has_category("episodic_memory"));

        record.grant("episodic_memory");
        assert!(record.has_category("episodic_memory"));
        // Different category not granted
        assert!(!record.has_category("semantic_memory"));

        record.revoke();
        // After revoke, even granted categories are denied
        assert!(!record.has_category("episodic_memory"));
    }
}
