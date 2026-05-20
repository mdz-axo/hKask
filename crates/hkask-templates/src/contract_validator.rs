//! Template Contract Validator for Okapi capabilities
//!
//! Validates template contracts at registration time with actionable error messages.

use hkask_types::TemplateType;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use thiserror::Error;

/// Template registration frontmatter (for validation)
#[derive(Debug, Deserialize, Clone)]
pub struct RegistrationFrontmatter {
    pub template_type: TemplateType,
    pub domain: String,
    pub requires_okapi: Option<OkapiRequirements>,
    pub confidence: Option<ConfidenceConfig>,
    pub lexicon_terms: Vec<String>,
    pub contract: Option<ContractSchema>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OkapiRequirements {
    pub n_probs: Option<i32>,
    pub grammar: Option<String>,
    pub adapter: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConfidenceConfig {
    pub threshold: f64,
    pub escalate_to_model: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ContractSchema {
    pub input: Option<JsonValue>,
    pub output: Option<JsonValue>,
}

/// Okapi capabilities (from /api/engine/status)
#[derive(Debug, Deserialize, Clone)]
pub struct OkapiCapabilities {
    pub runner_type: String,
    pub lora_hot_swap: bool,
    pub token_probs: bool,
    pub grammar_native: bool,
    pub advanced_sampling: bool,
}

/// Validation error with actionable message
#[derive(Debug, Clone, Error)]
pub enum ValidationError {
    #[error("Template type '{template_type}' requires 'n_probs' in requires_okapi, but it was not specified. Add 'n_probs: 5' to enable token probability-based confidence routing.")]
    MissingNProbs { template_type: String },

    #[error("Process template requires 'grammar' constraint in requires_okapi, but it was not specified. Add 'grammar: \"path/to/constraint.gbnf\"' to enable grammar-constrained decoding.")]
    MissingGrammar,

    #[error("Template requires LoRA adapter '{adapter}', but Okapi's lora_hot_swap capability is disabled. Okapi runner type: {runner_type}. Use an ollamarunner-compatible model or remove the adapter requirement.")]
    AdapterNotSupported { adapter: String, runner_type: String },

    #[error("Template requires 'n_probs' but Okapi's token_probs capability is disabled. Okapi runner type: {runner_type}. Token probabilities are only available with ollamarunner.")]
    TokenProbsNotSupported { runner_type: String },

    #[error("Template requires 'grammar' but Okapi's grammar_native capability is disabled. Okapi runner type: {runner_type}. Grammar constraints are only available with ollamarunner.")]
    GrammarNotSupported { runner_type: String },

    #[error("Invalid lexicon term '{term}' - not found in hLexicon. Available terms: {available_terms:?}. Use only canonical hLexicon terms to ensure consistent LLM interpretation.")]
    UnknownLexiconTerm { term: String, available_terms: Vec<String> },

    #[error("Confidence threshold {threshold} is outside valid range [0.0, 1.0]. Use a value between 0.0 and 1.0 inclusive.")]
    InvalidConfidenceThreshold { threshold: f64 },

    #[error("Failed to fetch Okapi capabilities: {0}")]
    CapabilityFetchError(String),
}

/// Contract validator for template registration
pub struct ContractValidator {
    okapi_capabilities: OkapiCapabilities,
    hlexicon_terms: HashSet<String>,
}

impl ContractValidator {
    pub fn new(okapi_capabilities: OkapiCapabilities, hlexicon_terms: Vec<String>) -> Self {
        Self {
            okapi_capabilities,
            hlexicon_terms: hlexicon_terms.into_iter().collect(),
        }
    }

    /// Validate template frontmatter at registration time
    pub fn validate(&self, frontmatter: &RegistrationFrontmatter) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        if let Some(reqs) = &frontmatter.requires_okapi {
            if frontmatter.template_type == TemplateType::Prompt && reqs.n_probs.is_none() {
                errors.push(ValidationError::MissingNProbs {
                    template_type: "Prompt".to_string(),
                });
            }

            if frontmatter.template_type == TemplateType::Process && reqs.grammar.is_none() {
                errors.push(ValidationError::MissingGrammar);
            }

            if reqs.n_probs.is_some() && !self.okapi_capabilities.token_probs {
                errors.push(ValidationError::TokenProbsNotSupported {
                    runner_type: self.okapi_capabilities.runner_type.clone(),
                });
            }

            if reqs.grammar.is_some() && !self.okapi_capabilities.grammar_native {
                errors.push(ValidationError::GrammarNotSupported {
                    runner_type: self.okapi_capabilities.runner_type.clone(),
                });
            }

            if reqs.adapter.is_some() && !self.okapi_capabilities.lora_hot_swap {
                errors.push(ValidationError::AdapterNotSupported {
                    adapter: reqs.adapter.clone().unwrap(),
                    runner_type: self.okapi_capabilities.runner_type.clone(),
                });
            }
        }

        for term in &frontmatter.lexicon_terms {
            if !self.hlexicon_terms.contains(term) {
                errors.push(ValidationError::UnknownLexiconTerm {
                    term: term.clone(),
                    available_terms: self.hlexicon_terms.iter().cloned().collect(),
                });
            }
        }

        if let Some(conf) = &frontmatter.confidence {
            if conf.threshold < 0.0 || conf.threshold > 1.0 {
                errors.push(ValidationError::InvalidConfidenceThreshold {
                    threshold: conf.threshold,
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Fetch Okapi capabilities at hKask startup
/// TODO: Implement with proper HTTP client when reqwest is added
pub async fn fetch_okapi_capabilities(_okapi_base_url: &str) -> Result<OkapiCapabilities, ValidatorError> {
    // Placeholder - returns default capabilities
    Ok(OkapiCapabilities {
        runner_type: "ollamarunner".to_string(),
        lora_hot_swap: true,
        token_probs: true,
        grammar_native: true,
        advanced_sampling: true,
    })
}

#[derive(Debug, Error)]
pub enum ValidatorError {
    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("JSON parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_capabilities() -> OkapiCapabilities {
        OkapiCapabilities {
            runner_type: "ollamarunner".to_string(),
            lora_hot_swap: true,
            token_probs: true,
            grammar_native: true,
            advanced_sampling: true,
        }
    }

    fn create_test_lexicon() -> Vec<String> {
        vec![
            "classify".to_string(),
            "discriminate".to_string(),
            "route".to_string(),
            "recognize".to_string(),
        ]
    }

    #[test]
    fn test_validator_valid_template() {
        let capabilities = create_test_capabilities();
        let lexicon = create_test_lexicon();
        let validator = ContractValidator::new(capabilities, lexicon);

        let frontmatter = RegistrationFrontmatter {
            template_type: TemplateType::Prompt,
            domain: "WordAct".to_string(),
            requires_okapi: Some(OkapiRequirements {
                n_probs: Some(5),
                grammar: None,
                adapter: None,
            }),
            confidence: Some(ConfidenceConfig {
                threshold: 0.75,
                escalate_to_model: "qwen3:70b".to_string(),
            }),
            lexicon_terms: vec!["classify".to_string()],
            contract: None,
        };

        let result = validator.validate(&frontmatter);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validator_missing_n_probs() {
        let capabilities = create_test_capabilities();
        let lexicon = create_test_lexicon();
        let validator = ContractValidator::new(capabilities, lexicon);

        let frontmatter = RegistrationFrontmatter {
            template_type: TemplateType::Prompt,
            domain: "WordAct".to_string(),
            requires_okapi: Some(OkapiRequirements {
                n_probs: None,
                grammar: None,
                adapter: None,
            }),
            confidence: None,
            lexicon_terms: vec![],
            contract: None,
        };

        let result = validator.validate(&frontmatter);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(e, ValidationError::MissingNProbs { .. })));
    }

    #[test]
    fn test_validator_missing_grammar() {
        let capabilities = create_test_capabilities();
        let lexicon = create_test_lexicon();
        let validator = ContractValidator::new(capabilities, lexicon);

        let frontmatter = RegistrationFrontmatter {
            template_type: TemplateType::Process,
            domain: "FlowDef".to_string(),
            requires_okapi: Some(OkapiRequirements {
                n_probs: None,
                grammar: None,
                adapter: None,
            }),
            confidence: None,
            lexicon_terms: vec![],
            contract: None,
        };

        let result = validator.validate(&frontmatter);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| matches!(e, ValidationError::MissingGrammar)));
    }

    #[test]
    fn test_validator_unknown_lexicon_term() {
        let capabilities = create_test_capabilities();
        let lexicon = create_test_lexicon();
        let validator = ContractValidator::new(capabilities, lexicon);

        let frontmatter = RegistrationFrontmatter {
            template_type: TemplateType::Prompt,
            domain: "WordAct".to_string(),
            requires_okapi: Some(OkapiRequirements {
                n_probs: Some(5),
                grammar: None,
                adapter: None,
            }),
            confidence: None,
            lexicon_terms: vec!["unknown_term".to_string()],
            contract: None,
        };

        let result = validator.validate(&frontmatter);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(e, ValidationError::UnknownLexiconTerm { .. })));
    }

    #[test]
    fn test_validator_invalid_confidence_threshold() {
        let capabilities = create_test_capabilities();
        let lexicon = create_test_lexicon();
        let validator = ContractValidator::new(capabilities, lexicon);

        let frontmatter = RegistrationFrontmatter {
            template_type: TemplateType::Prompt,
            domain: "WordAct".to_string(),
            requires_okapi: Some(OkapiRequirements {
                n_probs: Some(5),
                grammar: None,
                adapter: None,
            }),
            confidence: Some(ConfidenceConfig {
                threshold: 1.5,
                escalate_to_model: "qwen3:70b".to_string(),
            }),
            lexicon_terms: vec!["classify".to_string()],
            contract: None,
        };

        let result = validator.validate(&frontmatter);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| matches!(e, ValidationError::InvalidConfidenceThreshold { .. })));
    }

    #[test]
    fn test_validator_token_probs_not_supported() {
        let mut capabilities = create_test_capabilities();
        capabilities.token_probs = false;
        capabilities.runner_type = "llamarunner".to_string();
        
        let lexicon = create_test_lexicon();
        let validator = ContractValidator::new(capabilities, lexicon);

        let frontmatter = RegistrationFrontmatter {
            template_type: TemplateType::Prompt,
            domain: "WordAct".to_string(),
            requires_okapi: Some(OkapiRequirements {
                n_probs: Some(5),
                grammar: None,
                adapter: None,
            }),
            confidence: None,
            lexicon_terms: vec![],
            contract: None,
        };

        let result = validator.validate(&frontmatter);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| matches!(e, ValidationError::TokenProbsNotSupported { .. })));
    }
}
