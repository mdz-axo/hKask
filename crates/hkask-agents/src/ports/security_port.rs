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
    fn acquire(&self, key: &str, tokens: f64) -> Result<(), ValidationError>;
    fn available(&self, key: &str) -> f64;
    fn reset(&self, key: &str);
}

/// Expiry Port — Expiry enforcement interface
pub trait ExpiryPort {
    fn calculate_expiry(&self, creation_time: i64) -> i64;
    fn is_expired(&self, expires_at: i64, current_time: i64) -> bool;
    fn validate_expiry(&self, expires_at: i64, current_time: i64) -> ValidationResult<()>;
    fn max_lifetime_secs(&self) -> u64;
}

/// Security Policy Port — Unified security interface
pub trait SecurityPolicyPort {
    fn validation(&self) -> &dyn InputValidationPort;
    fn rate_limit(&self) -> &dyn RateLimitPort;
    fn expiry(&self) -> &dyn ExpiryPort;
}
