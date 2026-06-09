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

/// Skill polarity — the cybernetic role a skill plays in a bundle
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

/// Conflict type — what kind of conflict exists between two skills
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

/// Conflict resolution strategy — how to resolve a declared conflict
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

/// Complementarity type — how two skills enhance each other
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

/// Convergence configuration — controls when iterative steps stop improving.
///
/// Loaded from manifest YAML but not yet enforced by ManifestExecutor.
/// The executor currently only processes `steps`; convergence gating is a future wiring target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ConvergenceConfig {
    /// Minimum quality improvement to continue iterating
    pub threshold: f64,
    /// Maximum number of iterations before forcing convergence
    pub max_iterations: u32,
    /// Action when convergence is not reached (e.g. "invoke_child_manifest")
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
    /// Total gas cap for the entire bundle
    pub cap: u32,
    /// Cost per token (fractional, stored as fixed-point)
    pub cost_per_token: f64,
    /// Alert when this fraction of gas is consumed (0.0–1.0)
    pub alert_threshold: f64,
    /// Whether the cap is a hard limit (abort on exceed)
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

/// Error handling configuration — loaded from manifest YAML but not yet enforced by ManifestExecutor.
/// The executor currently only processes `steps`; error handling policies are a future wiring target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ErrorHandlingConfig {
    /// Action on gas exceeded ("abort" | "degrade")
    pub on_gas_exceeded: String,
    /// Action on timeout ("retry" | "abort")
    pub on_timeout: String,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Backoff interval between retries (seconds)
    pub retry_backoff_seconds: u32,
    /// Action on validation failure ("abort" | "skip")
    pub on_validation_failure: String,
}

impl Default for ErrorHandlingConfig {
    fn default() -> Self {
        Self {
            on_gas_exceeded: "abort".to_string(),
            on_timeout: "retry".to_string(),
            max_retries: 2,
            retry_backoff_seconds: 1,
            on_validation_failure: "abort".to_string(),
        }
    }
}

/// OCAP (Object Capability) configuration — loaded from manifest YAML but not yet enforced by ManifestExecutor.
/// The executor currently only processes `steps`; OCAP enforcement is a future wiring target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OcapConfig {
    /// Whether a delegation chain is required
    pub delegation_chain_required: bool,
    /// Signature algorithm (e.g. "ed25519")
    pub signature_algorithm: String,
    /// How long capabilities remain valid (seconds)
    pub capability_expiry_seconds: u32,
    /// Whether capabilities are scoped to this template
    pub template_scoped: bool,
}

impl Default for OcapConfig {
    fn default() -> Self {
        Self {
            delegation_chain_required: true,
            signature_algorithm: "ed25519".to_string(),
            capability_expiry_seconds: 3600,
            template_scoped: true,
        }
    }
}

/// CNS (Cybernetic Nervous System) monitoring configuration — loaded from manifest YAML but not yet enforced by ManifestExecutor.
/// The executor currently only processes `steps`; CNS span emission is handled by GovernedTool at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CnsConfig {
    /// Whether to emit CNS spans during execution
    pub emit_spans: bool,
    /// Span namespace (e.g. "cns.prompt.bundle-id")
    pub span_namespace: String,
    /// Whether to track variety counters
    pub variety_monitoring: bool,
    /// Variety deficit threshold that triggers algedonic escalation
    pub algedonic_threshold: u32,
    /// Where to escalate when algedonic threshold is breached
    pub escalation_target: String,
}

impl Default for CnsConfig {
    fn default() -> Self {
        Self {
            emit_spans: true,
            span_namespace: String::new(),
            variety_monitoring: true,
            algedonic_threshold: 100,
            escalation_target: "Curator".to_string(),
        }
    }
}

/// Audit trail configuration — loaded from manifest YAML but not yet enforced by ManifestExecutor.
/// The executor currently only processes `steps`; audit logging is a future wiring target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuditConfig {
    /// Whether audit logging is enabled
    pub enabled: bool,
    /// Log level ("info" | "debug" | "warn" | "error")
    pub log_level: String,
    /// Whether to include step inputs in audit records
    pub include_input: bool,
    /// Whether to include step outputs in audit records
    pub include_output: bool,
    /// Whether to include gas cost in audit records
    pub include_gas_cost: bool,
    /// Whether to include CNS events in audit records
    pub include_cns_events: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_level: "info".to_string(),
            include_input: true,
            include_output: true,
            include_gas_cost: true,
            include_cns_events: true,
        }
    }
}

// Top-level BundleManifest

/// BundleManifest — a composed bundle of skills with declared conflicts,
/// complementarities, and a cascade of execution steps.
///
/// This is the top-level type for the skill-bundler feature. It brings together
/// multiple skills, declares how they interact (conflicts and complementarities),
/// and defines an ordered cascade of steps with associated configuration.
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
    /// Functional role of this manifest (e.g. "knowact", "process").
    ///
    /// Set from the `manifest.functional_role` YAML field. Present in most
    /// manifests but optional for backward compatibility.
    #[serde(default)]
    pub functional_role: Option<String>,
    /// Manifest-level inputs (parameter declarations).
    #[serde(default)]
    pub inputs: Option<serde_json::Value>,
    /// Principles or behavioral constraints.
    #[serde(default)]
    pub principles: Option<serde_json::Value>,
}

impl BundleManifest {
    /// Validate the bundle manifest against hKask constraints.
    ///
    /// Returns a list of validation warnings. An empty list means the manifest
    /// is valid. Warnings are non-fatal issues; errors are violations of
    /// hard constraints.
    pub fn validate(&self) -> ValidationResult {
        let mut errors: Vec<String> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();

        // P7: Minimum 2 skills in a bundle
        if self.skills.len() < 2 {
            errors.push(format!(
                "Bundle must have at least 2 skills, found {}",
                self.skills.len()
            ));
        }

        // P6: Cascade depth ≤ 7 (matroshka limit)
        if self.steps.len() > 7 {
            errors.push(format!(
                "Cascade depth exceeds matroshka limit ({} steps, max 7)",
                self.steps.len()
            ));
        }

        // P7: Each skill ≤ 10 hLexicon terms
        for skill in &self.skills {
            if skill.lexicon_terms.len() > 10 {
                errors.push(format!(
                    "Skill '{}' has {} lexicon terms (max 10)",
                    skill.id,
                    skill.lexicon_terms.len()
                ));
            }
        }

        // P7: Bundle ≤ 30 unique terms
        let all_terms: std::collections::HashSet<&str> = self
            .skills
            .iter()
            .flat_map(|s| s.lexicon_terms.iter().map(|t| t.as_str()))
            .collect();
        if all_terms.len() > 30 {
            warnings.push(format!(
                "Bundle has {} unique lexicon terms (recommended max 30) — consider decomposing into sub-bundles",
                all_terms.len()
            ));
        }

        // P1: No divergent + convergent in the same phase
        let pre_skills: Vec<&SkillPolarity> = self
            .steps
            .iter()
            .filter(|s| s.phase == CascadePhase::Pre)
            .filter_map(|s| {
                self.skills
                    .iter()
                    .find(|sk| sk.id == s.description)
                    .map(|sk| &sk.polarity)
            })
            .collect();
        let core_skills: Vec<&SkillPolarity> = self
            .steps
            .iter()
            .filter(|s| s.phase == CascadePhase::Core)
            .filter_map(|s| {
                self.skills
                    .iter()
                    .find(|sk| sk.id == s.description)
                    .map(|sk| &sk.polarity)
            })
            .collect();

        let has_divergent_pre = pre_skills.iter().any(|p| p.is_divergent());
        let has_convergent_pre = pre_skills.iter().any(|p| p.is_convergent());
        let has_divergent_core = core_skills.iter().any(|p| p.is_divergent());
        let has_convergent_core = core_skills.iter().any(|p| p.is_convergent());

        if has_divergent_pre && has_convergent_pre {
            errors.push(
                "P1 violation: divergent and convergent skills in same Pre phase".to_string(),
            );
        }
        if has_divergent_core && has_convergent_core {
            errors.push(
                "P1 violation: divergent and convergent skills in same Core phase".to_string(),
            );
        }

        // Validate conflict skill references exist in the bundle
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

        // Validate complementarity skill references exist in the bundle
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

        // Validate step ordinals are sequential starting from 1
        let mut ordinals: Vec<u32> = self.steps.iter().map(|s| s.ordinal).collect();
        ordinals.sort();
        for (i, expected) in ordinals.iter().enumerate() {
            let ordinal = (i as u32) + 1;
            if *expected != ordinal {
                errors.push(format!(
                    "Step ordinals are not sequential: expected {}, found {}",
                    ordinal, expected
                ));
                break;
            }
        }

        // Validate version is semver-like
        if !self.version.contains('.') {
            warnings.push(format!(
                "Version '{}' does not follow semantic versioning",
                self.version
            ));
        }

        // Validate content hashes are present
        for skill in &self.skills {
            if skill.content_hash.is_empty() {
                warnings.push(format!(
                    "Skill '{}' has empty content_hash — evolution tracking will not work",
                    skill.id
                ));
            }
        }

        ValidationResult { errors, warnings }
    }

    /// Compute the total gas budget by summing all step gas caps.
    pub fn total_step_gas(&self) -> u32 {
        self.steps.iter().map(|s| s.gas_cap).sum()
    }

    /// Find skills by cascade phase.
    pub fn skills_in_phase(&self, phase: CascadePhase) -> Vec<&BundleSkill> {
        self.steps
            .iter()
            .filter(|s| s.phase == phase)
            .filter_map(|step| {
                // Try to match step to a skill by ID in description
                self.skills
                    .iter()
                    .find(|sk| step.description.contains(&sk.id))
            })
            .collect()
    }

    /// Get all skill IDs in this bundle.
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

// CNS Span Namespaces

/// CNS span namespaces for bundle composition operations.
/// These follow the hKask CNS naming convention: `cns.prompt.<operation>`.
#[allow(dead_code)] // reserved for CNS span wiring
pub(crate) mod cns_spans {
    /// Span namespace for bundle composition operations.
    pub const COMPOSE: &str = "cns.prompt.skill-bundler.compose";
    /// Span namespace for bundle application.
    pub const APPLY: &str = "cns.prompt.skill-bundler.apply";
    /// Span namespace for bundle evolution.
    pub const EVOLVE: &str = "cns.prompt.skill-bundler.evolve";
    /// Span namespace for bundle validation.
    pub const VALIDATE: &str = "cns.prompt.skill-bundler.validate";
}

mod tests {
    use super::*;
    use crate::visibility::Visibility;

    #[allow(dead_code)]
    fn make_skill(id: &str, polarity: SkillPolarity, terms: Vec<&str>) -> BundleSkill {
        BundleSkill {
            id: id.to_string(),
            polarity,
            lexicon_terms: terms.iter().map(|t| t.to_string()).collect(),
            manifest_ref: format!("{}-manifest", id),
            content_hash: format!("sha256:{}", id),
        }
    }

    #[allow(dead_code)]
    fn make_step(
        ordinal: u32,
        phase: CascadePhase,
        description: &str,
        gas_cap: u32,
    ) -> BundleManifestStep {
        BundleManifestStep {
            ordinal,
            action: format!("execute-{}", description),
            description: description.to_string(),
            renderer: None,
            template_ref: None,
            model_tier: None,
            mcp: None,
            gas_cap,
            timeout_seconds: 30,
            input_mapping: None,
            output_schema: None,
            phase,
        }
    }

    #[allow(dead_code)]
    fn valid_manifest() -> BundleManifest {
        let skill_a = make_skill("skill-a", SkillPolarity::Generative, vec!["term1", "term2"]);
        let skill_b = make_skill("skill-b", SkillPolarity::Evaluative, vec!["term3", "term4"]);
        BundleManifest {
            id: "bundle-test".to_string(),
            name: "Test Bundle".to_string(),
            description: "A valid test bundle".to_string(),
            version: "1.0.0".to_string(),
            editor: "test-editor".to_string(),
            visibility: Visibility::Public,
            skills: vec![skill_a, skill_b],
            conflicts: vec![],
            complementarities: vec![],
            steps: vec![
                make_step(1, CascadePhase::Pre, "skill-a", 100),
                make_step(2, CascadePhase::Core, "skill-b", 200),
            ],
            convergence: ConvergenceConfig::default(),
            gas: GasConfig::default(),
            error_handling: ErrorHandlingConfig::default(),
            ocap: OcapConfig::default(),
            cns: CnsConfig::default(),
            audit: AuditConfig::default(),
            functional_role: None,
            inputs: None,
            principles: None,
        }
    }
}
