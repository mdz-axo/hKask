//! Expertise — a semantic capability descriptor that grounds every trained
//! adapter in a provable skill domain.
//!
//! Maps to the RDF model:
//!   :Expertise rdf:type :CapabilityDescriptor .
//!   :Expertise :groundedIn :TrainingProvenance .
//!   :Expertise :namedBy xsd:string .

use serde::{Deserialize, Serialize};
use std::fmt;

/// Domain categories for expertise — maps to hKask skill domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MdsDomain {
    /// Code review and software engineering.
    CodeReview,
    /// Smart contract auditing (Solidity).
    SolidityAudit,
    /// Documentation authoring and curation.
    Documentation,
    /// Constraint classification and enforcement.
    ConstraintAnalysis,
    /// Specification authoring and decomposition.
    Specification,
    /// Agent communication and improvisation.
    Communication,
    /// System diagnosis and debugging.
    Diagnosis,
    /// Architecture review and improvement.
    ArchitectureReview,
    /// General-purpose reasoning.
    General,
}

impl MdsDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            MdsDomain::CodeReview => "code-review",
            MdsDomain::SolidityAudit => "solidity-audit",
            MdsDomain::Documentation => "documentation",
            MdsDomain::ConstraintAnalysis => "constraint-analysis",
            MdsDomain::Specification => "specification",
            MdsDomain::Communication => "communication",
            MdsDomain::Diagnosis => "diagnosis",
            MdsDomain::ArchitectureReview => "architecture-review",
            MdsDomain::General => "general",
        }
    }
}

/// Training provenance — how and from what source this expertise was derived.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingProvenance {
    /// Path or URI to the training source (SKILL.md, QA dataset, etc.).
    pub source_uri: String,
    /// Training mode used (expertise, skill, contrastive, hybrid).
    pub training_mode: String,
    /// Base model the adapter was trained on.
    pub base_model: String,
    /// Training job ID for traceability.
    pub training_job_id: Option<String>,
    /// Hash of the training dataset.
    pub dataset_hash: Option<String>,
}

/// Expertise — a named, domain-scoped capability descriptor.
///
/// Every trained adapter is linked to an Expertise. This grounds the adapter
/// in a provable capability rather than an ad-hoc name string.
///
/// REQ: P8-adt-expertise-definition
/// [P8] Semantic Grounding — expertise is a named, domain-scoped capability descriptor
/// pre:  name is non-empty
/// post: Expertise carries a capability_manifest linking to the training source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Expertise {
    /// Unique human-readable name (e.g., "constraint-forces", "solidity-audit").
    pub name: String,
    /// Domain this expertise applies to.
    pub domain: MdsDomain,
    /// Training mode — how the adapter was produced.
    pub training_mode: String,
    /// Provenance chain — links back to training source.
    pub provenance: TrainingProvenance,
    /// Additional capability metadata (SKILL.md summary, evaluation scores, etc.).
    #[serde(default)]
    pub capability_manifest: serde_json::Value,
}

impl Expertise {
    /// Create a new Expertise descriptor.
    ///
    /// Returns None if name is empty (P8 contract).
    pub fn new(
        name: &str,
        domain: MdsDomain,
        training_mode: &str,
        provenance: TrainingProvenance,
    ) -> Option<Self> {
        if name.trim().is_empty() {
            return None;
        }
        Some(Self {
            name: name.to_string(),
            domain,
            training_mode: training_mode.to_string(),
            provenance,
            capability_manifest: serde_json::Value::Null,
        })
    }

    /// Check whether this expertise is of a procedural skill type.
    pub fn is_skill(&self) -> bool {
        self.training_mode == "decomposition_trace" || self.training_mode == "contrastive_trace"
    }

    /// Check whether this expertise is of a factual knowledge type.
    pub fn is_expertise(&self) -> bool {
        self.training_mode == "expertise"
    }
}

impl fmt::Display for Expertise {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [{} domain, {} mode]",
            self.name,
            self.domain.as_str(),
            self.training_mode
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expertise_new_rejects_empty_name() {
        let provenance = TrainingProvenance {
            source_uri: "skills/constraint-forces/SKILL.md".into(),
            training_mode: "decomposition_trace".into(),
            base_model: "Qwen/Qwen2.5-7B".into(),
            training_job_id: None,
            dataset_hash: None,
        };
        assert!(
            Expertise::new(
                "",
                MdsDomain::ConstraintAnalysis,
                "decomposition_trace",
                provenance
            )
            .is_none()
        );
        assert!(
            Expertise::new(
                "  ",
                MdsDomain::ConstraintAnalysis,
                "decomposition_trace",
                TrainingProvenance {
                    source_uri: "test".into(),
                    training_mode: "decomposition_trace".into(),
                    base_model: "test".into(),
                    training_job_id: None,
                    dataset_hash: None,
                }
            )
            .is_none()
        );
    }

    #[test]
    fn expertise_new_accepts_valid_name() {
        let provenance = TrainingProvenance {
            source_uri: "skills/constraint-forces/SKILL.md".into(),
            training_mode: "decomposition_trace".into(),
            base_model: "Qwen/Qwen2.5-7B".into(),
            training_job_id: Some("job-123".into()),
            dataset_hash: None,
        };
        let e = Expertise::new(
            "constraint-forces",
            MdsDomain::ConstraintAnalysis,
            "decomposition_trace",
            provenance,
        )
        .unwrap();
        assert_eq!(e.name, "constraint-forces");
        assert!(e.is_skill());
        assert!(!e.is_expertise());
    }

    #[test]
    fn expertise_display_format() {
        let provenance = TrainingProvenance {
            source_uri: "qa/rust-patterns.jsonl".into(),
            training_mode: "expertise".into(),
            base_model: "Qwen/Qwen2.5-7B".into(),
            training_job_id: None,
            dataset_hash: None,
        };
        let e = Expertise::new(
            "rust-idioms",
            MdsDomain::CodeReview,
            "expertise",
            provenance,
        )
        .unwrap();
        let display = format!("{}", e);
        assert!(display.contains("rust-idioms"));
        assert!(display.contains("code-review"));
        assert!(e.is_expertise());
    }
}
