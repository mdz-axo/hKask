//! Sovereignty checking for agent pods
//!
//! Ensures agent operations respect user sovereignty boundaries.
//! Integrates with CNS for sovereignty event emission.

use hkask_cns::spans::SpanEmitter;
use hkask_types::{UserSovereigntyState, WebID};
use serde_json::Value;

/// Sovereignty checker for agent pods
pub struct SovereigntyChecker {
    state: UserSovereigntyState,
    span_emitter: SpanEmitter,
    owner_webid: WebID,
}

impl SovereigntyChecker {
    /// Create new sovereignty checker
    pub fn new(owner_webid: WebID) -> Self {
        Self {
            state: UserSovereigntyState::new(),
            span_emitter: SpanEmitter::new(owner_webid),
            owner_webid,
        }
    }

    /// Create with custom sovereignty state
    pub fn with_state(state: UserSovereigntyState, owner_webid: WebID) -> Self {
        Self {
            state,
            span_emitter: SpanEmitter::new(owner_webid),
            owner_webid,
        }
    }

    /// Check if data category is accessible
    pub fn can_access(&self, data_category: &str, requester: &WebID) -> bool {
        let boundary = &self.state.boundary;

        // Sovereign data requires explicit consent and owner
        if boundary.is_sovereign(data_category) {
            return self.state.explicit_consent && requester == &self.owner_webid;
        }

        // Shared data requires consent
        if boundary.shared_data.contains(&data_category.to_string()) {
            return self.state.explicit_consent;
        }

        // Public data is always accessible
        boundary.public_data.contains(&data_category.to_string())
    }

    /// Check if operation respects sovereignty
    pub fn check_operation(&self, operation: &str, data_category: &str) -> bool {
        // Check acquisition resistance
        if operation == "acquisition" {
            return !self
                .state
                .boundary
                .resistance
                .prevents_passive_acquisition();
        }

        // Check data access
        self.can_access(data_category, &self.owner_webid)
    }

    /// Mark acquisition attempt and emit CNS event
    pub fn mark_acquisition_attempt(&mut self, details: &Value) {
        self.state.mark_acquisition_attempt();
        self.span_emitter
            .emit_sovereignty("acquisition_attempt", details.clone());
    }

    /// Update VC investment and check for kill zone
    pub fn update_vc_investment(&mut self, vc_investment: f32) {
        self.state.update_vc_investment(vc_investment);

        if self.state.is_compromised() {
            self.span_emitter.emit_sovereignty_alert(
                "killzone",
                serde_json::json!({
                    "vc_investment": vc_investment,
                    "threshold": self.state.detector.threshold,
                    "compromised": true
                }),
            );
        }
    }

    /// Get current sovereignty state
    pub fn get_state(&self) -> &UserSovereigntyState {
        &self.state
    }

    /// Grant explicit consent
    pub fn grant_consent(&mut self) {
        self.state.grant_consent();
        self.span_emitter.emit_sovereignty(
            "consent_granted",
            serde_json::json!({
                "consent": true
            }),
        );
    }

    /// Revoke explicit consent
    pub fn revoke_consent(&mut self) {
        self.state.revoke_consent();
        self.span_emitter.emit_sovereignty(
            "consent_revoked",
            serde_json::json!({
                "consent": false
            }),
        );
    }

    /// Check if sovereignty is compromised
    pub fn is_compromised(&self) -> bool {
        self.state.is_compromised()
    }

    /// Check if kill zone is active
    pub fn kill_zone_active(&self) -> bool {
        self.state.detector.kill_zone_active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sovereignty_checker_new() {
        let checker = SovereigntyChecker::new(WebID::new());
        assert!(!checker.is_compromised());
        assert!(!checker.kill_zone_active());
    }

    #[test]
    fn test_can_access_sovereign_data() {
        let owner = WebID::new();
        let mut checker = SovereigntyChecker::new(owner);
        // Sovereign data requires consent
        assert!(!checker.can_access("episodic_memory", &owner));

        // Grant consent
        checker.grant_consent();
        // Now accessible to owner
        assert!(checker.can_access("episodic_memory", &owner));
        // But not to others
        assert!(!checker.can_access("episodic_memory", &WebID::new()));
    }

    #[test]
    fn test_can_access_public_data() {
        let checker = SovereigntyChecker::new(WebID::new());
        // Public data is always accessible
        assert!(checker.can_access("hlexicon_terms", &WebID::new()));
    }

    #[test]
    fn test_acquisition_resistance() {
        let checker = SovereigntyChecker::new(WebID::new());
        // Default resistance is Maximum, which prevents passive acquisition
        assert!(!checker.check_operation("acquisition", "test"));
    }

    #[test]
    fn test_kill_zone_detection() {
        let mut checker = SovereigntyChecker::new(WebID::new());
        checker.mark_acquisition_attempt(&serde_json::json!({}));
        checker.update_vc_investment(0.3);
        assert!(checker.is_compromised());
        assert!(checker.kill_zone_active());
    }

    #[test]
    fn test_consent_tracking() {
        let mut checker = SovereigntyChecker::new(WebID::new());
        assert!(!checker.get_state().explicit_consent);
        checker.grant_consent();
        assert!(checker.get_state().explicit_consent);
        checker.revoke_consent();
        assert!(!checker.get_state().explicit_consent);
    }
}
