//! BundleManifest type system — skill bundling for hKask
//!
//! A BundleManifest composes multiple skills into a coherent agent capability,
//! declaring conflicts, complementarities, and cascade steps that govern
//! how the bundled skills execute together.
//!
//! The config sub-structs (ConvergenceConfig, GasConfig, etc.) mirror the
//! fields found in existing process manifests under `registry/manifests/`.

use serde::{Deserialize, Serialize};

use super::cascade::CascadePhase;
use super::composition::{BundleComplementarity, BundleConflict};
use super::config::{
    AuditConfig, CnsConfig, ConvergenceConfig, ErrorHandlingConfig, GasConfig, OcapConfig,
};
use hkask_types::Visibility;

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

/// Skill polarity — cybernetic role in a bundle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SkillPolarity {
    Generative,
    Evaluative,
    Regulative,
    Procedural,
}

// as_str pre:  self is a valid SkillPolarity variant
// as_str post: returns PascalCase string ("Generative", "Evaluative", "Regulative", "Procedural")
// parse_str pre:  s is PascalCase or snake_case (e.g. "Generative"/"generative")
// parse_str post: returns Some(SkillPolarity) if s matches; None otherwise
enum_str_ops!(SkillPolarity, {
    Generative => ("Generative", "generative"),
    Evaluative => ("Evaluative", "evaluative"),
    Regulative => ("Regulative", "regulative"),
    Procedural => ("Procedural", "procedural"),
});
impl SkillPolarity {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is a valid SkillPolarity variant
    /// post: returns true if self is Generative (divergent/creative role); false otherwise
    pub fn is_divergent(&self) -> bool {
        matches!(self, Self::Generative)
    }
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is a valid SkillPolarity variant
    /// post: returns true if self is Evaluative (convergent/critical role); false otherwise
    pub fn is_convergent(&self) -> bool {
        matches!(self, Self::Evaluative)
    }
}

/// A skill reference within a bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleSkill {
    pub id: String,
    pub polarity: SkillPolarity,
    pub lexicon_terms: Vec<String>,
    pub manifest_ref: String,
    pub content_hash: String,
}

/// A single step in a bundle's cascade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifestStep {
    pub ordinal: u32,
    pub action: String,
    pub description: String,
    pub renderer: Option<String>,
    pub template_ref: Option<String>,
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

impl BundleManifestStep {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self.phase is a valid CascadePhase variant
    /// post: returns the PascalCase string representation of the cascade phase
    pub fn phase_str(&self) -> &'static str {
        self.phase.as_str()
    }
}

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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is a fully constructed BundleManifest
    /// post: returns ValidationResult with errors for hard violations (skill count, cascade depth, P1 polarity, etc.) and warnings for soft recommendations
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self.steps is populated with valid BundleManifestStep entries
    /// post: returns the sum of all step gas_cap values
    pub fn total_step_gas(&self) -> u32 {
        self.steps.iter().map(|s| s.gas_cap).sum()
    }
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  phase is a valid CascadePhase variant
    /// post: returns Vec of &BundleSkill references for skills whose step description contains their id and whose phase matches
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns `Vec<String>` of all skill ids in the bundle
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
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns true if errors is empty (no hard violations); false otherwise
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns true if warnings is non-empty; false otherwise
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}
