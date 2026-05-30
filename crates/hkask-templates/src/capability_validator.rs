//! Capability-aware validation types (deprecated — merged into ContractValidator)
//!
//! This module retains auxiliary types that were previously used by the
//! now-removed `CapabilityAwareValidator`. That struct was near-identical
//! to `ContractValidator` and has been collapsed into it.
//!
//! **Migration guide:** Replace `CapabilityAwareValidator::new(caps, terms)`
//! with `ContractValidator::new(caps, terms)`. The `validate()` method
//! signature is identical.

use crate::contract_validator::ValidationError;
use thiserror::Error;

/// **Deprecated:** Use `ContractValidator` instead.
///
/// `CapabilityAwareValidator` was near-identical to `ContractValidator` and
/// has been collapsed into it. This type alias exists for backward compatibility.
#[deprecated(
    since = "0.21.0",
    note = "Use `ContractValidator` instead. CapabilityAwareValidator has been collapsed into ContractValidator."
)]
pub type CapabilityAwareValidator = crate::contract_validator::ContractValidator;

/// Validation result with capability information
#[derive(Debug, Clone)]
pub struct ValidationWithCapabilities {
    pub template_id: String,
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub required_capabilities: Vec<String>,
    pub available_capabilities: Vec<String>,
}

impl ValidationWithCapabilities {
    pub fn new(template_id: String, valid: bool) -> Self {
        Self {
            template_id,
            valid,
            errors: Vec::new(),
            required_capabilities: Vec::new(),
            available_capabilities: Vec::new(),
        }
    }

    pub fn with_errors(mut self, errors: Vec<ValidationError>) -> Self {
        self.errors = errors;
        self.valid = self.errors.is_empty();
        self
    }

    pub fn with_capabilities(mut self, required: Vec<String>, available: Vec<String>) -> Self {
        self.required_capabilities = required;
        self.available_capabilities = available;
        self
    }
}

/// Error type for capability-aware validation
#[derive(Debug, Error)]
pub enum CapabilityAwareValidationError {
    #[error("Capability fetch failed: {0}")]
    CapabilityFetchError(String),

    #[error("Validation failed: {0:?}")]
    ValidationFailed(Vec<ValidationError>),
}
