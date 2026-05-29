//! Consent Manager — User consent tracking for sovereignty boundaries
//!
//! Manages explicit user consent for data access:
//! - Grant consent for specific data categories
//! - Revoke consent
//! - Audit consent history
//! - Check consent status

use hkask_storage::SovereigntyBoundaryStore;
use hkask_types::DataCategory;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::RwLock;
use thiserror::Error;
use tracing::{debug, info};

/// Consent manager errors
#[derive(Debug, Error)]
pub enum ConsentError {
    #[error("Sovereignty store error: {0}")]
    Store(#[from] hkask_storage::SovereigntyStoreError),

    #[error("Consent not found for WebID: {0}")]
    ConsentNotFound(String),

    #[error("Invalid data category: {0}")]
    InvalidCategory(String),

    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),
}

/// Consent record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentRecord {
    pub webid: String,
    pub granted_categories: HashSet<String>,
    pub granted_at: i64,
    pub revoked_at: Option<i64>,
    pub active: bool,
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

/// Consent manager
pub struct ConsentManager {
    #[allow(dead_code)]
    sovereignty_store: SovereigntyBoundaryStore,
    consent_cache: Arc<RwLock<Vec<ConsentRecord>>>,
}

impl ConsentManager {
    /// Create new consent manager
    pub fn new(sovereignty_store: SovereigntyBoundaryStore) -> Self {
        Self {
            sovereignty_store,
            consent_cache: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Grant consent for a data category
    pub fn grant_consent(&self, webid: &str, category: &DataCategory) -> Result<(), ConsentError> {
        let mut cache = self.consent_cache.write().map_err(|_| {
            ConsentError::ConsentNotFound("Consent cache lock poisoned".to_string())
        })?;

        // Find or create consent record
        let record = cache.iter_mut().find(|r| r.webid == webid);

        if let Some(record) = record {
            record.grant(category.as_str());
        } else {
            let mut new_record = ConsentRecord::new(webid);
            new_record.grant(category.as_str());
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
        let mut cache = self.consent_cache.write().map_err(|_| {
            ConsentError::ConsentNotFound("Consent cache lock poisoned".to_string())
        })?;

        if let Some(record) = cache.iter_mut().find(|r| r.webid == webid) {
            record.revoke();
            debug!("Revoked consent for WebID: {}", webid);
            Ok(())
        } else {
            Err(ConsentError::ConsentNotFound(webid.to_string()))
        }
    }

    /// Check if consent is granted for a data category
    pub fn has_consent(&self, webid: &str, category: &DataCategory) -> Result<bool, ConsentError> {
        let cache = self
            .consent_cache
            .read()
            .map_err(|e| ConsentError::LockPoisoned(e.to_string()))?;

        Ok(cache
            .iter()
            .find(|r| r.webid == webid)
            .map(|r| r.has_category(category.as_str()))
            .unwrap_or(false))
    }

    /// Get all granted categories for a WebID
    pub fn get_granted_categories(&self, webid: &str) -> Result<HashSet<String>, ConsentError> {
        let cache = self
            .consent_cache
            .read()
            .map_err(|e| ConsentError::LockPoisoned(e.to_string()))?;

        Ok(cache
            .iter()
            .find(|r| r.webid == webid)
            .map(|r| r.granted_categories.clone())
            .unwrap_or_default())
    }

    /// Check if any consent is active for a WebID
    pub fn has_any_consent(&self, webid: &str) -> Result<bool, ConsentError> {
        let cache = self
            .consent_cache
            .read()
            .map_err(|e| ConsentError::LockPoisoned(e.to_string()))?;

        Ok(cache
            .iter()
            .find(|r| r.webid == webid)
            .map(|r| r.is_active())
            .unwrap_or(false))
    }

    /// Clear all consent records
    pub fn clear(&self) -> Result<(), ConsentError> {
        let mut cache = self
            .consent_cache
            .write()
            .map_err(|e| ConsentError::LockPoisoned(e.to_string()))?;
        cache.clear();
        info!("Cleared all consent records");
        Ok(())
    }
}
