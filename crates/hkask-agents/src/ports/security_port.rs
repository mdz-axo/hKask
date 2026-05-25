<<<<<<< HEAD
pub use crate::security::ValidationError;

pub trait RateLimitPort: Send + Sync {
=======
//! Security Ports — Hexagonal Architecture Interfaces

use crate::security::ValidationError;

/// Validation result type
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Input Validation Port — Schema-based validation interface
pub trait InputValidationPort {
    fn validate<T>(&self, input: &T) -> ValidationResult<()>;
}

/// Rate Limit Port — Rate checking interface
pub trait RateLimitPort {
>>>>>>> origin/main
    fn acquire(&self, key: &str, tokens: f64) -> Result<(), ValidationError>;
    fn available(&self, key: &str) -> f64;
    fn reset(&self, key: &str);
}
<<<<<<< HEAD
=======

/// Expiry Port — Expiry enforcement interface
pub trait ExpiryPort {
    fn calculate_expiry(&self, creation_time: i64) -> i64;
    fn is_expired(&self, expires_at: i64, current_time: i64) -> bool;
    fn validate_expiry(&self, expires_at: i64, current_time: i64) -> ValidationResult<()>;
    fn max_lifetime_secs(&self) -> u64;
}

/// Security Policy Port — Unified security interface
pub trait SecurityPolicyPort: Sized {
    fn rate_limit(&self) -> &dyn RateLimitPort;
    fn expiry(&self) -> &dyn ExpiryPort;
}

/// Sovereignty result type
pub type SovereigntyResult<T> = Result<T, SovereigntyError>;

/// Sovereignty error types
#[derive(Debug, thiserror::Error)]
pub enum SovereigntyError {
    #[error("Sovereignty boundary violation: {0}")]
    BoundaryViolation(String),

    #[error("Acquisition attempt detected: {0}")]
    AcquisitionAttempt(String),

    #[error("Kill zone active: {0}")]
    KillZoneActive(String),

    #[error("Consent required: {0}")]
    ConsentRequired(String),
}

/// Sovereignty Port — User sovereignty interface
pub trait SovereigntyPort {
    fn check_access(&self, user_webid: &str, category: &str) -> SovereigntyResult<bool>;
    fn grant_consent(&self, user_webid: &str, category: &str) -> SovereigntyResult<()>;
    fn revoke_consent(&self, user_webid: &str, category: &str) -> SovereigntyResult<()>;
    fn is_kill_zone_active(&self, user_webid: &str) -> bool;
}
>>>>>>> origin/main
