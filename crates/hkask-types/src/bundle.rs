//! BundleManifest type system — skill bundling for hKask
//!
//! A BundleManifest composes multiple skills into a coherent agent capability,
//! declaring conflicts, complementarities, and cascade steps that govern
//! how the bundled skills execute together.
//!
//! The config sub-structs (ConvergenceConfig, GasConfig, etc.) mirror the
//! fields found in existing process manifests under `registry/manifests/`.

use serde::{Deserialize, Serialize};

use crate::lexicon::TemplateType;
use crate::visibility::Visibility;

/// Generates `as_str()` and `parse_str()` for a PascalCase enum.
macro_rules! enum_str_ops {
    ($ty:ident, { $($variant:ident => ($pascal:literal, $snake:literal)),+ $(,)? }) => {
        impl $ty {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $($ty::$variant => $pascal),+
                }
            }
            pub fn parse_str(s: &str) -> Option<Self> {
                match s {
                    $($pascal | $snake => Some($ty::$variant)),+,
                    _ => None,
                }
            }
        }
    };
}

// Enums

/// Skill polarity — cybernetic role in a bundle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SkillPolarity {
    Generative,
    Evaluative,
    Regulative,
    Procedural,
}

enum_str_ops!(SkillPolarity, {
    Generative => ("Generative", "generative"),
    Evaluative => ("Evaluative", "evaluative"),
    Regulative => ("Regulative", "regulative"),
    Procedural => ("Procedural", "procedural"),
});
impl SkillPolarity {
    pub fn is_divergent(&self) -> bool {
        matches!(self, Self::Generative)
    }
    pub fn is_convergent(&self) -> bool {
        matches!(self, Self::Evaluative)
    }
}

/// What kind of conflict exists between two skills
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ConflictType {
    CancelOut,
    ContradictoryDirective,
    OrderingCollision,
    ResourceContention,
}

enum_str_ops!(ConflictType, {
    CancelOut => ("CancelOut", "cancel_out"),
    ContradictoryDirective => ("ContradictoryDirective", "contradictory_directive"),
    OrderingCollision => ("OrderingCollision", "ordering_collision"),
    ResourceContention => ("ResourceContention", "resource_contention"),
});

/// How to resolve a declared conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ConflictResolution {
    DomainSeparation,
    PhaseSeparation,
    SpecificityOverride,
    ManifestOverride,
    UserIntent,
}

enum_str_ops!(ConflictResolution, {
    DomainSeparation => ("DomainSeparation", "domain_separation"),
    PhaseSeparation => ("PhaseSeparation", "phase_separation"),
    SpecificityOverride => ("SpecificityOverride", "specificity_override"),
    ManifestOverride => ("ManifestOverride", "manifest_override"),
    UserIntent => ("UserIntent", "user_intent"),
});

/// How two skills enhance each other
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ComplementarityType {
    SequentialFeed,
    ParallelAmplify,
    CrossDomainEnhance,
}

enum_str_ops!(ComplementarityType, {
    SequentialFeed => ("SequentialFeed", "sequential_feed"),
    ParallelAmplify => ("ParallelAmplify", "parallel_amplify"),
    CrossDomainEnhance => ("CrossDomainEnhance", "cross_domain_enhance"),
});

/// Cascade phase — where a step sits in the Pre/Core/Post pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CascadePhase {
    Pre,
    #[default]
    Core,
    Post,
}

enum_str_ops!(CascadePhase, {
    Pre => ("Pre", "pre"),
    Core => ("Core", "core"),
    Post => ("Post", "post"),
});

// Structs — Bundle skill, conflict, complementarity

/// A skill reference within a bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleSkill {
    pub id: String,
    pub polarity: SkillPolarity,
    pub lexicon_terms: Vec<String>,
    pub manifest_ref: String,
    pub content_hash: String,
}

/// A declared conflict between exactly two skills in a bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleConflict {
    pub skills: Vec<String>,
    pub domain: TemplateType,
    pub conflict_type: ConflictType,
    pub resolution: ConflictResolution,
    pub resolution_detail: String,
}

/// A declared complementarity between exactly two skills in a bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleComplementarity {
    pub skills: Vec<String>,
    pub complementarity_type: ComplementarityType,
    pub detail: String,
}

// Structs — Bundle manifest step

/// A single step in a bundle's cascade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifestStep {
    pub ordinal: u32,
    pub action: String,
    pub description: String,
    pub renderer: Option<String>,
    pub template_ref: Option<String>,
    pub model_tier: Option<String>,
    pub mcp: Option<String>,
    pub gas_cap: u32,
    pub timeout_seconds: u32,
    #[serde(default)]
    pub input_mapping: Option<serde_json::Value>,
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub phase: CascadePhase,
}

// Config sub-structs — mirror existing manifest YAML fields

/// Loaded from manifest YAML. Not yet enforced by ManifestExecutor (future wiring target).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ConvergenceConfig {
    pub threshold: f64,
    pub max_iterations: u32,
    pub on_not_reached: String,
}

impl Default for ConvergenceConfig {
    fn default() -> Self {
        Self {
            threshold: 0.1,
            max_iterations: 3,
            on_not_reached: "abort".to_string(),
        }
    }
}

/// Gas (energy budget) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GasConfig {
    pub cap: u32,
    pub cost_per_token: f64,
    pub alert_threshold: f64,
    pub hard_limit: bool,
}
impl Default for GasConfig {
    fn default() -> Self {
        Self {
            cap: 10000,
            cost_per_token: 0.25,
            alert_threshold: 0.8,
            hard_limit: true,
        }
    }
}

/// Error handling configuration. Loaded from manifest YAML, future wiring target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ErrorHandlingConfig {
    pub on_gas_exceeded: String,
    pub on_timeout: String,
    pub max_retries: u32,
    pub retry_backoff_seconds: u32,
    pub on_validation_failure: String,
}
impl Default for ErrorHandlingConfig {
    fn default() -> Self {
        Self {
            on_gas_exceeded: "abort".into(),
            on_timeout: "retry".into(),
            max_retries: 2,
            retry_backoff_seconds: 1,
            on_validation_failure: "abort".into(),
        }
    }
}

/// OCAP configuration. Loaded from manifest YAML, future wiring target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OcapConfig {
    pub delegation_chain_required: bool,
    pub signature_algorithm: String,
    pub capability_expiry_seconds: u32,
    pub template_scoped: bool,
}
impl Default for OcapConfig {
    fn default() -> Self {
        Self {
            delegation_chain_required: true,
            signature_algorithm: "ed25519".into(),
            capability_expiry_seconds: 3600,
            template_scoped: true,
        }
    }
}

/// CNS monitoring configuration. Loaded from manifest YAML, spans handled by GovernedTool at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CnsConfig {
    pub emit_spans: bool,
    pub span_namespace: String,
    pub variety_monitoring: bool,
    pub algedonic_threshold: u32,
    pub escalation_target: String,
}
impl Default for CnsConfig {
    fn default() -> Self {
        Self {
            emit_spans: true,
            span_namespace: String::new(),
            variety_monitoring: true,
            algedonic_threshold: 100,
            escalation_target: "Curator".into(),
        }
    }
}

/// Audit trail configuration. Loaded from manifest YAML, future wiring target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_level: String,
    pub include_input: bool,
    pub include_output: bool,
    pub include_gas_cost: bool,
    pub include_cns_events: bool,
}
impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_level: "info".into(),
            include_input: true,
            include_output: true,
            include_gas_cost: true,
            include_cns_events: true,
        }
    }
}

// Top-level BundleManifest

/// Composed bundle of skills with declared conflicts, complementarities, and cascade steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub editor: String,
    pub visibility: Visibility,
    pub skills: Vec<BundleSkill>,
    pub conflicts: Vec<BundleConflict>,
    pub complementarities: Vec<BundleComplementarity>,
    pub steps: Vec<BundleManifestStep>,
    pub convergence: ConvergenceConfig,
    pub gas: GasConfig,
    pub error_handling: ErrorHandlingConfig,
    pub ocap: OcapConfig,
    pub cns: CnsConfig,
    pub audit: AuditConfig,
    #[serde(default)]
    pub functional_role: Option<String>,
    #[serde(default)]
    pub inputs: Option<serde_json::Value>,
    #[serde(default)]
    pub principles: Option<serde_json::Value>,
}

impl BundleManifest {
    pub fn validate(&self) -> ValidationResult {
        let mut errors: Vec<String> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();
        if self.skills.len() < 2 {
            errors.push(format!(
                "Bundle must have at least 2 skills, found {}",
                self.skills.len()
            ));
        }
        if self.steps.len() > 7 {
            errors.push(format!(
                "Cascade depth exceeds matroshka limit ({} steps, max 7)",
                self.steps.len()
            ));
        }
        for skill in &self.skills {
            if skill.lexicon_terms.len() > 10 {
                errors.push(format!(
                    "Skill '{}' has {} lexicon terms (max 10)",
                    skill.id,
                    skill.lexicon_terms.len()
                ));
            }
        }
        let all_terms: std::collections::HashSet<&str> = self
            .skills
            .iter()
            .flat_map(|s| s.lexicon_terms.iter().map(|t| t.as_str()))
            .collect();
        if all_terms.len() > 30 {
            warnings.push(format!(
                "Bundle has {} unique lexicon terms (recommended max 30)",
                all_terms.len()
            ));
        }
        // P1: No divergent + convergent in the same phase
        let polarities_in = |phase: CascadePhase| -> Vec<&SkillPolarity> {
            self.steps
                .iter()
                .filter(|s| s.phase == phase)
                .filter_map(|s| {
                    self.skills
                        .iter()
                        .find(|sk| sk.id == s.description)
                        .map(|sk| &sk.polarity)
                })
                .collect()
        };
        for (phase, name) in [(CascadePhase::Pre, "Pre"), (CascadePhase::Core, "Core")] {
            let ps = polarities_in(phase);
            if ps.iter().any(|p| p.is_divergent()) && ps.iter().any(|p| p.is_convergent()) {
                errors.push(format!(
                    "P1 violation: divergent and convergent skills in same {name} phase"
                ));
            }
        }
        let skill_ids: std::collections::HashSet<&str> =
            self.skills.iter().map(|s| s.id.as_str()).collect();
        for conflict in &self.conflicts {
            for skill_id in &conflict.skills {
                if !skill_ids.contains(skill_id.as_str()) {
                    errors.push(format!(
                        "Conflict references skill '{}' not found in bundle",
                        skill_id
                    ));
                }
            }
            if conflict.skills.len() != 2 {
                errors.push(format!(
                    "Conflict must reference exactly 2 skills, found {}",
                    conflict.skills.len()
                ));
            }
        }
        for comp in &self.complementarities {
            for skill_id in &comp.skills {
                if !skill_ids.contains(skill_id.as_str()) {
                    errors.push(format!(
                        "Complementarity references skill '{}' not found in bundle",
                        skill_id
                    ));
                }
            }
            if comp.skills.len() != 2 {
                warnings.push(format!(
                    "Complementarity typically references 2 skills, found {}",
                    comp.skills.len()
                ));
            }
        }
        let mut ordinals: Vec<u32> = self.steps.iter().map(|s| s.ordinal).collect();
        ordinals.sort();
        for (i, expected) in ordinals.iter().enumerate() {
            if *expected != (i as u32) + 1 {
                errors.push(format!(
                    "Step ordinals not sequential: expected {}, found {}",
                    (i as u32) + 1,
                    expected
                ));
                break;
            }
        }
        if !self.version.contains('.') {
            warnings.push(format!(
                "Version '{}' does not follow semantic versioning",
                self.version
            ));
        }
        for skill in &self.skills {
            if skill.content_hash.is_empty() {
                warnings.push(format!("Skill '{}' has empty content_hash", skill.id));
            }
        }
        ValidationResult { errors, warnings }
    }
    pub fn total_step_gas(&self) -> u32 {
        self.steps.iter().map(|s| s.gas_cap).sum()
    }
    pub fn skills_in_phase(&self, phase: CascadePhase) -> Vec<&BundleSkill> {
        self.steps
            .iter()
            .filter(|s| s.phase == phase)
            .filter_map(|step| {
                self.skills
                    .iter()
                    .find(|sk| step.description.contains(&sk.id))
            })
            .collect()
    }
    pub fn skill_ids(&self) -> Vec<String> {
        self.skills.iter().map(|s| s.id.clone()).collect()
    }
}

/// Result of validating a BundleManifest.
#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
}
