//! DDMVSS specification types — domain specifications, completeness predicates, and curation
//!
//! Domain specifications define completeness predicates and validation criteria.
//! Curation (Loop 5) cultivates specs; Cybernetics (Loop 6) tracks coverage;
//! Inference (Loop 1) uses them for guided generation.
//!
//! Relocated from `hkask-types` per P1: these types are consumed primarily by
//! `hkask-storage` (implementation) and `hkask-mcp-spec` (MCP surface).

use chrono::{DateTime, Utc};
use hkask_types::curation::{CurationDecision, OCAPBoundary};
use hkask_types::id::{GoalID, WebID};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpecId(pub Uuid);

impl SpecId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpecCategory {
    Domain,
    Capability,
    Interface,
    Composition,
}

impl SpecCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpecCategory::Domain => "domain",
            SpecCategory::Capability => "capability",
            SpecCategory::Interface => "interface",
            SpecCategory::Composition => "composition",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "domain" => Some(SpecCategory::Domain),
            "capability" => Some(SpecCategory::Capability),
            "interface" => Some(SpecCategory::Interface),
            "composition" => Some(SpecCategory::Composition),
            _ => None,
        }
    }

    pub fn all() -> &'static [SpecCategory] {
        &[
            SpecCategory::Domain,
            SpecCategory::Capability,
            SpecCategory::Interface,
            SpecCategory::Composition,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainAnchor {
    Okapi,
    Hkask,
}

impl DomainAnchor {
    pub fn as_str(&self) -> &'static str {
        match self {
            DomainAnchor::Okapi => "okapi",
            DomainAnchor::Hkask => "hkask",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "okapi" => Some(DomainAnchor::Okapi),
            "hkask" => Some(DomainAnchor::Hkask),
            _ => None,
        }
    }
}

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
        }
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
        let sub_coherence = if self.sub_goals.is_empty() {
            1.0
        } else {
            self.sub_goals.iter().map(|g| g.coherence()).sum::<f64>() / self.sub_goals.len() as f64
        };
        ((ratio + sub_coherence) / 2.0).clamp(0.0, 1.0)
    }
}

/// Drift report comparing a spec's declared verbs against actual registered tools.
///
/// Produced by `Spec::drift()`. High drift indicates the spec is out of sync
/// with the runtime tool inventory — either declaring verbs that no tool
/// implements, or missing verbs that tools provide.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    /// Jaccard distance between declared and registered verbs (0.0 = no drift, 1.0 = full drift).
    pub drift_magnitude: f64,
    /// Verbs the spec declares but no registered tool provides.
    pub missing_verbs: Vec<String>,
    /// Verbs that registered tools provide but the spec doesn't declare.
    pub extra_verbs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub id: SpecId,
    pub name: String,
    pub category: SpecCategory,
    pub domain_anchor: DomainAnchor,
    /// Verbs this spec declares as required capabilities.
    /// Compared against registered tool verbs by `drift()` to detect spec drift.
    pub declared_verbs: Vec<String>,
    pub goals: Vec<GoalSpec>,
    pub signed_by: Option<WebID>,
    pub created_at: DateTime<Utc>,
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
            signed_by: None,
            created_at: Utc::now(),
        }
    }

    /// Add a declared verb to this spec.
    pub fn with_declared_verb(mut self, verb: &str) -> Self {
        self.declared_verbs.push(verb.to_string());
        self
    }

    /// Compute drift between this spec's declared verbs and the actual registered tools.
    ///
    /// Returns a `DriftReport` with the Jaccard distance and the mismatched verbs.
    /// `registered_verbs` is provided by the caller (typically from MCP runtime)
    /// to avoid coupling `Spec` to the MCP runtime.
    pub fn drift(&self, registered_verbs: &[String]) -> DriftReport {
        let declared: HashSet<String> = self.declared_verbs.iter().cloned().collect();
        let registered: HashSet<String> = registered_verbs.iter().cloned().collect();

        let missing: Vec<String> = declared.difference(&registered).cloned().collect();
        let extra: Vec<String> = registered.difference(&declared).cloned().collect();

        let union_size = declared.union(&registered).count();
        let drift_magnitude = if union_size == 0 {
            0.0
        } else {
            let intersection_size = declared.intersection(&registered).count();
            1.0 - (intersection_size as f64 / union_size as f64)
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

pub trait SpecStore {
    fn load(&self, id: SpecId) -> Result<Spec, SpecError>;
    fn save(&self, spec: &Spec) -> Result<(), SpecError>;
    fn delete(&self, id: SpecId) -> Result<(), SpecError>;
    fn list_all(&self) -> Result<Vec<Spec>, SpecError>;
    fn list_by_category(&self, cat: SpecCategory) -> Result<Vec<Spec>, SpecError>;
}

pub trait SpecCurator {
    fn evaluate(
        &self,
        spec: &Spec,
        registered_verbs: &[String],
    ) -> Result<SpecCurationRecord, SpecError>;
    fn reconcile(
        &self,
        specs: &[Spec],
        registered_verbs: &[String],
    ) -> Result<Vec<SpecCurationRecord>, SpecError>;
    fn cultivate(&self, specs: &mut Vec<Spec>) -> Result<f64, SpecError>;
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
    #[error("Storage error: {0}")]
    Storage(String),
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
