//! Sovereignty checking for agent pods
//!
//! Ensures agent operations respect user sovereignty boundaries.
//! Integrates with CNS for sovereignty event emission.

use hkask_cns::spans::SpanEmitter;
use hkask_types::event::Span;
use hkask_types::{
    DataCategory, Phase, SovereigntyCheckResult, SovereigntyOperation, SovereigntyPort,
    UserSovereigntyState, WebID,
};
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

    /// Check if operation respects sovereignty boundaries
    ///
    /// # Arguments
    /// * `data_category` — Category of data being accessed
    /// * `operation` — Type of operation (read, write, acquisition, composition)
    /// * `requester` — WebID of the requesting agent
    ///
    /// # Returns
    /// * `SovereigntyCheckResult` — Check result with allowance and reason
    pub fn check(
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
        let result = if self.state.boundary.is_sovereign(&data_category) {
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
        };

        // Emit Regulate-phase span recording the access decision
        self.span_emitter.emit_with_phase(
            Span::sovereignty("regulate.access_check"),
            Phase::Regulate,
            serde_json::json!({
                "allowed": result.allowed,
                "denial_reason": result.denial_reason,
                "data_category": format!("{:?}", result.data_category),
                "operation": format!("{:?}", result.operation),
                "requester": format!("{}", requester),
            }),
        );

        result
    }

    /// Check if data category is accessible by requester
    pub fn can_access(&self, data_category: &DataCategory, requester: &WebID) -> bool {
        // Sovereign data requires explicit consent and owner
        if self.state.boundary.is_sovereign(data_category) {
            return self.state.explicit_consent && requester == &self.owner_webid;
        }

        // Shared data requires consent
        if self.state.boundary.is_shared(data_category) {
            return self.state.explicit_consent;
        }

        // Public data is always accessible
        self.state.boundary.is_public(data_category)
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
        self.span_emitter.emit_with_phase(
            Span::sovereignty("acquisition_attempt"),
            Phase::Observe,
            details.clone(),
        );
    }

    /// Update VC investment and check for kill zone
    pub fn update_vc_investment(&mut self, vc_investment: f32) {
        self.state.update_vc_investment(vc_investment);

        if self.state.is_compromised() {
            self.span_emitter.emit_with_phase(
                Span::sovereignty("alert.killzone"),
                Phase::Observe,
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
        self.span_emitter.emit_with_phase(
            Span::sovereignty("consent_granted"),
            Phase::Observe,
            serde_json::json!({
                "consent": true
            }),
        );
    }

    /// Revoke explicit consent
    pub fn revoke_consent(&mut self) {
        self.state.revoke_consent();
        self.span_emitter.emit_with_phase(
            Span::sovereignty("consent_revoked"),
            Phase::Observe,
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

    /// Get owner WebID
    pub fn owner_webid(&self) -> WebID {
        self.owner_webid
    }
}

/// Implement SovereigntyPort trait for dependency inversion
impl SovereigntyPort for SovereigntyChecker {
    fn can_access(&self, data_category: &DataCategory, requester: &WebID) -> bool {
        SovereigntyChecker::can_access(self, data_category, requester)
    }
}
