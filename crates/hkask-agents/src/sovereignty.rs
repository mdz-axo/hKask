//! Sovereignty checking for agent pods
//!
//! Ensures agent operations respect user sovereignty boundaries.

use hkask_types::{DataCategory, SovereigntyPort, UserSovereigntyState, WebID};

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
