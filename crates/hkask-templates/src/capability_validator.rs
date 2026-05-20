//! Contract Validator Integration with Okapi Capabilities
//!
//! Integrates template contract validation with Okapi capability-based security.

use crate::contract_validator::{OkapiCapabilities, RegistrationFrontmatter, ValidationError};
use hkask_ensemble::ports;
use hkask_types::TemplateType;
use std::collections::HashSet;
use thiserror::Error;

/// Enhanced contract validator with capability checking
pub struct CapabilityAwareValidator {
    capabilities: OkapiCapabilities,
    hlexicon_terms: HashSet<String>,
}

impl CapabilityAwareValidator {
    pub fn new(capabilities: OkapiCapabilities, hlexicon_terms: Vec<String>) -> Self {
        Self {
            capabilities,
            hlexicon_terms: hlexicon_terms.into_iter().collect(),
        }
    }

    /// Create from port capability provider
    pub async fn from_provider<CP>(
        provider: &CP,
        hlexicon_terms: Vec<String>,
    ) -> Result<Self, CP::Error>
    where
        CP: ports::CapabilityProvider,
        CP::Capabilities: Into<OkapiCapabilities>,
    {
        let caps = provider.get_capabilities().await?;
        let okapi_caps = caps.into();
        Ok(Self::new(okapi_caps, hlexicon_terms))
    }

    /// Validate template with capability awareness
    pub fn validate(
        &self,
        frontmatter: &RegistrationFrontmatter,
    ) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Validate Okapi requirements
        if let Some(reqs) = &frontmatter.requires_okapi {
            if frontmatter.template_type == TemplateType::Prompt && reqs.n_probs.is_none() {
                errors.push(ValidationError::MissingNProbs {
                    template_type: "Prompt".to_string(),
                });
            }

            if frontmatter.template_type == TemplateType::Process && reqs.grammar.is_none() {
                errors.push(ValidationError::MissingGrammar);
            }

            // Check capability compatibility
            if reqs.n_probs.is_some() && !self.capabilities.token_probs {
                errors.push(ValidationError::TokenProbsNotSupported {
                    runner_type: self.capabilities.runner_type.clone(),
                });
            }

            if reqs.grammar.is_some() && !self.capabilities.grammar_native {
                errors.push(ValidationError::GrammarNotSupported {
                    runner_type: self.capabilities.runner_type.clone(),
                });
            }

            if reqs.adapter.is_some() && !self.capabilities.lora_hot_swap {
                errors.push(ValidationError::AdapterNotSupported {
                    adapter: reqs.adapter.clone().unwrap(),
                    runner_type: self.capabilities.runner_type.clone(),
                });
            }
        }

        // Validate lexicon terms
        for term in &frontmatter.lexicon_terms {
            if !self.hlexicon_terms.contains(term) {
                errors.push(ValidationError::UnknownLexiconTerm {
                    term: term.clone(),
                    available_terms: self.hlexicon_terms.iter().cloned().collect(),
                });
            }
        }

        // Validate confidence config
        if let Some(conf) = &frontmatter.confidence
            && (conf.threshold < 0.0 || conf.threshold > 1.0)
        {
            errors.push(ValidationError::InvalidConfidenceThreshold {
                threshold: conf.threshold,
            });
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Convert hkask_ensemble OkapiCapabilities to contract_validator OkapiCapabilities
impl From<hkask_ensemble::OkapiCapabilities> for crate::contract_validator::OkapiCapabilities {
    fn from(caps: hkask_ensemble::OkapiCapabilities) -> Self {
        crate::contract_validator::OkapiCapabilities {
            runner_type: caps.runner_type,
            lora_hot_swap: caps.lora_hot_swap,
            token_probs: caps.token_probs,
            grammar_native: caps.grammar_native,
            advanced_sampling: caps.advanced_sampling,
        }
    }
}

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

