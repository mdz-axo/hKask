//! Sovereignty checking for agent pods
//!
//! Ensures agent operations respect user sovereignty boundaries. Per the
//! Magna Carta, episodic memory / personal context / capability tokens /
//! OCAP boundaries are sovereign and require explicit consent; semantic
//! memory and template invocations are shared and require consent; the
//! hLexicon and template registry are public.
//!
//! Consent is resolved through a `SovereigntyConsent` port, decoupling the
//! checker from the concrete `ConsentManager`. Production wiring uses a
//! `ConsentManager`-backed port so that grants via `kask sovereignty grant`
//! or `POST /consent/grant` are observed on the next sovereignty check.

use hkask_types::{DataCategory, SovereigntyPort, UserSovereigntyState, WebID};
use std::sync::Arc;

/// Port for resolving explicit user consent for a (webid, category) pair.
///
/// Implementations are responsible for whatever backing store is appropriate
/// (a SQLite-backed `ConsentManager` in production, an in-memory map in
/// tests, an `AllowAll` policy in scaffolding).
pub trait SovereigntyConsent: Send + Sync {
    /// Returns `true` iff the given WebID has active, non-revoked consent
    /// for the given data category.
    fn has_consent(&self, webid: &str, category: &DataCategory) -> bool;
}

/// Default `SovereigntyConsent` implementation: deny everything.
///
/// Sovereignty must fail closed. New `PodManager`s use this until
/// `with_consent_port` is called with a real backend. This guarantees
/// that a misconfigured or partially-initialized agent cannot access
/// sovereign data without an explicit grant.
pub struct DenyAllConsent;

impl SovereigntyConsent for DenyAllConsent {
    fn has_consent(&self, _webid: &str, _category: &DataCategory) -> bool {
        false
    }
}

/// Test/scaffolding `SovereigntyConsent` implementation: grant everything.
///
/// Used in unit tests that don't care about the consent semantics, only
/// about the access path. Production must never use this — it bypasses
/// the Magna Carta's explicit-consent requirement.
pub struct AllowAllConsent;

impl SovereigntyConsent for AllowAllConsent {
    fn has_consent(&self, _webid: &str, _category: &DataCategory) -> bool {
        true
    }
}

/// Sovereignty checker for agent pods.
///
/// Reads explicit consent from the supplied `SovereigntyConsent` port.
/// This is the live wiring of the Magna Carta's "explicit consent tracking"
/// requirement.
pub struct SovereigntyChecker {
    state: UserSovereigntyState,
    owner_webid: WebID,
    consent: Arc<dyn SovereigntyConsent>,
}

impl Clone for SovereigntyChecker {
    fn clone(&self) -> Self {
        // The Arc<dyn SovereigntyConsent> shares state by reference, so a
        // shallow clone is safe and preserves the consent-port wiring.
        // `UserSovereigntyState` is `Copy`-able (it holds primitive enums).
        Self {
            state: self.state.clone(),
            owner_webid: self.owner_webid,
            consent: Arc::clone(&self.consent),
        }
    }
}

impl SovereigntyChecker {
    pub fn new(owner_webid: WebID, consent: Arc<dyn SovereigntyConsent>) -> Self {
        Self {
            state: UserSovereigntyState::new(),
            owner_webid,
            consent,
        }
    }

    /// Live per-category consent lookup.
    fn has_consent(&self, webid: &WebID, category: &DataCategory) -> bool {
        self.consent.has_consent(&webid.to_string(), category)
    }

    pub fn can_access(&self, data_category: &DataCategory, requester: &WebID) -> bool {
        if self.state.boundary.is_sovereign(data_category) {
            // Sovereign data: requires explicit consent AND requester == owner.
            return self.has_consent(requester, data_category) && requester == &self.owner_webid;
        }
        if self.state.boundary.is_shared(data_category) {
            // Shared data: requires explicit consent for the requesting WebID.
            return self.has_consent(requester, data_category);
        }
        self.state.boundary.is_public(data_category)
    }

    pub fn check_operation(&self, operation: &str, data_category: &DataCategory) -> bool {
        if operation == "acquisition" {
            return !self.state.boundary.prevents_passive_acquisition();
        }
        self.can_access(data_category, &self.owner_webid)
    }
}

impl SovereigntyPort for SovereigntyChecker {
    fn can_access(&self, data_category: &DataCategory, requester: &WebID) -> bool {
        SovereigntyChecker::can_access(self, data_category, requester)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::DataCategory;

    /// Helper: an in-memory consent port for tests.
    struct InMemoryConsent {
        grants: std::collections::HashSet<(String, DataCategory)>,
    }
    impl InMemoryConsent {
        fn new() -> Self {
            Self {
                grants: std::collections::HashSet::new(),
            }
        }
        fn grant(&mut self, webid: &str, category: DataCategory) {
            self.grants.insert((webid.to_string(), category));
        }
    }
    impl SovereigntyConsent for InMemoryConsent {
        fn has_consent(&self, webid: &str, category: &DataCategory) -> bool {
            self.grants.contains(&(webid.to_string(), category.clone()))
        }
    }

    /// Property: a sovereign category (episodic memory) requires BOTH
    /// explicit consent AND requester == owner. With consent but a different
    /// requester, access is denied.
    #[test]
    fn sovereign_category_requires_consent_and_owner() {
        let owner = WebID::new();
        let mut consent = InMemoryConsent::new();
        consent.grant(&owner.to_string(), DataCategory::EpisodicMemory);
        let checker =
            SovereigntyChecker::new(owner, Arc::new(consent) as Arc<dyn SovereigntyConsent>);

        // Owner with consent: allowed.
        assert!(checker.can_access(&DataCategory::EpisodicMemory, &owner));

        // Non-owner with consent: denied (sovereign requires ownership).
        let other = WebID::new();
        let mut consent = InMemoryConsent::new();
        consent.grant(&other.to_string(), DataCategory::EpisodicMemory);
        let checker =
            SovereigntyChecker::new(owner, Arc::new(consent) as Arc<dyn SovereigntyConsent>);
        assert!(
            !checker.can_access(&DataCategory::EpisodicMemory, &other),
            "sovereign data must require requester == owner"
        );
    }

    /// Property: a sovereign category with NO consent grant is denied
    /// even for the owner. This is the consent-tracking part of the
    /// Magna Carta.
    #[test]
    fn sovereign_category_denied_without_consent() {
        let owner = WebID::new();
        let consent = InMemoryConsent::new(); // no grants
        let checker =
            SovereigntyChecker::new(owner, Arc::new(consent) as Arc<dyn SovereigntyConsent>);
        assert!(!checker.can_access(&DataCategory::EpisodicMemory, &owner));
        assert!(!checker.can_access(&DataCategory::PersonalContext, &owner));
        assert!(!checker.can_access(&DataCategory::CapabilityTokens, &owner));
        assert!(!checker.can_access(&DataCategory::OcapBoundaries, &owner));
    }

    /// Property: shared data (semantic memory) requires explicit consent
    /// for the requesting WebID — ownership is irrelevant.
    #[test]
    fn shared_category_requires_consent_for_requester() {
        let owner = WebID::new();
        let other = WebID::new();
        let mut consent = InMemoryConsent::new();
        consent.grant(&other.to_string(), DataCategory::SemanticMemory);
        let checker =
            SovereigntyChecker::new(owner, Arc::new(consent) as Arc<dyn SovereigntyConsent>);
        // Other has consent: allowed.
        assert!(checker.can_access(&DataCategory::SemanticMemory, &other));
        // Owner has no consent: denied.
        assert!(!checker.can_access(&DataCategory::SemanticMemory, &owner));
    }

    /// Property: public data (hLexicon, registry) is always accessible.
    #[test]
    fn public_category_always_accessible() {
        let owner = WebID::new();
        let other = WebID::new();
        let consent = InMemoryConsent::new(); // no grants at all
        let checker =
            SovereigntyChecker::new(owner, Arc::new(consent) as Arc<dyn SovereigntyConsent>);
        assert!(checker.can_access(&DataCategory::HLexiconTerms, &other));
        assert!(checker.can_access(&DataCategory::TemplateRegistry, &other));
    }

    /// Property: `DenyAllConsent` denies everything except public data.
    /// This is the fail-closed default for misconfigured managers.
    #[test]
    fn deny_all_consent_blocks_sovereign_and_shared() {
        let owner = WebID::new();
        let checker = SovereigntyChecker::new(owner, Arc::new(DenyAllConsent));
        assert!(!checker.can_access(&DataCategory::EpisodicMemory, &owner));
        assert!(!checker.can_access(&DataCategory::SemanticMemory, &owner));
        // Public data is still accessible.
        assert!(checker.can_access(&DataCategory::HLexiconTerms, &owner));
    }

    /// Property: `AllowAllConsent` grants everything (test-only scaffold).
    /// Production code must not use this — it bypasses explicit consent.
    #[test]
    fn allow_all_consent_grants_sovereign_and_shared() {
        let owner = WebID::new();
        let other = WebID::new();
        let checker = SovereigntyChecker::new(owner, Arc::new(AllowAllConsent));
        assert!(checker.can_access(&DataCategory::EpisodicMemory, &other));
        assert!(checker.can_access(&DataCategory::SemanticMemory, &other));
    }
}
