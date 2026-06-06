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

// Composition Error Types

/// Errors that can occur during bundle composition.
///
/// These cover the full composition pipeline: YAML parsing, validation,
/// and conflict resolution failures.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CompositionError {
    /// The LLM output could not be parsed as valid YAML.
    #[error("YAML parse error: {message}")]
    YamlParse { message: String },

    /// The parsed YAML is valid but doesn't conform to the bundle schema.
    #[error("Schema validation failed: {details}")]
    SchemaValidation { details: String },

    /// The manifest has validation errors (P1/P6/P7 violations).
    #[error("Manifest validation failed with {error_count} errors: {errors:?}")]
    ValidationFailed {
        error_count: usize,
        errors: Vec<String>,
    },

    /// No skills were found for the requested skill IDs.
    #[error("Skills not found: {missing_ids:?}")]
    SkillsNotFound { missing_ids: Vec<String> },

    /// An existing bundle already contains these skills (smart match hit).
    #[error("Existing bundle '{bundle_id}' already contains these skills")]
    ExistingBundle { bundle_id: String },

    /// The composition exceeded the gas budget.
    #[error("Composition gas budget exceeded: needed {needed}, cap {cap}")]
    GasBudgetExceeded { needed: u32, cap: u32 },

    /// The LLM produced output that couldn't be recovered after retry.
    #[error("Composition failed after {attempts} attempts: {message}")]
    RetryExhausted { attempts: u32, message: String },
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

impl SkillPolarity {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillPolarity::Generative => "Generative",
            SkillPolarity::Evaluative => "Evaluative",
            SkillPolarity::Regulative => "Regulative",
            SkillPolarity::Procedural => "Procedural",
        }
    }

    /// Parse a skill polarity from string, accepting both PascalCase and snake_case forms.
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "Generative" | "generative" => Some(SkillPolarity::Generative),
            "Evaluative" | "evaluative" => Some(SkillPolarity::Evaluative),
            "Regulative" | "regulative" => Some(SkillPolarity::Regulative),
            "Procedural" | "procedural" => Some(SkillPolarity::Procedural),
            _ => None,
        }
    }

    /// Whether this polarity is divergent (expands solution space).
    pub fn is_divergent(&self) -> bool {
        matches!(self, SkillPolarity::Generative)
    }

    /// Whether this polarity is convergent (narrows solution space).
    pub fn is_convergent(&self) -> bool {
        matches!(self, SkillPolarity::Evaluative)
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

impl ConflictType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConflictType::CancelOut => "CancelOut",
            ConflictType::ContradictoryDirective => "ContradictoryDirective",
            ConflictType::OrderingCollision => "OrderingCollision",
            ConflictType::ResourceContention => "ResourceContention",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "CancelOut" | "cancel_out" => Some(ConflictType::CancelOut),
            "ContradictoryDirective" | "contradictory_directive" => {
                Some(ConflictType::ContradictoryDirective)
            }
            "OrderingCollision" | "ordering_collision" => Some(ConflictType::OrderingCollision),
            "ResourceContention" | "resource_contention" => Some(ConflictType::ResourceContention),
            _ => None,
        }
    }
}

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

impl ConflictResolution {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConflictResolution::DomainSeparation => "DomainSeparation",
            ConflictResolution::PhaseSeparation => "PhaseSeparation",
            ConflictResolution::SpecificityOverride => "SpecificityOverride",
            ConflictResolution::ManifestOverride => "ManifestOverride",
            ConflictResolution::UserIntent => "UserIntent",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "DomainSeparation" | "domain_separation" => Some(ConflictResolution::DomainSeparation),
            "PhaseSeparation" | "phase_separation" => Some(ConflictResolution::PhaseSeparation),
            "SpecificityOverride" | "specificity_override" => {
                Some(ConflictResolution::SpecificityOverride)
            }
            "ManifestOverride" | "manifest_override" => Some(ConflictResolution::ManifestOverride),
            "UserIntent" | "user_intent" => Some(ConflictResolution::UserIntent),
            _ => None,
        }
    }
}

/// Complementarity type — how two skills enhance each other
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ComplementarityType {
    SequentialFeed,
    ParallelAmplify,
    CrossDomainEnhance,
}

impl ComplementarityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ComplementarityType::SequentialFeed => "SequentialFeed",
            ComplementarityType::ParallelAmplify => "ParallelAmplify",
            ComplementarityType::CrossDomainEnhance => "CrossDomainEnhance",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "SequentialFeed" | "sequential_feed" => Some(ComplementarityType::SequentialFeed),
            "ParallelAmplify" | "parallel_amplify" => Some(ComplementarityType::ParallelAmplify),
            "CrossDomainEnhance" | "cross_domain_enhance" => {
                Some(ComplementarityType::CrossDomainEnhance)
            }
            _ => None,
        }
    }
}

/// Cascade phase — where a step sits in the Pre/Core/Post pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CascadePhase {
    Pre,
    Core,
    Post,
}

impl CascadePhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            CascadePhase::Pre => "Pre",
            CascadePhase::Core => "Core",
            CascadePhase::Post => "Post",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "Pre" | "pre" => Some(CascadePhase::Pre),
            "Core" | "core" => Some(CascadePhase::Core),
            "Post" | "post" => Some(CascadePhase::Post),
            _ => None,
        }
    }
}

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
    pub phase: CascadePhase,
}

// Config sub-structs — mirror existing manifest YAML fields

/// Convergence configuration — controls when iterative steps stop improving
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

/// Error handling configuration
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

/// OCAP (Object Capability) configuration
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

/// CNS (Cybernetic Nervous System) monitoring configuration
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

/// Audit trail configuration
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
pub mod cns_spans {
    /// Span namespace for bundle composition operations.
    pub const COMPOSE: &str = "cns.prompt.skill-bundler.compose";
    /// Span namespace for bundle application.
    pub const APPLY: &str = "cns.prompt.skill-bundler.apply";
    /// Span namespace for bundle evolution.
    pub const EVOLVE: &str = "cns.prompt.skill-bundler.evolve";
    /// Span namespace for bundle validation.
    pub const VALIDATE: &str = "cns.prompt.skill-bundler.validate";
}

// Bundle Versioning

/// Bundle versioning strategy.
///
/// hKask uses semantic versioning for bundles:
/// - **Major**: Structural changes to the cascade (reordering phases, adding/removing steps)
/// - **Minor**: New skill added/removed, conflict/complementarity changes
/// - **Patch**: Instruction-only changes within skills, gas budget adjustments
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VersionBump {
    /// Structural cascade change (phase reorder, step add/remove)
    Major,
    /// Skill composition change (skill add/remove, conflict/complementarity update)
    Minor,
    /// Instruction-only change (prompt text, gas budget)
    Patch,
}

impl VersionBump {
    /// Apply the version bump to a semver string ("major.minor.patch").
    /// Returns the new version string.
    pub fn apply(&self, version: &str) -> String {
        let parts: Vec<u32> = version.split('.').filter_map(|s| s.parse().ok()).collect();

        let (mut major, mut minor, mut patch) = match parts.as_slice() {
            [ma, mi, pa] => (*ma, *mi, *pa),
            [ma, mi] => (*ma, *mi, 0),
            [ma] => (*ma, 0, 0),
            _ => (0, 0, 0),
        };

        match self {
            VersionBump::Major => {
                major += 1;
                minor = 0;
                patch = 0;
            }
            VersionBump::Minor => {
                minor += 1;
                patch = 0;
            }
            VersionBump::Patch => {
                patch += 1;
            }
        }

        format!("{}.{}.{}", major, minor, patch)
    }
}

// Bundle Dependency Tracking

/// Tracks which bundles depend on which skills.
/// Used for evolution: when a skill changes, we know which bundles need updating.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleDependencyIndex {
    /// Map from skill ID to the set of bundle IDs that depend on it.
    /// When a skill's content_hash changes, these bundles need evolution.
    skill_to_bundles: std::collections::HashMap<String, std::collections::HashSet<String>>,
}

impl BundleDependencyIndex {
    /// Create a new empty dependency index.
    pub fn new() -> Self {
        Self {
            skill_to_bundles: std::collections::HashMap::new(),
        }
    }

    /// Register a bundle, recording its skill dependencies.
    pub fn register_bundle(&mut self, bundle: &BundleManifest) {
        for skill in &bundle.skills {
            self.skill_to_bundles
                .entry(skill.id.clone())
                .or_default()
                .insert(bundle.id.clone());
        }
    }

    /// Remove a bundle from the index.
    pub fn remove_bundle(&mut self, bundle: &BundleManifest) {
        for skill in &bundle.skills {
            if let Some(dep_set) = self.skill_to_bundles.get_mut(&skill.id) {
                dep_set.remove(&bundle.id);
                if dep_set.is_empty() {
                    self.skill_to_bundles.remove(&skill.id);
                }
            }
        }
    }

    /// Find all bundles that depend on a given skill.
    pub fn bundles_dependent_on(&self, skill_id: &str) -> Vec<&str> {
        self.skill_to_bundles
            .get(skill_id)
            .map(|set| set.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Find all skills that have changed (by content hash) and return the
    /// bundles that need evolution.
    pub fn bundles_needing_evolution(&self, changed_skills: &[BundleSkillChange]) -> Vec<String> {
        let mut bundles: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for change in changed_skills {
            if let Some(dep_set) = self.skill_to_bundles.get(&change.skill_id) {
                bundles.extend(dep_set.iter().map(|s| s.as_str()));
            }
        }
        bundles.iter().map(|s| s.to_string()).collect()
    }
}

impl Default for BundleDependencyIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a change to a skill's content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleSkillChange {
    /// The skill ID that changed.
    pub skill_id: String,
    /// The previous content hash.
    pub previous_hash: String,
    /// The current content hash.
    pub current_hash: String,
    /// Whether the polarity changed.
    pub polarity_changed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexicon::TemplateType;
    use crate::visibility::Visibility;


    fn make_skill(id: &str, polarity: SkillPolarity, terms: Vec<&str>) -> BundleSkill {
        BundleSkill {
            id: id.to_string(),
            polarity,
            lexicon_terms: terms.iter().map(|t| t.to_string()).collect(),
            manifest_ref: format!("{}-manifest", id),
            content_hash: format!("sha256:{}", id),
        }
    }

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
        }
    }


    #[test]
    fn skill_polarity_roundtrip() {
        let variants = [
            SkillPolarity::Generative,
            SkillPolarity::Evaluative,
            SkillPolarity::Regulative,
            SkillPolarity::Procedural,
        ];

        // PascalCase round-trip
        for v in &variants {
            assert_eq!(SkillPolarity::parse_str(v.as_str()), Some(*v));
        }

        // snake_case forms
        assert_eq!(
            SkillPolarity::parse_str("generative"),
            Some(SkillPolarity::Generative)
        );
        assert_eq!(
            SkillPolarity::parse_str("evaluative"),
            Some(SkillPolarity::Evaluative)
        );
        assert_eq!(
            SkillPolarity::parse_str("regulative"),
            Some(SkillPolarity::Regulative)
        );
        assert_eq!(
            SkillPolarity::parse_str("procedural"),
            Some(SkillPolarity::Procedural)
        );

        // Unknown string returns None
        assert_eq!(SkillPolarity::parse_str("unknown"), None);

        // Divergent / convergent classification
        assert!(SkillPolarity::Generative.is_divergent());
        assert!(!SkillPolarity::Evaluative.is_divergent());
        assert!(!SkillPolarity::Regulative.is_divergent());
        assert!(!SkillPolarity::Procedural.is_divergent());

        assert!(SkillPolarity::Evaluative.is_convergent());
        assert!(!SkillPolarity::Generative.is_convergent());
        assert!(!SkillPolarity::Regulative.is_convergent());
        assert!(!SkillPolarity::Procedural.is_convergent());
    }


    #[test]
    fn conflict_type_roundtrip() {
        let variants = [
            ConflictType::CancelOut,
            ConflictType::ContradictoryDirective,
            ConflictType::OrderingCollision,
            ConflictType::ResourceContention,
        ];

        // PascalCase round-trip
        for v in &variants {
            assert_eq!(ConflictType::parse_str(v.as_str()), Some(*v));
        }

        // snake_case forms
        assert_eq!(
            ConflictType::parse_str("cancel_out"),
            Some(ConflictType::CancelOut)
        );
        assert_eq!(
            ConflictType::parse_str("contradictory_directive"),
            Some(ConflictType::ContradictoryDirective)
        );
        assert_eq!(
            ConflictType::parse_str("ordering_collision"),
            Some(ConflictType::OrderingCollision)
        );
        assert_eq!(
            ConflictType::parse_str("resource_contention"),
            Some(ConflictType::ResourceContention)
        );

        assert_eq!(ConflictType::parse_str("nonsense"), None);
    }


    #[test]
    fn conflict_resolution_roundtrip() {
        let variants = [
            ConflictResolution::DomainSeparation,
            ConflictResolution::PhaseSeparation,
            ConflictResolution::SpecificityOverride,
            ConflictResolution::ManifestOverride,
            ConflictResolution::UserIntent,
        ];

        for v in &variants {
            assert_eq!(ConflictResolution::parse_str(v.as_str()), Some(*v));
        }

        assert_eq!(
            ConflictResolution::parse_str("domain_separation"),
            Some(ConflictResolution::DomainSeparation)
        );
        assert_eq!(
            ConflictResolution::parse_str("phase_separation"),
            Some(ConflictResolution::PhaseSeparation)
        );
        assert_eq!(
            ConflictResolution::parse_str("specificity_override"),
            Some(ConflictResolution::SpecificityOverride)
        );
        assert_eq!(
            ConflictResolution::parse_str("manifest_override"),
            Some(ConflictResolution::ManifestOverride)
        );
        assert_eq!(
            ConflictResolution::parse_str("user_intent"),
            Some(ConflictResolution::UserIntent)
        );

        assert_eq!(ConflictResolution::parse_str("bogus"), None);
    }


    #[test]
    fn complementarity_type_roundtrip() {
        let variants = [
            ComplementarityType::SequentialFeed,
            ComplementarityType::ParallelAmplify,
            ComplementarityType::CrossDomainEnhance,
        ];

        for v in &variants {
            assert_eq!(ComplementarityType::parse_str(v.as_str()), Some(*v));
        }

        assert_eq!(
            ComplementarityType::parse_str("sequential_feed"),
            Some(ComplementarityType::SequentialFeed)
        );
        assert_eq!(
            ComplementarityType::parse_str("parallel_amplify"),
            Some(ComplementarityType::ParallelAmplify)
        );
        assert_eq!(
            ComplementarityType::parse_str("cross_domain_enhance"),
            Some(ComplementarityType::CrossDomainEnhance)
        );

        assert_eq!(ComplementarityType::parse_str("nope"), None);
    }


    #[test]
    fn cascade_phase_roundtrip() {
        let variants = [CascadePhase::Pre, CascadePhase::Core, CascadePhase::Post];

        for v in &variants {
            assert_eq!(CascadePhase::parse_str(v.as_str()), Some(*v));
        }

        // lowercase forms
        assert_eq!(CascadePhase::parse_str("pre"), Some(CascadePhase::Pre));
        assert_eq!(CascadePhase::parse_str("core"), Some(CascadePhase::Core));
        assert_eq!(CascadePhase::parse_str("post"), Some(CascadePhase::Post));

        assert_eq!(CascadePhase::parse_str("invalid"), None);
    }


    #[test]
    fn bundle_manifest_validate_valid() {
        let manifest = valid_manifest();
        let result = manifest.validate();
        assert!(
            result.is_valid(),
            "Expected no errors, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn bundle_manifest_validate_too_few_skills() {
        let mut manifest = valid_manifest();
        manifest.skills = vec![make_skill("lonely", SkillPolarity::Generative, vec!["t1"])];
        let result = manifest.validate();
        assert!(!result.is_valid());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("at least 2 skills"))
        );
    }

    #[test]
    fn bundle_manifest_validate_exceeds_matroshka_limit() {
        let mut manifest = valid_manifest();
        // 8 steps exceeds the matroshka limit of 7
        manifest.steps = (1..=8)
            .map(|i| make_step(i, CascadePhase::Core, &format!("step-{}", i), 50))
            .collect();
        let result = manifest.validate();
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.contains("matroshka limit")));
    }

    #[test]
    fn bundle_manifest_validate_too_many_lexicon_terms() {
        let mut manifest = valid_manifest();
        // Skill with 11 terms exceeds the per-skill limit of 10
        let terms: Vec<String> = (1..=11).map(|i| format!("t{}", i)).collect();
        let skill_b = BundleSkill {
            id: "skill-b".to_string(),
            polarity: SkillPolarity::Evaluative,
            lexicon_terms: terms,
            manifest_ref: "skill-b-manifest".to_string(),
            content_hash: "sha256:skill-b".to_string(),
        };
        manifest.skills = vec![
            make_skill("skill-a", SkillPolarity::Generative, vec!["t1"]),
            skill_b,
        ];
        let result = manifest.validate();
        assert!(!result.is_valid());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("skill-b") && e.contains("lexicon terms"))
        );
    }

    #[test]
    fn bundle_manifest_validate_too_many_unique_terms() {
        let mut manifest = valid_manifest();
        // 4 skills × 8 unique terms each = 32 unique terms, exceeding the recommended max of 30
        // Each individual skill has ≤10 terms so per-skill validation passes
        let skills: Vec<BundleSkill> = (0..4)
            .map(|i| BundleSkill {
                id: format!("skill-{}", i),
                polarity: if i % 2 == 0 {
                    SkillPolarity::Generative
                } else {
                    SkillPolarity::Evaluative
                },
                lexicon_terms: (0..8).map(|j| format!("s{}-t{}", i, j)).collect(),
                manifest_ref: format!("skill-{}-manifest", i),
                content_hash: format!("sha256:skill-{}", i),
            })
            .collect();
        manifest.skills = skills;
        manifest.steps = vec![
            make_step(1, CascadePhase::Pre, "skill-0", 50),
            make_step(2, CascadePhase::Core, "skill-1", 50),
            make_step(3, CascadePhase::Pre, "skill-2", 50),
            make_step(4, CascadePhase::Core, "skill-3", 50),
        ];
        let result = manifest.validate();
        assert!(
            result.is_valid(),
            "Expected no errors, got: {:?}",
            result.errors
        ); // Too many unique terms is a warning, not an error
        assert!(result.has_warnings());
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("unique lexicon terms"))
        );
    }

    #[test]
    fn bundle_manifest_validate_divergent_convergent_same_phase() {
        let mut manifest = valid_manifest();
        // Put both a Generative (divergent) and Evaluative (convergent) skill in Core phase
        manifest.skills = vec![
            make_skill("gen-skill", SkillPolarity::Generative, vec!["t1"]),
            make_skill("eval-skill", SkillPolarity::Evaluative, vec!["t2"]),
        ];
        manifest.steps = vec![
            make_step(1, CascadePhase::Core, "gen-skill", 100),
            make_step(2, CascadePhase::Core, "eval-skill", 100),
        ];
        let result = manifest.validate();
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.contains("P1 violation")));
    }

    #[test]
    fn bundle_manifest_validate_conflict_references_invalid_skill() {
        let mut manifest = valid_manifest();
        manifest.conflicts = vec![BundleConflict {
            skills: vec!["skill-a".to_string(), "ghost-skill".to_string()],
            domain: TemplateType::WordAct,
            conflict_type: ConflictType::CancelOut,
            resolution: ConflictResolution::DomainSeparation,
            resolution_detail: "Separate by domain".to_string(),
        }];
        let result = manifest.validate();
        assert!(!result.is_valid());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("ghost-skill") && e.contains("not found"))
        );
    }

    #[test]
    fn bundle_manifest_validate_non_sequential_ordinals() {
        let mut manifest = valid_manifest();
        manifest.steps = vec![
            make_step(2, CascadePhase::Pre, "skill-a", 100),
            make_step(5, CascadePhase::Core, "skill-b", 200),
        ];
        let result = manifest.validate();
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.contains("sequential")));
    }


    #[test]
    fn bundle_manifest_total_step_gas() {
        let manifest = valid_manifest();
        // Pre step gas_cap=100, Core step gas_cap=200 → total=300
        assert_eq!(manifest.total_step_gas(), 300);
    }


    #[test]
    fn bundle_manifest_skill_ids() {
        let manifest = valid_manifest();
        let ids = manifest.skill_ids();
        assert_eq!(ids, vec!["skill-a", "skill-b"]);
    }
}
