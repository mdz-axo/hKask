//! Composition Error Types
//!
//! Unified error hierarchy for pragmatic composition operations.
//! Per architecture v0.21.0: Errors must encode recovery semantics.
//!
//! **Error Categories:**
//! - `Transient` — Retryable (network, timeout, temporary resource unavailability)
//! - `Permanent` — Not retryable (validation failure, capability denied, path traversal)
//! - `ResourceExhausted` — Energy/capacity limits exceeded
//! - `SecurityViolation` — OCAP/threat mitigation triggered
//!
//! **Recovery Semantics:**
//! - Transient errors trigger automatic retry (max 3, exponential backoff)
//! - Permanent errors fail immediately with clear diagnostic
//! - Resource exhausted errors emit CNS spans for monitoring
//! - Security violations logged and escalated

use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Composition error with recovery semantics
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum CompositionError {
    /// Transient error — retryable
    #[error("Transient error: {message}")]
    Transient { message: String, retry_count: u32 },

    /// Permanent error — not retryable
    #[error("Permanent error: {message}")]
    Permanent {
        message: String,
        diagnostic: Option<String>,
    },

    /// Resource exhausted — energy/capacity limits
    #[error("Resource exhausted: {resource_type} limit exceeded")]
    ResourceExhausted {
        resource_type: String,
        requested: u64,
        available: u64,
    },

    /// Security violation — OCAP/threat mitigation
    #[error("Security violation: {violation_type}")]
    SecurityViolation {
        violation_type: String,
        holder: Option<WebID>,
        details: Option<String>,
    },

    /// Recursion limit exceeded
    #[error("Recursion limit exceeded (max depth: {max})")]
    RecursionLimit { max: u8, actual: u8 },

    /// Capability denied
    #[error("Capability denied: {reason}")]
    CapabilityDenied {
        reason: String,
        resource: Option<String>,
        action: Option<String>,
    },

    /// Validation failure
    #[error("Validation failure: {reason}")]
    ValidationFailure {
        reason: String,
        field: Option<String>,
    },

    /// Path traversal attempt
    #[error("Path traversal attempt: {path}")]
    PathTraversal { path: String },

    /// Jinja2 injection attempt
    #[error("Jinja2 injection attempt: {pattern}")]
    Jinja2Injection { pattern: String },

    /// Energy budget exceeded — hard abort (security-critical)
    #[error("Energy budget exceeded: {manifest_id}/{capability_id} allocated {budget_allocated} tokens, consumed {budget_consumed} tokens ({}% over budget)",
        if *budget_allocated > 0 { ((budget_consumed.saturating_sub(*budget_allocated)) as f64 / *budget_allocated as f64 * 100.0) as u64 } else { 0 }
    )]
    EnergyBudgetHardAbort {
        manifest_id: String,
        capability_id: String,
        budget_allocated: u64,
        budget_consumed: u64,
    },

    /// Energy budget exceeded — escalate to curator (user-facing)
    #[error(
        "Energy budget exhausted: {manifest_id}/{capability_id} requires {additional_tokens} additional tokens to complete (allocated: {budget_allocated}, consumed: {budget_consumed})"
    )]
    EnergyBudgetEscalate {
        manifest_id: String,
        capability_id: String,
        budget_allocated: u64,
        budget_consumed: u64,
        additional_tokens: u64,
        remaining_work: String,
    },

    /// Cycle detected in composition graph
    #[error("Cycle detected in composition: {cycle_path:?}")]
    CycleDetected { cycle_path: Vec<String> },

    /// Stage communication failure (CSP channel error)
    #[error("Stage communication failed: {stage_name}")]
    StageCommunicationFailed { stage_name: String },

    /// Stage timeout
    #[error("Stage timeout: {stage_name} exceeded {timeout_ms}ms")]
    StageTimeout { stage_name: String, timeout_ms: u64 },
}

impl CompositionError {
    /// Check if error is retryable (transient)
    pub fn is_retryable(&self) -> bool {
        matches!(self, CompositionError::Transient { .. })
    }

    /// Check if error is permanent (not retryable)
    pub fn is_permanent(&self) -> bool {
        !matches!(self, CompositionError::Transient { .. })
    }

    /// Get retry count if transient
    pub fn retry_count(&self) -> Option<u32> {
        match self {
            CompositionError::Transient { retry_count, .. } => Some(*retry_count),
            _ => None,
        }
    }

    /// Create transient error
    pub fn transient(message: &str) -> Self {
        CompositionError::Transient {
            message: message.to_string(),
            retry_count: 0,
        }
    }

    /// Create permanent error
    pub fn permanent(message: &str, diagnostic: Option<&str>) -> Self {
        CompositionError::Permanent {
            message: message.to_string(),
            diagnostic: diagnostic.map(String::from),
        }
    }

    /// Create resource exhausted error
    pub fn resource_exhausted(resource_type: &str, requested: u64, available: u64) -> Self {
        CompositionError::ResourceExhausted {
            resource_type: resource_type.to_string(),
            requested,
            available,
        }
    }

    /// Create security violation error
    pub fn security_violation(
        violation_type: &str,
        holder: Option<WebID>,
        details: Option<&str>,
    ) -> Self {
        CompositionError::SecurityViolation {
            violation_type: violation_type.to_string(),
            holder,
            details: details.map(String::from),
        }
    }

    /// Increment retry count
    pub fn increment_retry(&self) -> Self {
        match self {
            CompositionError::Transient {
                message,
                retry_count,
            } => CompositionError::Transient {
                message: message.clone(),
                retry_count: retry_count + 1,
            },
            _ => self.clone(),
        }
    }

    /// Get error category name
    pub fn category(&self) -> &'static str {
        match self {
            CompositionError::Transient { .. } => "transient",
            CompositionError::Permanent { .. } => "permanent",
            CompositionError::ResourceExhausted { .. } => "resource_exhausted",
            CompositionError::SecurityViolation { .. } => "security_violation",
            CompositionError::RecursionLimit { .. } => "recursion_limit",
            CompositionError::CapabilityDenied { .. } => "capability_denied",
            CompositionError::ValidationFailure { .. } => "validation_failure",
            CompositionError::PathTraversal { .. } => "path_traversal",
            CompositionError::Jinja2Injection { .. } => "jinja2_injection",
            CompositionError::EnergyBudgetHardAbort { .. } => "energy_budget_hard_abort",
            CompositionError::EnergyBudgetEscalate { .. } => "energy_budget_escalate",
            CompositionError::CycleDetected { .. } => "cycle_detected",
            CompositionError::StageCommunicationFailed { .. } => "stage_communication_failed",
            CompositionError::StageTimeout { .. } => "stage_timeout",
        }
    }
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Base delay for exponential backoff (ms)
    pub base_delay_ms: u64,
    /// Maximum delay cap (ms)
    pub max_delay_ms: u64,
}

impl RetryConfig {
    pub fn new(max_retries: u32, base_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_retries,
            base_delay_ms,
            max_delay_ms,
        }
    }

    /// Calculate backoff delay for given attempt
    pub fn backoff_delay(&self, attempt: u32) -> u64 {
        let delay = self.base_delay_ms * 2u64.pow(attempt);
        delay.min(self.max_delay_ms)
    }

    /// Check if retry should continue
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 10000,
        }
    }
}


    #[test]
    fn test_permanent_error() {
        let error = CompositionError::permanent("validation failed", Some("missing field"));
        assert!(!error.is_retryable());
        assert_eq!(error.category(), "permanent");
    }

    #[test]
    fn test_retry_increment() {
        let error = CompositionError::transient("timeout");
        let retry1 = error.increment_retry();
        assert_eq!(retry1.retry_count(), Some(1));

        let retry2 = retry1.increment_retry();
        assert_eq!(retry2.retry_count(), Some(2));
    }

    #[test]
    fn test_retry_config_backoff() {
        let config = RetryConfig::default();
        assert_eq!(config.backoff_delay(0), 1000);
        assert_eq!(config.backoff_delay(1), 2000);
        assert_eq!(config.backoff_delay(2), 4000);
        assert_eq!(config.backoff_delay(3), 8000); // Exponential
        assert_eq!(config.backoff_delay(4), 10000); // Cap
    }

    #[test]
    fn test_retry_config_should_retry() {
        let config = RetryConfig::default();
        assert!(config.should_retry(0));
        assert!(config.should_retry(1));
        assert!(config.should_retry(2));
        assert!(!config.should_retry(3)); // max_retries = 3
    }

    #[test]
    fn test_resource_exhausted() {
        let error = CompositionError::resource_exhausted("energy", 1000, 500);
        assert!(!error.is_retryable());
        assert_eq!(error.category(), "resource_exhausted");
    }

    #[test]
    fn test_security_violation() {
        let error = CompositionError::security_violation("path_traversal", None, None);
        assert!(!error.is_retryable());
        assert_eq!(error.category(), "security_violation");
    }

    #[test]
    fn test_recursion_limit() {
        let error = CompositionError::RecursionLimit { max: 7, actual: 8 };
        assert!(!error.is_retryable());
        assert_eq!(error.category(), "recursion_limit");
    }

    #[test]
    fn test_capability_denied() {
        let error = CompositionError::CapabilityDenied {
            reason: "missing token".to_string(),
            resource: Some("template".to_string()),
            action: Some("render".to_string()),
        };
        assert!(!error.is_retryable());
        assert_eq!(error.category(), "capability_denied");
    }

    #[test]
    fn test_cycle_detected() {
        let error = CompositionError::CycleDetected {
            cycle_path: vec!["a".to_string(), "b".to_string(), "a".to_string()],
        };
        assert!(!error.is_retryable());
        assert_eq!(error.category(), "cycle_detected");
    }
}
