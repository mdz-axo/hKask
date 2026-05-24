//! Specification domain types — DDMVSS (Domain-Driven Minimum Viable Specification Set)
//!
//! Load-bearing types for the specification toolset:
//! - `Spec`, `GoalSpec`, `Criterion` — recursive goal decomposition
//! - `SpecCategory` — 9 DDMVSS categories
//! - `CompletenessCheck` — trait for MVP completeness verification
//! - `SpecCurationRecord` — curation integration with existing `CurationDecision`
//! - Port traits: `SpecStore`, `SpecSigner`, `SpecObserver`, `SpecCurator`

use crate::curation::{CurationDecision, OCAPBoundary};
use crate::id::{GoalID, WebID};
use chrono::{DateTime, Utc};
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
    Trust,
    Observability,
    Persistence,
    Lifecycle,
    Curation,
}

impl SpecCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpecCategory::Domain => "domain",
            SpecCategory::Capability => "capability",
            SpecCategory::Interface => "interface",
            SpecCategory::Composition => "composition",
            SpecCategory::Trust => "trust",
            SpecCategory::Observability => "observability",
            SpecCategory::Persistence => "persistence",
            SpecCategory::Lifecycle => "lifecycle",
            SpecCategory::Curation => "curation",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "domain" => Some(SpecCategory::Domain),
            "capability" => Some(SpecCategory::Capability),
            "interface" => Some(SpecCategory::Interface),
            "composition" => Some(SpecCategory::Composition),
            "trust" => Some(SpecCategory::Trust),
            "observability" => Some(SpecCategory::Observability),
            "persistence" => Some(SpecCategory::Persistence),
            "lifecycle" => Some(SpecCategory::Lifecycle),
            "curation" => Some(SpecCategory::Curation),
            _ => None,
        }
    }

    pub fn all() -> &'static [SpecCategory] {
        &[
            SpecCategory::Domain,
            SpecCategory::Capability,
            SpecCategory::Interface,
            SpecCategory::Composition,
            SpecCategory::Trust,
            SpecCategory::Observability,
            SpecCategory::Persistence,
            SpecCategory::Lifecycle,
            SpecCategory::Curation,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainAnchor {
    Okapi,
    Russell,
    Hkask,
}

impl DomainAnchor {
    pub fn as_str(&self) -> &'static str {
        match self {
            DomainAnchor::Okapi => "okapi",
            DomainAnchor::Russell => "russell",
            DomainAnchor::Hkask => "hkask",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "okapi" => Some(DomainAnchor::Okapi),
            "russell" => Some(DomainAnchor::Russell),
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

    pub fn with_sub_goal(mut self, sub: GoalSpec) -> Result<Self, SpecError> {
        if self.depth >= 7 {
            return Err(SpecError::DepthExceeded);
        }
        let mut child = sub;
        child.depth = self.depth + 1;
        self.sub_goals.push(child);
        Ok(self)
    }

    pub fn can_have_subgoals(&self) -> bool {
        self.depth < 7
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub id: SpecId,
    pub name: String,
    pub category: SpecCategory,
    pub domain_anchor: DomainAnchor,
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
            goals: Vec::new(),
            signed_by: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_goal(mut self, goal: GoalSpec) -> Self {
        self.goals.push(goal);
        self
    }
}

pub trait CompletenessCheck {
    fn is_complete(&self) -> bool;
    fn coherence(&self) -> f64;
}

impl CompletenessCheck for GoalSpec {
    fn is_complete(&self) -> bool {
        !self.criteria.is_empty()
            && self.criteria.iter().all(|c| c.satisfied)
            && self.sub_goals.iter().all(|g| g.is_complete())
    }

    fn coherence(&self) -> f64 {
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

impl CompletenessCheck for Spec {
    fn is_complete(&self) -> bool {
        !self.goals.is_empty() && self.goals.iter().all(|g| g.is_complete())
    }

    fn coherence(&self) -> f64 {
        if self.goals.is_empty() {
            return 0.0;
        }
        self.goals.iter().map(|g| g.coherence()).sum::<f64>() / self.goals.len() as f64
    }
}

pub trait CollectionCoherence {
    fn collection_coherence(&self) -> f64;
    fn is_collection_complete(&self) -> bool;
}

impl CollectionCoherence for [Spec] {
    fn is_collection_complete(&self) -> bool {
        !self.is_empty() && self.iter().all(|s| s.is_complete())
    }

    fn collection_coherence(&self) -> f64 {
        if self.is_empty() {
            return 0.0;
        }
        let complete_count = self.iter().filter(|s| s.is_complete()).count();
        let categories_coveraged = self
            .iter()
            .map(|s| s.category.as_str())
            .collect::<HashSet<_>>()
            .len();
        let coverage = categories_coveraged as f64 / SpecCategory::all().len() as f64;
        let completeness = complete_count as f64 / self.len() as f64;
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

pub trait SpecSigner {
    fn sign(&self, spec: &mut Spec) -> Result<(), SpecError>;
    fn verify(&self, spec: &Spec) -> Result<bool, SpecError>;
}

pub trait SpecObserver {
    fn emit_span(&self, spec_id: SpecId, operation: &str, outcome: &serde_json::Value);
}

pub trait SpecCurator {
    fn evaluate(&self, spec: &Spec) -> Result<SpecCurationRecord, SpecError>;
    fn reconcile(&self, specs: &[Spec]) -> Result<Vec<SpecCurationRecord>, SpecError>;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_id_roundtrip() {
        let id = SpecId::new();
        let s = id.to_string();
        let parsed = SpecId::from_string(&s).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn spec_id_from_string_invalid() {
        let result = SpecId::from_string("not-a-uuid");
        assert!(result.is_err());
        match result {
            Err(SpecError::InvalidId(s)) => assert_eq!(s, "not-a-uuid"),
            _ => panic!("Expected InvalidId error"),
        }
    }

    #[test]
    fn spec_category_all_has_nine_variants() {
        assert_eq!(SpecCategory::all().len(), 9);
    }

    #[test]
    fn spec_category_parse_roundtrip() {
        for cat in SpecCategory::all() {
            let s = cat.as_str();
            let parsed = SpecCategory::parse_str(s).unwrap();
            assert_eq!(*cat, parsed);
        }
    }

    #[test]
    fn domain_anchor_parse_roundtrip() {
        assert_eq!(DomainAnchor::parse_str("okapi"), Some(DomainAnchor::Okapi));
        assert_eq!(
            DomainAnchor::parse_str("russell"),
            Some(DomainAnchor::Russell)
        );
        assert_eq!(DomainAnchor::parse_str("hkask"), Some(DomainAnchor::Hkask));
        assert_eq!(DomainAnchor::parse_str("unknown"), None);
    }

    #[test]
    fn goal_spec_completeness_requires_criteria() {
        let goal = GoalSpec::new("test");
        assert!(!goal.is_complete());
    }

    #[test]
    fn goal_spec_complete_when_all_criteria_satisfied() {
        let mut goal = GoalSpec::new("test").with_criterion("c1");
        goal.criteria[0].mark_satisfied();
        assert!(goal.is_complete());
    }

    #[test]
    fn goal_spec_subgoal_depth_limit() {
        let mut goal = GoalSpec::new("root");
        goal.depth = 7;
        assert!(!goal.can_have_subgoals());
        let child = GoalSpec::new("child");
        assert!(goal.with_sub_goal(child).is_err());
    }

    #[test]
    fn goal_spec_recursive_completeness() {
        let mut child = GoalSpec::new("child").with_criterion("c1");
        child.criteria[0].mark_satisfied();

        let mut parent = GoalSpec::new("parent")
            .with_criterion("p1")
            .with_sub_goal(child)
            .unwrap();
        assert!(!parent.is_complete());

        parent.criteria[0].mark_satisfied();
        assert!(parent.is_complete());
    }

    #[test]
    fn spec_completeness_requires_goals() {
        let spec = Spec::new("test", SpecCategory::Domain, DomainAnchor::Hkask);
        assert!(!spec.is_complete());
    }

    #[test]
    fn spec_complete_with_complete_goals() {
        let mut goal = GoalSpec::new("g1").with_criterion("c1");
        goal.criteria[0].mark_satisfied();

        let spec = Spec::new("test", SpecCategory::Domain, DomainAnchor::Hkask).with_goal(goal);
        assert!(spec.is_complete());
    }

    #[test]
    fn curation_record_coherence_clamped() {
        let record = SpecCurationRecord::new(
            SpecId::new(),
            CurationDecision::Merge,
            "test",
            1.5,
            OCAPBoundary::explicit("spec:curate".to_string()),
        );
        assert_eq!(record.coherence_score, 1.0);

        let record = SpecCurationRecord::new(
            SpecId::new(),
            CurationDecision::Discard,
            "test",
            -0.5,
            OCAPBoundary::denied("spec:curate".to_string()),
        );
        assert_eq!(record.coherence_score, 0.0);
    }

    #[test]
    fn spec_error_display() {
        let err = SpecError::DepthExceeded;
        assert_eq!(err.to_string(), "Depth limit exceeded: max 7");

        let err = SpecError::CoherenceInsufficient(0.3);
        assert_eq!(err.to_string(), "Coherence below threshold: 0.3");
    }

    #[test]
    fn goal_coherence_empty_criteria() {
        let goal = GoalSpec::new("test");
        assert_eq!(goal.coherence(), 0.0);
    }

    #[test]
    fn goal_coherence_partial() {
        let goal = GoalSpec::new("test")
            .with_criterion("c1")
            .with_criterion("c2");
        assert_eq!(goal.coherence(), 0.5);
    }

    #[test]
    fn goal_coherence_complete() {
        let mut goal = GoalSpec::new("test").with_criterion("c1");
        goal.criteria[0].mark_satisfied();
        assert_eq!(goal.coherence(), 1.0);
    }

    #[test]
    fn spec_coherence_empty() {
        let spec = Spec::new("test", SpecCategory::Domain, DomainAnchor::Hkask);
        assert_eq!(spec.coherence(), 0.0);
    }

    #[test]
    fn spec_coherence_complete() {
        let mut goal = GoalSpec::new("g1").with_criterion("c1");
        goal.criteria[0].mark_satisfied();
        let spec = Spec::new("test", SpecCategory::Domain, DomainAnchor::Hkask).with_goal(goal);
        assert_eq!(spec.coherence(), 1.0);
    }

    #[test]
    fn slice_coherence_empty() {
        let specs: Vec<Spec> = vec![];
        assert_eq!(specs.as_slice().collection_coherence(), 0.0);
    }

    #[test]
    fn slice_coherence_partial_coverage() {
        let mut goal = GoalSpec::new("g1").with_criterion("c1");
        goal.criteria[0].mark_satisfied();
        let spec = Spec::new("test", SpecCategory::Domain, DomainAnchor::Hkask).with_goal(goal);
        let specs = vec![spec];
        let coherence = specs.as_slice().collection_coherence();
        assert!(coherence > 0.0);
        assert!(coherence < 1.0);
    }
}
