//! Sovereignty Port — Abstract sovereignty checking interface

use hkask_types::{CapabilityToken, CnsEvent, WebID};
use serde_json::Value;
use thiserror::Error;

/// Sovereignty error types
#[derive(Debug, Error)]
pub enum SovereigntyError {
    #[error("OCAP violation: {0}")]
    OCAPViolation(String),

    #[error("Sovereignty check failed: {0}")]
    CheckFailed(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("CNS emission error: {0}")]
    CNSEmission(String),

    #[error("Capability check error: {0}")]
    CapabilityCheck(String),
}

/// Result type for sovereignty operations
pub type SovereigntyResult<T> = Result<T, SovereigntyError>;

/// Sovereignty Port — Abstract sovereignty checking interface
pub trait SovereigntyPort {
    fn can_access(
        &self,
        category: &str,
        requester: &WebID,
        token: &CapabilityToken,
    ) -> SovereigntyResult<bool>;

    fn can_perform_operation(
        &self,
        operation: &str,
        category: &str,
        requester: &WebID,
        token: &CapabilityToken,
    ) -> SovereigntyResult<bool>;

    fn emit_event(&self, event: CnsEvent) -> SovereigntyResult<()>;

    fn get_state(&self, user_webid: &WebID) -> SovereigntyResult<UserSovereigntyState>;

    fn update_state(
        &self,
        user_webid: &WebID,
        state: &UserSovereigntyState,
    ) -> SovereigntyResult<()>;

    fn grant_consent(&self, user_webid: &WebID, token: &CapabilityToken) -> SovereigntyResult<()>;

    fn revoke_consent(&self, user_webid: &WebID, token: &CapabilityToken) -> SovereigntyResult<()>;

    fn mark_acquisition_attempt(
        &self,
        user_webid: &WebID,
        details: &Value,
    ) -> SovereigntyResult<()>;

    fn update_vc_investment(
        &self,
        user_webid: &WebID,
        vc_investment: f32,
    ) -> SovereigntyResult<()>;

    fn is_kill_zone_active(&self, user_webid: &WebID) -> SovereigntyResult<bool>;
}

pub use hkask_types::UserSovereigntyState;
