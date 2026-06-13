//! Capability-aware validator — OCAP enforcement point for template execution.
//!
//! FocusingAssumption FA-C2: At template registration time, capability validation
//! checks that the registering agent holds the required OCAP tokens for the template's
//! declared capability requirements. At runtime, OCAP enforcement is handled by
//! `GovernedTool` in `hkask-cns::governed_tool`. This validator covers the
//! registration-time concern: ensuring template capability declarations are consistent
//! with the agent's granted capabilities.
//!
//! REQ: DRIFT-D5 — existence justified by spec-code alignment, deletion test pending.

use crate::ports::Result;
use hkask_types::capability::DelegationToken;

/// Validates that an agent's capability tokens satisfy a template's requirements.
///
/// This is a registration-time gate. Runtime enforcement is delegated to
/// `GovernedTool` in `hkask-cns`.
pub struct CapabilityAwareValidator;

impl CapabilityAwareValidator {
    /// Create a new validator. Currently a passthrough — full implementation
    /// deferred until template registration requires OCAP validation.
    pub fn new() -> Self {
        Self
    }

    /// Validate that the given tokens satisfy the template's capability requirements.
    ///
    /// FocusingAssumption FA-C2a: Currently returns `Ok(())` for all inputs.
    /// Full implementation will check token attenuation chains against template
    /// capability declarations.
    pub fn validate_capabilities(
        &self,
        _template_id: &str,
        _required_capabilities: &[String],
        _held_tokens: &[DelegationToken],
    ) -> Result<()> {
        // Stub: passthrough until template capability requirements are defined.
        // Delegates to GovernedTool at runtime per FA-C2.
        Ok(())
    }
}

impl Default for CapabilityAwareValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: DRIFT-D5 — verify stub compiles and accepts any input
    #[test]
    fn validator_stub_accepts_any_input() {
        let validator = CapabilityAwareValidator::new();
        let result = validator.validate_capabilities("any", &[], &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn validator_default_is_passthrough() {
        let validator = CapabilityAwareValidator;
        let result = validator.validate_capabilities("test", &["cap.read".into()], &[]);
        assert!(result.is_ok());
    }
}
