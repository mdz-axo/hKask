//! Capability-aware validator — OCAP enforcement point for template execution.
//!
//! FocusingAssumption FA-T3: Minimal stub — full implementation deferred until
//! template-registration-time capability validation is needed. At runtime, all
//! tool invocations pass through `GovernedTool` (hkask-cns::governed_tool) which
//! enforces OCAP boundaries. This validator covers the registration-time concern:
//! verifying that a template's declared capabilities match its OCAP grants.

use crate::ports::TemplateError;

/// Validates that a template's declared capabilities match its OCAP grants.
///
/// FocusingAssumption FA-T3: Minimal stub — always returns Ok(()) until
/// full capability-aware validation is implemented. At runtime, `GovernedTool`
/// in `hkask-cns::governed_tool` handles all OCAP enforcement for tool invocations.
pub struct CapabilityAwareValidator;

impl CapabilityAwareValidator {
    /// Create a new CapabilityAwareValidator.
    pub fn new() -> Self {
        Self
    }

    /// Validate that the template's declared capabilities match its OCAP grants.
    ///
    /// FocusingAssumption FA-T3: Always succeeds until full validation is implemented.
    /// Once implemented, this will verify that each declared capability in the template
    /// manifest has a corresponding grant in the agent's capability set.
    pub fn validate_capabilities(
        &self,
        _declared_capabilities: &[String],
        _granted_capabilities: &[String],
    ) -> Result<(), TemplateError> {
        // FocusingAssumption FA-T3: No-op until full implementation.
        // At runtime, GovernedTool handles all OCAP enforcement.
        Ok(())
    }
}

impl Default for CapabilityAwareValidator {
    fn default() -> Self {
        Self::new()
    }
}
