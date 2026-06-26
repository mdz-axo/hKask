//! MDS specification types — domain specifications, completeness predicates, curation.
//! Relocated from `hkask-types` per P1: consumed by `hkask-storage`,
//! `hkask-services::SpecService` (via `spec_ops`).
//!
//! Five categories per MDS §1: Domain, Composition, Trust, Lifecycle, Curation.
use chrono::{DateTime, Utc};
use hkask_types::curation::{CurationDecision, OCAPBoundary};
use hkask_types::id::{GoalID, WebID};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;
/// Macro for string↔enum conversion pairs.
macro_rules! str_enum {
    ($enum:ident { $($variant:ident => $s:literal),+ $(,)? }) => {
        impl $enum {
            /// Get string representation.
            ///
            /// expect: "Storage types preserve semantic identity across operations"
            /// \[P8\] Motivating: Semantic Grounding — stable string representation
            /// post: returns lowercase string
            pub fn as_str(&self) -> &'static str {
                match self { $($enum::$variant => $s),+ }
            }
            /// Parse from string.
            ///
            /// expect: "Storage types preserve semantic identity across operations"
            /// \[P8\] Motivating: Semantic Grounding — parse from string
            /// post: returns Some if valid, None otherwise
            pub fn parse_str(s: &str) -> Option<Self> {
                match s.to_lowercase().as_str() {
                    $($s => Some($enum::$variant),)+
                    _ => None,
                }
            }
        }
    };
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpecId(pub Uuid);
impl SpecId {
    /// Create a new SpecId.
    ///
    /// expect: "Storage types preserve semantic identity across operations"
    /// \[P8\] Motivating: Semantic Grounding — new SpecId
    /// post: returns new random SpecId
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    /// Create a SpecId from a string.
    ///
    /// expect: "Storage types preserve semantic identity across operations"
    /// \[P8\] Motivating: Semantic Grounding — SpecId from string
    /// pre:  s is a valid UUID string
    /// post: returns SpecId
    pub fn from_string(s: &str) -> Result<Self, SpecError> {
        Uuid::parse_str(s)
            .map(SpecId)
            .map_err(|_| SpecError::InvalidId(s.to_string()))
    }
}
impl Default for SpecId {
    fn default() -> Self {
        Self::new()
    }
}
impl std::fmt::Display for SpecId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
/// MDS 5-category spec taxonomy (MDS §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpecCategory {
    Domain,
    Composition,
    Trust,
    Lifecycle,
    Curation,
}
impl SpecCategory {
    /// Get string representation of category.
    ///
    /// expect: "Storage types preserve semantic identity across operations"
    /// \[P8\] Motivating: Semantic Grounding — category string label
    /// post: returns snake_case string
    pub fn as_str(&self) -> &'static str {
        match self {
            SpecCategory::Domain => "domain",
            SpecCategory::Composition => "composition",
            SpecCategory::Trust => "trust",
            SpecCategory::Lifecycle => "lifecycle",
            SpecCategory::Curation => "curation",
        }
    }
    /// Parse a string into a `SpecCategory`.
    ///
    /// expect: "Storage types preserve semantic identity across operations"
    /// \[P8\] Motivating: Semantic Grounding — parse category
    /// post: returns Some(SpecCategory) if valid, None otherwise
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "domain" => Some(SpecCategory::Domain),
            "composition" => Some(SpecCategory::Composition),
            "trust" => Some(SpecCategory::Trust),
            "lifecycle" => Some(SpecCategory::Lifecycle),
            "curation" => Some(SpecCategory::Curation),
            _ => None,
        }
    }
    pub fn all() -> &'static [SpecCategory] {
        &[
            SpecCategory::Domain,
            SpecCategory::Composition,
            SpecCategory::Trust,
            SpecCategory::Lifecycle,
            SpecCategory::Curation,
        ]
    }
}
/// Infer MDS spec category from natural-language context keywords.
///
/// Single source of truth for context-keyword → MDS category mapping.
/// Used by `hkask-services::SpecService`, `hkask-storage::spec_ops`,
/// and (indirectly) `hkask-services::SpecService` — the canonical surface.
///
/// Defaults to [`SpecCategory::Domain`] when context is `None` or unrecognized.
/// pre:  arguments are valid
/// post: returns expected result
/// \[P8\] Motivating: Semantic Grounding — infer MDS category from context
pub fn infer_spec_category(context: Option<&str>) -> SpecCategory {
    let ctx = match context {
        Some(c) => c.to_lowercase(),
        None => return SpecCategory::Domain,
    };
    if ctx.contains("trust") || ctx.contains("security") || ctx.contains("threat") {
        SpecCategory::Trust
    } else if ctx.contains("compose") || ctx.contains("interface") || ctx.contains("api") {
        SpecCategory::Composition
    } else if ctx.contains("lifecycle") || ctx.contains("bootstrap") || ctx.contains("evolve") {
        SpecCategory::Lifecycle
    } else if ctx.contains("curat") || ctx.contains("review") || ctx.contains("coherence") {
        SpecCategory::Curation
    } else {
        SpecCategory::Domain
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainAnchor {
    Inference,
    Hkask,
}
str_enum!(DomainAnchor { Inference => "inference", Hkask => "hkask" });
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Criterion {
    pub description: String,
    pub satisfied: bool,
}
impl Criterion {
    pub fn new(description: &str) -> Self {
        Self {
            description: description.to_string(),
            satisfied: false,
        }
    }
    pub fn mark_satisfied(&mut self) {
        self.satisfied = true;
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSpec {
    pub id: GoalID,
    pub text: String,
    pub criteria: Vec<Criterion>,
    pub constraints: Vec<OCAPBoundary>,
    pub sub_goals: Vec<GoalSpec>,
    pub depth: u8,
    pub display_name: Option<String>,
}
impl GoalSpec {
    pub fn new(text: &str) -> Self {
        Self {
            id: GoalID::new(),
            text: text.to_string(),
            criteria: Vec::new(),
            constraints: Vec::new(),
            sub_goals: Vec::new(),
            depth: 0,
            display_name: None,
        }
    }
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }
    pub fn with_criterion(mut self, description: &str) -> Self {
        self.criteria.push(Criterion::new(description));
        self
    }
    pub fn can_have_subgoals(&self) -> bool {
        self.depth < 7
    }
    pub fn is_complete(&self) -> bool {
        !self.criteria.is_empty()
            && self.criteria.iter().all(|c| c.satisfied)
            && self.sub_goals.iter().all(|g| g.is_complete())
    }
    pub fn coherence(&self) -> f64 {
        if self.criteria.is_empty() {
            return 0.0;
        }
        let satisfied = self.criteria.iter().filter(|c| c.satisfied).count();
        let ratio = satisfied as f64 / self.criteria.len() as f64;
        if self.sub_goals.is_empty() {
            ratio
        } else {
            let sub_coherence = self.sub_goals.iter().map(|g| g.coherence()).sum::<f64>()
                / self.sub_goals.len() as f64;
            ((ratio + sub_coherence) / 2.0).clamp(0.0, 1.0)
        }
    }
}
/// Jaccard drift report: declared vs registered verb sets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    pub drift_magnitude: f64,
    pub missing_verbs: Vec<String>,
    pub extra_verbs: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub id: SpecId,
    pub name: String,
    pub category: SpecCategory,
    pub domain_anchor: DomainAnchor,
    pub declared_verbs: Vec<String>,
    pub goals: Vec<GoalSpec>,
    pub version: Option<String>,
    pub signature: Option<String>,
    pub signed_by: Option<WebID>,
    pub created_at: DateTime<Utc>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
}
impl Spec {
    pub fn new(name: &str, category: SpecCategory, domain_anchor: DomainAnchor) -> Self {
        Self {
            id: SpecId::new(),
            name: name.to_string(),
            category,
            domain_anchor,
            declared_verbs: Vec::new(),
            goals: Vec::new(),
            version: None,
            signature: None,
            signed_by: None,
            created_at: Utc::now(),
            valid_from: None,
            valid_to: None,
        }
    }
    pub fn with_declared_verb(mut self, verb: &str) -> Self {
        self.declared_verbs.push(verb.to_string());
        self
    }
    pub fn with_version(mut self, sha: &str) -> Self {
        self.version = Some(sha.to_string());
        self
    }
    pub fn with_signature(mut self, sig: &str) -> Self {
        self.signature = Some(sig.to_string());
        self
    }
    pub fn with_valid_from(mut self, dt: DateTime<Utc>) -> Self {
        self.valid_from = Some(dt);
        self
    }
    pub fn with_valid_to(mut self, dt: DateTime<Utc>) -> Self {
        self.valid_to = Some(dt);
        self
    }
    /// Compute Jaccard drift between declared verbs and registered tools.
    pub fn drift(&self, registered_verbs: &[String]) -> DriftReport {
        let declared: HashSet<String> = self.declared_verbs.iter().cloned().collect();
        let registered: HashSet<String> = registered_verbs.iter().cloned().collect();
        let missing: Vec<String> = declared.difference(&registered).cloned().collect();
        let extra: Vec<String> = registered.difference(&declared).cloned().collect();
        let union_size = declared.union(&registered).count();
        let drift_magnitude = if union_size == 0 {
            0.0
        } else {
            1.0 - (declared.intersection(&registered).count() as f64 / union_size as f64)
        };
        DriftReport {
            drift_magnitude: drift_magnitude.clamp(0.0, 1.0),
            missing_verbs: missing,
            extra_verbs: extra,
        }
    }
    pub fn with_goal(mut self, goal: GoalSpec) -> Self {
        self.goals.push(goal);
        self
    }
    pub fn is_complete(&self) -> bool {
        !self.goals.is_empty() && self.goals.iter().all(|g| g.is_complete())
    }
    pub fn coherence(&self) -> f64 {
        if self.goals.is_empty() {
            return 0.0;
        }
        self.goals.iter().map(|g| g.coherence()).sum::<f64>() / self.goals.len() as f64
    }
    pub fn collection_coherence(specs: &[Spec]) -> f64 {
        if specs.is_empty() {
            return 0.0;
        }
        let complete_count = specs.iter().filter(|s| s.is_complete()).count();
        let categories_coveraged = specs
            .iter()
            .map(|s| s.category.as_str())
            .collect::<HashSet<_>>()
            .len();
        let coverage = categories_coveraged as f64 / SpecCategory::all().len() as f64;
        let completeness = complete_count as f64 / specs.len() as f64;
        ((coverage + completeness) / 2.0).clamp(0.0, 1.0)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecCurationRecord {
    pub spec_id: SpecId,
    pub decision: CurationDecision,
    pub rationale: String,
    pub coherence_score: f64,
    pub ocap_boundary: OCAPBoundary,
    pub curated_at: DateTime<Utc>,
}
impl SpecCurationRecord {
    pub fn new(
        spec_id: SpecId,
        decision: CurationDecision,
        rationale: &str,
        coherence_score: f64,
        ocap_boundary: OCAPBoundary,
    ) -> Self {
        Self {
            spec_id,
            decision,
            rationale: rationale.to_string(),
            coherence_score: coherence_score.clamp(0.0, 1.0),
            ocap_boundary,
            curated_at: Utc::now(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum SpecError {
    #[error("Spec not found: {0}")]
    NotFound(SpecId),
    #[error("Invalid spec ID: {0}")]
    InvalidId(String),
    #[error("Capability denied: {0}")]
    CapabilityDenied(String),
    #[error("Signature invalid")]
    InvalidSignature,
    #[error(transparent)]
    Infra(#[from] hkask_types::InfrastructureError),
    #[error("Depth limit exceeded: max 7")]
    DepthExceeded,
    #[error("Curation authority required")]
    CurationDenied,
    #[error("Coherence below threshold: {0}")]
    CoherenceInsufficient(f64),
    #[error("Curation depth exceeded: max iterations reached")]
    CurationDepthExceeded,
    #[error("Spec drift exceeded threshold: {0}")]
    DriftExceeded(f64),
}
impl_from_rusqlite!(SpecError, Infra);
impl_from_serde_json!(SpecError, Infra);
/// Curation trait — evaluates spec coherence and makes curation decisions.
///
/// Implemented by `DefaultSpecCurator` in `hkask-agents`. This trait lives
/// in `hkask-storage` because it's tightly coupled to the spec data model
/// (Spec, SpecCurationRecord, SpecError) and is consumed by `hkask-services`.
pub trait SpecCurator: Send + Sync {
    /// Evaluate a single spec against registered verbs, producing a curation decision.
    fn evaluate(
        &self,
        spec: &Spec,
        registered_verbs: &[String],
    ) -> Result<SpecCurationRecord, SpecError>;
    /// Evaluate all specs and produce records.
    fn reconcile(
        &self,
        specs: &[Spec],
        registered_verbs: &[String],
    ) -> Result<Vec<SpecCurationRecord>, SpecError>;
    /// Iteratively cultivate a collection until coherence meets threshold.
    fn cultivate(&self, specs: &mut Vec<Spec>) -> Result<f64, SpecError>;
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_str_handles_mds_names() {
        assert_eq!(
            SpecCategory::parse_str("composition"),
            Some(SpecCategory::Composition)
        );
        assert_eq!(
            SpecCategory::parse_str("lifecycle"),
            Some(SpecCategory::Lifecycle)
        );
    }
    #[test]
    fn parse_str_returns_none_for_unknown() {
        assert_eq!(SpecCategory::parse_str("nonsense"), None);
    }
    #[test]
    fn spec_category_all_has_exactly_five_variants() {
        assert_eq!(SpecCategory::all().len(), 5);
    }
}
