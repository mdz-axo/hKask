//! Contract validator — lexicon-term enforcement at template registration.
//!
//! FocusingAssumption FA-C1: The contract validator checks that templates reference
//! only hLexicon-registered terms in their declarations. When an hLexicon is set,
//! unknown terms are reported as errors or warnings depending on configuration.
//!
//! At runtime, OCAP enforcement for tool invocation is handled by `GovernedTool`
//! in `hkask-cns::governed_tool`. This validator covers the registration-time concern:
//! ensuring template metadata is consistent with the hLexicon vocabulary.

use crate::ports::TemplateError;
use hkask_types::HLexicon;

/// Validation mode — controls whether unknown terms cause hard or soft rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationMode {
    /// Unknown terms produce warnings but do not block registration.
    Warn,
    /// Unknown terms block registration with a `TemplateError::Validation`.
    Reject,
}

/// Validates that template declarations reference only registered hLexicon terms.
///
/// Holding an optional `HLexicon` reference. When set, `validate_terms()` checks
/// each declared term against the canonical vocabulary and returns unknown terms
/// according to the configured `ValidationMode`.
pub struct ContractValidator<'a> {
    hlexicon: Option<&'a HLexicon>,
    mode: ValidationMode,
}

impl<'a> ContractValidator<'a> {
    /// Create a new ContractValidator with no hLexicon (always passes).
    pub fn new() -> Self {
        Self {
            hlexicon: None,
            mode: ValidationMode::Warn,
        }
    }

    /// Create a ContractValidator with a loaded hLexicon for term validation.
    pub fn with_lexicon(hlexicon: &'a HLexicon) -> Self {
        Self {
            hlexicon: Some(hlexicon),
            mode: ValidationMode::Warn,
        }
    }

    /// Set the validation mode.
    pub fn with_mode(mut self, mode: ValidationMode) -> Self {
        self.mode = mode;
        self
    }

    /// Validate that the given template's declared terms exist in the hLexicon.
    ///
    /// When no hLexicon is set, always returns `Ok(())` (bootstrap/seed path).
    /// When an hLexicon is set:
    /// - `Warn` mode: unknown terms are logged as warnings via `tracing::warn!` but not rejected.
    /// - `Reject` mode: unknown terms cause `Err(TemplateError::Validation)`.
    ///
    /// Returns the set of unknown terms (for logging or caller inspection) alongside
    /// the result so callers can emit structured CNS spans.
    pub fn validate_terms(
        &self,
        template_id: &str,
        template_terms: &[String],
    ) -> (Result<(), TemplateError>, Vec<String>) {
        let unknown = match self.hlexicon {
            Some(lexicon) => lexicon.validate(template_terms),
            None => return (Ok(()), vec![]),
        };

        if unknown.is_empty() {
            return (Ok(()), vec![]);
        }

        let terms_str = unknown.join(", ");
        let msg = format!(
            "Template '{}' declares {} lexicon term(s) not in canonical vocabulary: {}",
            template_id,
            unknown.len(),
            terms_str
        );

        match self.mode {
            ValidationMode::Warn => {
                tracing::warn!(
                    target: "hkask.templates",
                    template_id = %template_id,
                    unknown_terms = ?unknown,
                    "Lexicon terms not in canonical vocabulary — template may diverge from spec."
                );
                (Ok(()), unknown)
            }
            ValidationMode::Reject => (Err(TemplateError::Validation(msg)), unknown),
        }
    }
}

impl Default for ContractValidator<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::{LexiconTerm, TemplateType};

    fn bootstrap_lexicon() -> HLexicon {
        let mut lexicon = HLexicon::new();
        lexicon.add(LexiconTerm::new(
            "query",
            TemplateType::WordAct,
            "Ask for information",
        ));
        lexicon.add(LexiconTerm::new(
            "sequence",
            TemplateType::FlowDef,
            "Linear ordering",
        ));
        lexicon
    }

    // P8: ContractValidator passes when no hLexicon is set (bootstrap path)
    #[test]
    fn validator_without_lexicon_always_passes() {
        let validator = ContractValidator::new();
        let (result, unknown) = validator.validate_terms("test-template", &["unknown".into()]);
        assert!(result.is_ok());
        assert!(unknown.is_empty());
    }

    // P8: ContractValidator with lexicon in Warn mode logs but does not reject unknown terms
    #[test]
    fn validator_warn_mode_reports_unknown_terms() {
        let lexicon = bootstrap_lexicon();
        let validator = ContractValidator::with_lexicon(&lexicon);
        let (result, unknown) =
            validator.validate_terms("test-template", &["query".into(), "nonexistent".into()]);
        assert!(result.is_ok());
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0], "nonexistent");
    }

    // P8: ContractValidator with lexicon in Reject mode blocks unknown terms
    #[test]
    fn validator_reject_mode_blocks_unknown_terms() {
        let lexicon = bootstrap_lexicon();
        let validator = ContractValidator::with_lexicon(&lexicon).with_mode(ValidationMode::Reject);
        let (result, unknown) = validator.validate_terms("test-template", &["nonexistent".into()]);
        assert!(result.is_err());
        assert_eq!(unknown.len(), 1);
    }

    // P8: ContractValidator passes when all terms are in the lexicon
    #[test]
    fn validator_accepts_known_terms() {
        let lexicon = bootstrap_lexicon();
        let validator = ContractValidator::with_lexicon(&lexicon).with_mode(ValidationMode::Reject);
        let (result, unknown) =
            validator.validate_terms("test-template", &["query".into(), "sequence".into()]);
        assert!(result.is_ok());
        assert!(unknown.is_empty());
    }

    // P8: ContractValidator Default builds a pass-through validator
    #[test]
    fn validator_default_is_passthrough() {
        let validator = ContractValidator::default();
        let (result, _) = validator.validate_terms("test", &["anything".into()]);
        assert!(result.is_ok());
    }
}
