//! Sovereignty checking for agent pods
//!
//! Ensures agent operations respect user sovereignty boundaries.

use hkask_types::{
    DataCategory, SovereigntyCheckResult, SovereigntyOperation, SovereigntyPort,
    UserSovereigntyState, WebID,
};

/// Sovereignty checker for agent pods
pub(crate) struct SovereigntyChecker {
    state: UserSovereigntyState,
    owner_webid: WebID,
}

impl SovereigntyChecker {
    pub fn new(owner_webid: WebID) -> Self {
        Self {
            state: UserSovereigntyState::new(),
            owner_webid,
        }
    }

    pub fn with_state(state: UserSovereigntyState, owner_webid: WebID) -> Self {
        Self { state, owner_webid }
    }

    /// Check if operation respects sovereignty boundaries
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
        if self.state.boundary.is_sovereign(&data_category) {
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
            SovereigntyCheckResult::allowed(data_category, operation)
        }
    }

    pub fn can_access(&self, data_category: &DataCategory, requester: &WebID) -> bool {
        if self.state.boundary.is_sovereign(data_category) {
            return self.state.explicit_consent && requester == &self.owner_webid;
        }
        if self.state.boundary.is_shared(data_category) {
            return self.state.explicit_consent;
        }
        self.state.boundary.is_public(data_category)
    }

    pub fn check_operation(&self, operation: &str, data_category: &DataCategory) -> bool {
        if operation == "acquisition" {
            return !self
                .state
                .boundary
                .resistance
                .prevents_passive_acquisition();
        }
        self.can_access(data_category, &self.owner_webid)
    }

    pub fn mark_acquisition_attempt(&mut self) {
        self.state.mark_acquisition_attempt();
    }

    pub fn update_vc_investment(&mut self, vc_investment: f32) {
        self.state.update_vc_investment(vc_investment);
    }

    pub fn get_state(&self) -> &UserSovereigntyState {
        &self.state
    }

    pub fn grant_consent(&mut self) {
        self.state.grant_consent();
    }

    pub fn revoke_consent(&mut self) {
        self.state.revoke_consent();
    }

    pub fn is_compromised(&self) -> bool {
        self.state.is_compromised()
    }

    pub fn kill_zone_active(&self) -> bool {
        self.state.detector.kill_zone_active
    }

    pub fn owner_webid(&self) -> WebID {
        self.owner_webid
    }
}

impl SovereigntyPort for SovereigntyChecker {
    fn can_access(&self, data_category: &DataCategory, requester: &WebID) -> bool {
        SovereigntyChecker::can_access(self, data_category, requester)
    }
}
