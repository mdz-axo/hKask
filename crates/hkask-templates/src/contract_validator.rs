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
use hkask_types::lexicon::HLexicon;

/// Validation mode — controls whether unknown terms cause hard or soft rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationMode {
    /// Unknown terms produce warnings but do not block registration.
    Warn,
    /// Unknown terms block registration with a `TemplateError::Validation`.
    Reject,
}

/// Validates that template declarations reference only registered hLexicon terms.
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

        match self.mode {
            ValidationMode::Warn => {
                tracing::warn!(
                    target: "hkask.templates",
                    template_id = %template_id,
                    unknown_terms = ?unknown,
                    "Lexicon terms not in canonical vocabulary"
                );
                (Ok(()), unknown)
            }
            ValidationMode::Reject => {
                let msg = format!(
                    "Template '{}' declares {} unknown terms: {}",
                    template_id,
                    unknown.len(),
                    unknown.join(", ")
                );
                (Err(TemplateError::Validation(msg)), unknown)
            }
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
        lexicon.add(LexiconTerm::new("query", TemplateType::WordAct, "Ask"));
        lexicon.add(LexiconTerm::new("sequence", TemplateType::FlowDef, "Order"));
        lexicon
    }

    // REQ: templates-contract-001 — ContractValidator without lexicon always passes
    #[test]
    fn validator_without_lexicon_always_passes() {
        let validator = ContractValidator::new();
        let (result, unknown) = validator.validate_terms("test", &["unknown".into()]);
        assert!(result.is_ok());
        assert!(unknown.is_empty());
    }

    // REQ: templates-contract-002 — ContractValidator in Warn mode reports unknown terms
    #[test]
    fn validator_warn_mode_reports_unknown_terms() {
        let lexicon = bootstrap_lexicon();
        let validator = ContractValidator::with_lexicon(&lexicon);
        let (result, unknown) = validator.validate_terms("test", &["query".into(), "bad".into()]);
        assert!(result.is_ok());
        assert_eq!(unknown.len(), 1);
    }

    // REQ: templates-contract-003 — ContractValidator in Reject mode blocks unknown terms
    #[test]
    fn validator_reject_mode_blocks_unknown_terms() {
        let lexicon = bootstrap_lexicon();
        let validator = ContractValidator::with_lexicon(&lexicon).with_mode(ValidationMode::Reject);
        let (result, _) = validator.validate_terms("test", &["nonexistent".into()]);
        assert!(result.is_err());
    }

    // REQ: templates-contract-004 — ContractValidator accepts known hLexicon terms
    #[test]
    fn validator_accepts_known_terms() {
        let lexicon = bootstrap_lexicon();
        let validator = ContractValidator::with_lexicon(&lexicon).with_mode(ValidationMode::Reject);
        let (result, unknown) =
            validator.validate_terms("test", &["query".into(), "sequence".into()]);
        assert!(result.is_ok());
        assert!(unknown.is_empty());
    }

    // REQ: templates-contract-005 — ContractValidator default is passthrough
    #[test]
    fn validator_default_is_passthrough() {
        let validator = ContractValidator::default();
        let (result, unknown) = validator.validate_terms("test", &["anything".into()]);
        assert!(result.is_ok());
        assert!(unknown.is_empty());
    }

    // ── Property-based tests (Wave 2) ─────────────────────────────────────

    use proptest::prelude::*;

    // REQ: TPL-001 — Manifest validation never panics (P4, P8)
    // For any lexicon and any term set, validate_terms never panics.
    proptest! {
        #[test]
        fn validator_never_panics(
            known_terms in prop::collection::vec(proptest::arbitrary::any::<String>(), 0..20),
            test_terms in prop::collection::vec(proptest::arbitrary::any::<String>(), 0..10),
            mode in proptest::sample::select(&[ValidationMode::Warn, ValidationMode::Reject]),
        ) {
            let mut lexicon = HLexicon::new();
            for term in &known_terms {
                if !term.is_empty() {
                    lexicon.add(LexiconTerm::new(term, TemplateType::WordAct, "test"));
                }
            }
            let validator = ContractValidator::with_lexicon(&lexicon).with_mode(mode);
            let result = std::panic::catch_unwind(|| {
                validator.validate_terms("test", &test_terms)
            });
            prop_assert!(result.is_ok(), "validator panicked");
        }
    }

    // REQ: TPL-002 — Known terms always accepted (P4, P8)
    // Terms registered in the lexicon are never reported as unknown.
    proptest! {
        #[test]
        fn known_terms_always_accepted(
            term in proptest::arbitrary::any::<String>(),
        ) {
            prop_assume!(!term.is_empty());
            let mut lexicon = HLexicon::new();
            lexicon.add(LexiconTerm::new(&term, TemplateType::WordAct, "test"));
            let validator = ContractValidator::with_lexicon(&lexicon).with_mode(ValidationMode::Reject);
            let (result, unknown) = validator.validate_terms("test", &[term]);
            prop_assert!(result.is_ok());
            prop_assert!(unknown.is_empty());
        }
    }
}
