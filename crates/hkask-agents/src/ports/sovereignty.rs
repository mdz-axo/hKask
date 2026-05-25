<<<<<<< HEAD
//! Sovereignty Port — Hexagonal interface for sovereignty checking
//!
//! Defines the trait for sovereignty verification operations.
//! Implementations enforce user sovereignty boundaries at pod level.

use hkask_types::{DataCategory, WebID};
use serde_json::Value;

/// Sovereignty operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SovereigntyOperation {
    /// Data read operation
    Read,
    /// Data write operation
    Write,
    /// Data acquisition (passive collection)
    Acquisition,
    /// Data composition (combining multiple sources)
    Composition,
}

/// Sovereignty check result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SovereigntyCheckResult {
    /// Whether operation is allowed
    pub allowed: bool,
    /// Reason for denial (if any)
    pub denial_reason: Option<String>,
    /// Data category being accessed
    pub data_category: DataCategory,
    /// Operation type
    pub operation: SovereigntyOperation,
}

impl SovereigntyCheckResult {
    pub fn allowed(data_category: DataCategory, operation: SovereigntyOperation) -> Self {
        Self {
            allowed: true,
            denial_reason: None,
            data_category,
            operation,
        }
    }

    pub fn denied(
        data_category: DataCategory,
        operation: SovereigntyOperation,
        reason: &str,
    ) -> Self {
        Self {
            allowed: false,
            denial_reason: Some(reason.to_string()),
            data_category,
            operation,
        }
    }
}

/// Sovereignty Port — Interface for sovereignty enforcement
///
/// This trait defines the hexagonal port for sovereignty checking.
/// Implementations enforce user sovereignty boundaries and emit
/// CNS events for sovereignty violations.
///
/// # Example
///
/// ```rust,no_run
/// use hkask_agents::ports::sovereignty::{SovereigntyPort, SovereigntyOperation};
/// use hkask_types::{DataCategory, WebID};
///
/// fn check_sovereignty<P: SovereigntyPort>(
///     port: &P,
///     data: DataCategory,
///     requester: &WebID,
/// ) -> bool {
///     port.check(data, SovereigntyOperation::Read, requester).allowed
/// }
/// ```
pub trait SovereigntyPort {
    /// Check if operation respects sovereignty boundaries
    ///
    /// # Arguments
    /// * `data_category` — Category of data being accessed
    /// * `operation` — Type of operation (read, write, acquisition, composition)
    /// * `requester` — WebID of the requesting agent
    ///
    /// # Returns
    /// * `SovereigntyCheckResult` — Check result with allowance and reason
    fn check(
        &self,
        data_category: DataCategory,
        operation: SovereigntyOperation,
        requester: &WebID,
    ) -> SovereigntyCheckResult;

    /// Check if data category is accessible by requester
    ///
    /// # Arguments
    /// * `data_category` — Category of data
    /// * `requester` — WebID of the requester
    ///
    /// # Returns
    /// * `true` — Data is accessible
    /// * `false` — Access denied
    fn can_access(&self, data_category: DataCategory, requester: &WebID) -> bool;

    /// Mark acquisition attempt for monitoring
    ///
    /// # Arguments
    /// * `details` — Acquisition attempt details
    fn mark_acquisition_attempt(&mut self, details: &Value);

    /// Update VC investment level
    ///
    /// # Arguments
    /// * `vc_investment` — Current VC investment (0.0 to 1.0)
    fn update_vc_investment(&mut self, vc_investment: f32);

    /// Check if sovereignty is compromised (kill zone active)
    ///
    /// # Returns
    /// * `true` — Sovereignty compromised, kill zone active
    /// * `false` — Sovereignty intact
    fn is_compromised(&self) -> bool;

    /// Grant explicit consent for data sharing
    fn grant_consent(&mut self);

    /// Revoke explicit consent
    fn revoke_consent(&mut self);

    /// Get owner WebID
    fn owner_webid(&self) -> WebID;
}
=======
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
>>>>>>> origin/main
