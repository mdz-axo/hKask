//! Contract validator — lexicon-term enforcement at template registration.
//!
//! FocusingAssumption FA-C1: Minimal stub — full implementation deferred until
//! template registration requires term validation. The contract validator checks
//! that templates reference only hLexicon-registered terms in their declarations.
//!
//! At runtime, OCAP enforcement for tool invocation is handled by `GovernedTool`
//! in `hkask-cns::governed_tool`. This validator covers the registration-time concern:
//! ensuring template metadata is consistent with the hLexicon vocabulary.

use crate::ports::TemplateError;

/// Validates that template declarations reference only registered hLexicon terms.
///
/// FocusingAssumption FA-C1: Minimal stub — always returns Ok(()) until
/// full hLexicon term validation is implemented.
pub struct ContractValidator;

impl ContractValidator {
    /// Create a new ContractValidator.
    pub fn new() -> Self {
        Self
    }

    /// Validate that the given template's declared terms exist in the hLexicon.
    ///
    /// FocusingAssumption FA-C1: Always succeeds until full validation is implemented.
    /// Once implemented, this will check each term in the template's hlexicon_terms
    /// list against the loaded hLexicon vocabulary.
    pub fn validate_terms(
        &self,
        _template_terms: &[String],
        _hlexicon_terms: &[String],
    ) -> Result<(), TemplateError> {
        // FocusingAssumption FA-C1: No-op until full implementation.
        // Emit cns.spec span when invoked at runtime so variety counter tracks usage.
        Ok(())
    }
}

impl Default for ContractValidator {
    fn default() -> Self {
        Self::new()
    }
}
