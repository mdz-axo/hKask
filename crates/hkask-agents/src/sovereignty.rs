//! Sovereignty checking for agent pods
//!
//! Ensures agent operations respect user sovereignty boundaries.
//! Integrates with CNS for sovereignty event emission.

use crate::ports::sovereignty::{
    SovereigntyCheckResult, SovereigntyOperation, SovereigntyPort,
};
use hkask_cns::spans::SpanEmitter;
use hkask_types::{DataCategory, UserSovereigntyState, WebID};
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
    pub fn can_access(&self, data_category: &DataCategory, requester: &WebID) -> bool {
        let category_str = data_category.as_str();

        // Sovereign data requires explicit consent and owner
        if self.state.boundary.is_sovereign_str(category_str) {
            return self.state.explicit_consent && requester == &self.owner_webid;
        }

        // Shared data requires consent
        if self.state.boundary.is_shared_str(category_str) {
            return self.state.explicit_consent;
        }

        // Public data is always accessible
        self.state.boundary.is_public_str(category_str)
    }

    /// Check if operation respects sovereignty
    pub fn check_operation(&self, operation: &str, data_category: &DataCategory) -> bool {
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

impl SovereigntyPort for SovereigntyChecker {
    fn check(
        &self,
        data_category: DataCategory,
        operation: SovereigntyOperation,
        requester: &WebID,
    ) -> SovereigntyCheckResult {
        // Check acquisition operations
        if operation == SovereigntyOperation::Acquisition {
            if !self
                .state
                .boundary
                .resistance
                .prevents_passive_acquisition()
            {
                return SovereigntyCheckResult::allowed(data_category, operation);
            } else {
                return SovereigntyCheckResult::denied(
                    data_category,
                    operation,
                    "Acquisition resistance prevents passive acquisition",
                );
            }
        }

        // Check data category sovereignty
        if self.state.boundary.is_sovereign(&data_category) {
            // Sovereign data requires explicit consent and must be owner
            if self.state.explicit_consent && requester == &self.owner_webid {
                SovereigntyCheckResult::allowed(data_category, operation)
            } else {
                SovereigntyCheckResult::denied(
                    data_category,
                    operation,
                    "Sovereign data requires owner consent",
                )
            }
        } else if self.state.boundary.is_shared(&data_category) {
            // Shared data requires consent
            if self.state.explicit_consent {
                SovereigntyCheckResult::allowed(data_category, operation)
            } else {
                SovereigntyCheckResult::denied(
                    data_category,
                    operation,
                    "Shared data requires explicit consent",
                )
            }
        } else {
            // Public data is always accessible
            SovereigntyCheckResult::allowed(data_category, operation)
        }
    }

    fn can_access(&self, data_category: DataCategory, requester: &WebID) -> bool {
        if self.state.boundary.is_sovereign(&data_category) {
            self.state.explicit_consent && requester == &self.owner_webid
        } else if self.state.boundary.is_shared(&data_category) {
            self.state.explicit_consent
        } else {
            self.state.boundary.is_public(&data_category)
        }
    }

    fn mark_acquisition_attempt(&mut self, details: &Value) {
        self.state.mark_acquisition_attempt();
        self.span_emitter
            .emit_sovereignty("acquisition_attempt", details.clone());
    }

    fn update_vc_investment(&mut self, vc_investment: f32) {
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

    fn is_compromised(&self) -> bool {
        self.state.is_compromised()
    }

    fn grant_consent(&mut self) {
        self.state.grant_consent();
        self.span_emitter.emit_sovereignty(
            "consent_granted",
            serde_json::json!({
                "consent": true
            }),
        );
    }

    fn revoke_consent(&mut self) {
        self.state.revoke_consent();
        self.span_emitter.emit_sovereignty(
            "consent_revoked",
            serde_json::json!({
                "consent": false
            }),
        );
    }

    fn owner_webid(&self) -> WebID {
        self.owner_webid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::sovereignty::SovereigntyOperation;

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
        assert!(!checker.can_access(&DataCategory::EpisodicMemory, &owner));

        // Grant consent
        checker.grant_consent();
        // Now accessible to owner
        assert!(checker.can_access(&DataCategory::EpisodicMemory, &owner));
        // But not to others
        assert!(!checker.can_access(
            &DataCategory::EpisodicMemory,
            &WebID::new()
        ));
    }

    #[test]
    fn test_can_access_public_data() {
        let checker = SovereigntyChecker::new(WebID::new());
        // Public data is always accessible
        assert!(checker.can_access(&DataCategory::HLexiconTerms, &WebID::new()));
    }

    #[test]
    fn test_acquisition_resistance() {
        let checker = SovereigntyChecker::new(WebID::new());
        // Default resistance is High, which prevents passive acquisition
        assert!(!checker.check_operation("acquisition", &DataCategory::SemanticMemory));
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

    #[test]
    fn test_sovereignty_port_check() {
        let owner = WebID::new();
        let mut checker = SovereigntyChecker::new(owner);

        // Sovereign data without consent should be denied
        let result = checker.check(
            DataCategory::EpisodicMemory,
            SovereigntyOperation::Read,
            &checker.owner_webid(),
        );
        assert!(!result.allowed);
        assert!(result.denial_reason.is_some());

        // Grant consent and retry
        let mut checker = SovereigntyChecker::new(owner);
        checker.grant_consent();
        let result = checker.check(
            DataCategory::EpisodicMemory,
            SovereigntyOperation::Read,
            &checker.owner_webid(),
        );
        assert!(result.allowed);
        assert!(result.denial_reason.is_none());
    }

    #[test]
    fn test_sovereignty_port_acquisition_denied() {
        let checker = SovereigntyChecker::new(WebID::new());
        let result = checker.check(
            DataCategory::SemanticMemory,
            SovereigntyOperation::Acquisition,
            &WebID::new(),
        );
        assert!(!result.allowed);
        assert!(result.denial_reason.is_some());
    }
}
