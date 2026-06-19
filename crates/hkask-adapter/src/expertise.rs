//! Expertise — semantic capability descriptor (P8 Semantic Grounding).
//!
//! Every trained adapter is grounded in an `Expertise`: a named, domain-scoped
//! capability with a manifest linking to its training source. This replaces
//! ad-hoc "skill name" strings with a provable semantic type.


use serde::{Deserialize, Serialize};

/// Domain categories recognized by the MDS (Minimal Domain Specification).
///
/// Each variant corresponds to a recognized domain of expertise.
/// The domain scopes what the adapter is trained to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MdsDomain {
    /// Solidity smart contract audit
    SolidityAudit,
    /// Rust code review
    RustReview,
    /// Template/jinja2 authoring
    TemplateAuthoring,
    /// General-purpose code generation
    CodeGeneration,
    /// Documentation generation
    Documentation,
    /// Test generation
    TestGeneration,
    /// Security analysis
    SecurityAnalysis,
}

impl MdsDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            MdsDomain::SolidityAudit => "solidity-audit",
            MdsDomain::RustReview => "rust-review",
            MdsDomain::TemplateAuthoring => "template-authoring",
            MdsDomain::CodeGeneration => "code-generation",
            MdsDomain::Documentation => "documentation",
            MdsDomain::TestGeneration => "test-generation",
            MdsDomain::SecurityAnalysis => "security-analysis",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "solidity-audit" => Some(MdsDomain::SolidityAudit),
            "rust-review" => Some(MdsDomain::RustReview),
            "template-authoring" => Some(MdsDomain::TemplateAuthoring),
            "code-generation" => Some(MdsDomain::CodeGeneration),
            "documentation" => Some(MdsDomain::Documentation),
            "test-generation" => Some(MdsDomain::TestGeneration),
            "security-analysis" => Some(MdsDomain::SecurityAnalysis),
            _ => None,
        }
    }
}

impl std::fmt::Display for MdsDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Training provenance — links an expertise to its training run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrainingProvenance {
    /// Identifier of the training run that produced this expertise
    pub training_run_id: String,
    /// URI to the training dataset or configuration
    pub training_source: String,
    /// Timestamp when training completed
    pub completed_at: String,
    /// Base model family the adapter was trained on (e.g. "llama-3.3-70b")
    pub base_model_family: String,
    /// Content hash of the training dataset (SHA-256)
    #[serde(default)]
    pub dataset_hash: Option<String>,
    /// Training metrics (loss, accuracy, etc.) as JSON
    #[serde(default)]
    pub training_metrics: serde_json::Value,
}

/// Expertise — a named, domain-scoped capability descriptor.
///
/// Grounds every trained adapter in a provable capability. Replaces
/// ad-hoc "skill name" strings with a semantic type carrying provenance.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Expertise {
    /// Human-readable name (e.g. "solidity-audit-v1")
    pub name: String,
    /// MDS domain category
    pub domain: MdsDomain,
    /// Arbitrary metadata describing what this expertise provides
    #[serde(default)]
    pub capability_manifest: serde_json::Value,
    /// Training provenance — links back to the training run
    pub training_source: TrainingProvenance,
}

impl Expertise {
    /// Create a new Expertise with validation.
    ///
    pub fn new(
        name: String,
        domain: MdsDomain,
        capability_manifest: serde_json::Value,
        training_source: TrainingProvenance,
    ) -> Result<Self, ExpertiseError> {
        if name.trim().is_empty() {
            return Err(ExpertiseError::EmptyName);
        }
        Ok(Self {
            name,
            domain,
            capability_manifest,
            training_source,
        })
    }
}

/// Errors for Expertise construction.
#[derive(Debug, thiserror::Error)]
pub enum ExpertiseError {
    #[error("Expertise name must not be empty")]
    EmptyName,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expertise_new_with_valid_data_succeeds() {
        let provenance = TrainingProvenance {
            training_run_id: "run-001".into(),
            training_source: "https://example.com/training".into(),
            completed_at: "2026-01-01T00:00:00Z".into(),
            base_model_family: "llama-3.3-70b".into(),
            dataset_hash: None,
            training_metrics: serde_json::json!({"loss": 0.01}),
        };
        let expertise = Expertise::new(
            "solidity-audit".into(),
            MdsDomain::SolidityAudit,
            serde_json::json!({"capabilities": ["reentrancy-detection", "overflow-check"]}),
            provenance,
        )
        .expect("expertise creation should succeed");

        assert_eq!(expertise.name, "solidity-audit");
        assert_eq!(expertise.domain, MdsDomain::SolidityAudit);
        assert_eq!(expertise.training_source.base_model_family, "llama-3.3-70b");
    }

    #[test]
    fn expertise_new_with_empty_name_fails() {
        let provenance = TrainingProvenance {
            training_run_id: "run-001".into(),
            training_source: "https://example.com/training".into(),
            completed_at: "2026-01-01T00:00:00Z".into(),
            base_model_family: "llama-3.3-70b".into(),
            dataset_hash: None,
            training_metrics: serde_json::Value::Null,
        };
        let result = Expertise::new(
            "".into(),
            MdsDomain::SolidityAudit,
            serde_json::Value::Null,
            provenance,
        );
        assert!(result.is_err());
    }

    #[test]
    fn expertise_new_with_whitespace_name_fails() {
        let provenance = TrainingProvenance {
            training_run_id: "run-001".into(),
            training_source: "https://example.com/training".into(),
            completed_at: "2026-01-01T00:00:00Z".into(),
            base_model_family: "llama-3.3-70b".into(),
            dataset_hash: None,
            training_metrics: serde_json::Value::Null,
        };
        let result = Expertise::new(
            "   ".into(),
            MdsDomain::SolidityAudit,
            serde_json::Value::Null,
            provenance,
        );
        assert!(result.is_err());
    }

    #[test]
    fn mds_domain_parse_recognized_domains() {
        assert_eq!(
            MdsDomain::parse("solidity-audit"),
            Some(MdsDomain::SolidityAudit)
        );
        assert_eq!(MdsDomain::parse("rust-review"), Some(MdsDomain::RustReview));
        assert_eq!(
            MdsDomain::parse("template-authoring"),
            Some(MdsDomain::TemplateAuthoring)
        );
    }

    #[test]
    fn mds_domain_parse_unrecognized_returns_none() {
        assert_eq!(MdsDomain::parse("not-a-domain"), None);
        assert_eq!(MdsDomain::parse(""), None);
    }

    #[test]
    fn mds_domain_as_str_round_trips() {
        for domain in &[
            MdsDomain::SolidityAudit,
            MdsDomain::RustReview,
            MdsDomain::TemplateAuthoring,
        ] {
            assert_eq!(
                MdsDomain::parse(domain.as_str()),
                Some(*domain),
                "round-trip failed for {domain:?}"
            );
        }
    }

    #[test]
    fn expertise_serde_round_trips() {
        let provenance = TrainingProvenance {
            training_run_id: "run-001".into(),
            training_source: "https://example.com/training".into(),
            completed_at: "2026-01-01T00:00:00Z".into(),
            base_model_family: "llama-3.3-70b".into(),
            dataset_hash: None,
            training_metrics: serde_json::json!({"loss": 0.01}),
        };
        let original = Expertise::new(
            "solidity-audit".into(),
            MdsDomain::SolidityAudit,
            serde_json::json!({"capabilities": ["reentrancy-detection"]}),
            provenance,
        )
        .expect("creation should succeed");

        let json = serde_json::to_string(&original).expect("serialization should succeed");
        let deserialized: Expertise =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(original.name, deserialized.name);
        assert_eq!(original.domain, deserialized.domain);
        assert_eq!(
            original.training_source.training_run_id,
            deserialized.training_source.training_run_id
        );
    }
}
