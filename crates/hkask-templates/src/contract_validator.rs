//! Contract validator — declaration consistency enforcement at template registration.
//!
//! At runtime, OCAP enforcement for tool invocation is handled by `GovernedTool`
//! in `hkask-cns::governed_tool`. This validator covers the registration-time concern:
//! ensuring template metadata is consistent.

use crate::ports::TemplateError;

/// Validation mode — controls whether declaration failures cause hard or soft rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationMode {
    /// Failures produce warnings but do not block registration.
    Warn,
    /// Failures block registration with a `TemplateError::Validation`.
    Reject,
}

/// Validates template declarations for consistency.
pub struct ContractValidator {
    mode: ValidationMode,
}

impl ContractValidator {
    /// Create a new ContractValidator with default passthrough behavior.
    ///
    /// REQ: P3-tpl-contract-validator-new
    /// [P3] Motivating: Generative Space — passthrough validator for unconstrained registration
    /// [P4] Constraining: Clear Boundaries — default Warn mode allows registration
    /// post: returns ContractValidator with Warn mode
    pub fn new() -> Self {
        Self {
            mode: ValidationMode::Warn,
        }
    }

    /// Set the validation mode.
    ///
    /// REQ: P3-tpl-contract-validator-with-mode
    /// [P3] Motivating: Generative Space — configures validation strictness
    /// post: returns Self with mode updated (builder pattern)
    pub fn with_mode(mut self, mode: ValidationMode) -> Self {
        self.mode = mode;
        self
    }

    /// Validate that the given template's declared terms meet consistency rules.
    ///
    /// REQ: P3-tpl-contract-validator-validate-terms
    /// [P3] Motivating: Generative Space — declaration consistency gate
    /// pre:  template_id is non-empty
    /// post: returns (Ok(()), vec![]) in Warn mode
    /// post: returns (Ok(()), vec![]) in Reject mode if terms valid
    #[allow(dead_code)]
    pub fn validate_terms(
        &self,
        _template_id: &str,
        _template_terms: &[String],
    ) -> (Result<(), TemplateError>, Vec<String>) {
        (Ok(()), vec![])
    }
}

impl Default for ContractValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: P3-tpl-test-contract-validator-passthrough — ContractValidator always passes
    // [P3] Motivating: Generative Space — validates passthrough mode
    #[test]
    fn validator_always_passes() {
        let validator = ContractValidator::new();
        let (result, unknown) = validator.validate_terms("test", &["anything".into()]);
        assert!(result.is_ok());
        assert!(unknown.is_empty());
    }

    // REQ: P3-tpl-test-contract-validator-default-passthrough — ContractValidator default is passthrough
    // [P3] Motivating: Generative Space — validates Default impl is passthrough
    #[test]
    fn validator_default_is_passthrough() {
        let validator = ContractValidator::default();
        let (result, unknown) = validator.validate_terms("test", &["anything".into()]);
        assert!(result.is_ok());
        assert!(unknown.is_empty());
    }
}
