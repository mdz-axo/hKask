//! Sovereignty Service — Stub implementation

use hkask_types::UserSovereigntyState;

/// Sovereignty service stub
pub struct SovereigntyService;

impl SovereigntyService {
    pub fn new() -> Self {
        Self
    }

    pub fn get_state(&self) -> UserSovereigntyState {
        UserSovereigntyState::new()
    }
}

impl Default for SovereigntyService {
    fn default() -> Self {
        Self::new()
    }
}
