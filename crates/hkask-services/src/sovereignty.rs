//! Sovereignty service — consent management wrapped behind a clean async interface.
//!
//! SovereigntyService wraps `ConsentManager` so external callers (CLI/API)
//! never access raw store internals. Created via `AgentService::build()`.

use std::sync::Arc;

use hkask_agents::consent::ConsentManager;
use hkask_types::sovereignty::DataCategory;

/// Service for sovereignty consent operations — grant, revoke, check.
///
/// Wraps the shared `ConsentManager` behind a clean API so callers
/// don't need a direct dependency on the `hkask_agents::consent` module.
#[derive(Clone)]
pub struct SovereigntyService {
    consent: Arc<ConsentManager>,
}

impl SovereigntyService {
    /// Create from the shared consent manager.
    pub fn new(consent: Arc<ConsentManager>) -> Self {
        Self { consent }
    }

    /// Grant consent for a data category to the given WebID.
    pub fn grant_consent(&self, webid: &str, category: &DataCategory) -> Result<(), String> {
        self.consent
            .grant_consent(webid, category)
            .map_err(|e| e.to_string())
    }

    /// Revoke all consent for the given WebID.
    pub fn revoke_consent(&self, webid: &str) -> Result<(), String> {
        self.consent
            .revoke_consent(webid)
            .map_err(|e| e.to_string())
    }

    /// Check if the given WebID has consent for a data category.
    pub fn has_consent(&self, webid: &str, category: &DataCategory) -> bool {
        self.consent.has_consent(webid, category).unwrap_or(false)
    }

    /// Get all categories the given WebID has granted consent for.
    pub fn get_granted_categories(&self, webid: &str) -> Result<Vec<String>, String> {
        self.consent
            .get_granted_categories(webid)
            .map_err(|e| e.to_string())
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_agents::consent::ConsentManager;
    use hkask_storage::{ConsentStore, Database};

    // REQ: svc-sovereignty-001 — service delegates to ConsentManager
    //
    // A fresh SovereigntyService wrapping an empty ConsentManager should
    // report no consent and return zero granted categories.
    #[test]
    fn fresh_service_reports_no_consent() {
        let db = Database::in_memory().expect("in-memory database");
        let consent_store = ConsentStore::new(db.conn_arc());
        consent_store
            .initialize_schema()
            .expect("initialize schema");
        let cm = Arc::new(ConsentManager::new(consent_store));
        let svc = SovereigntyService::new(cm);

        let has = svc.has_consent("test-user", &DataCategory::EpisodicMemory);
        assert!(!has, "Fresh consent manager should not have consent");

        let granted = svc.get_granted_categories("test-user").unwrap();
        assert!(
            granted.is_empty(),
            "Fresh consent manager should have no grants"
        );
    }
}
