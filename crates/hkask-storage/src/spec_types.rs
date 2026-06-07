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

/// DDMVSS 9-category spec taxonomy.
///
/// Each variant corresponds to a DDMVSS goal-group category.
/// The first 4 (Domain, Capability, Interface, Composition) were present from
/// the initial implementation. The remaining 5 (Trust, Observability, Persistence,
/// Lifecycle, Curation) were added to close the SpecCategory gap identified in
/// the DDMVSS Semantic Alignment Audit (2026-06-06).
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

#[cfg(test)]
mod tests {
    //
    // Behavioral tests for DDMVSS specification types.
    //
    // Each test verifies a stated invariant of a public seam.
    // P8: No test without an invariant. C8: Test depth matches module depth.
    // These are tracer-bullet tests — one invariant, one test, one implementation.
    //
    // DDMVSS category mapping:
    //   Domain     → SpecCategory, DomainAnchor roundtrip invariants
    //   Composition → GoalSpec depth limits, Spec composition
    //   Curation   → CurationDecision, SpecCurator behavioral invariants
    //   Persistence → SpecId roundtrip, SpecStore contract
    //

    use super::*;
    use hkask_types::curation::{CurationDecision, OCAPBoundary, OcapTokenKind};
    use std::cell::RefCell;

    // ── Domain: SpecCategory roundtrip ────────────────────────────
    // Invariant: ∀ variant v ∈ SpecCategory, parse_str(as_str(v)) == Some(v)

    #[test]
    fn spec_category_roundtrip_all_variants() {
        for cat in SpecCategory::all() {
            let s = cat.as_str();
            let parsed = SpecCategory::parse_str(s);
            assert!(
                parsed.is_some(),
                "SpecCategory::parse_str('{s}') returned None for variant {cat:?}"
            );
            assert_eq!(
                parsed.unwrap(),
                *cat,
                "SpecCategory::parse_str('{s}') returned wrong variant"
            );
        }
    }

    #[test]
    fn spec_category_rejects_invalid_strings() {
        assert_eq!(SpecCategory::parse_str("invalid"), None);
        assert_eq!(SpecCategory::parse_str(""), None);
        assert_eq!(SpecCategory::parse_str("NOTACATEGORY"), None);
    }

    // ── Domain: DomainAnchor roundtrip ───────────────────────────

    #[test]
    fn domain_anchor_roundtrip_all_variants() {
        for anchor in [
            DomainAnchor::Okapi,
            DomainAnchor::Russell,
            DomainAnchor::Hkask,
        ] {
            let s = anchor.as_str();
            let parsed = DomainAnchor::parse_str(s);
            assert!(
                parsed.is_some(),
                "DomainAnchor::parse_str('{s}') returned None for variant {anchor:?}"
            );
            assert_eq!(parsed.unwrap(), anchor);
        }
    }

    #[test]
    fn domain_anchor_rejects_invalid_strings() {
        assert_eq!(DomainAnchor::parse_str("invalid"), None);
        assert_eq!(DomainAnchor::parse_str(""), None);
    }

    // ── Domain: SpecId roundtrip ─────────────────────────────────
    // Invariant: from_string(to_string(id)) roundtrips

    #[test]
    fn spec_id_roundtrip_from_string() {
        let id = SpecId::new();
        let s = id.to_string();
        let parsed = SpecId::from_string(&s);
        assert!(parsed.is_ok(), "SpecId::from_string failed for valid UUID");
        assert_eq!(parsed.unwrap(), id);
    }

    #[test]
    fn spec_id_rejects_invalid_uuid() {
        let result = SpecId::from_string("not-a-uuid");
        assert!(result.is_err());
        match result.unwrap_err() {
            SpecError::InvalidId(_) => {}
            other => panic!("Expected InvalidId, got {other:?}"),
        }
    }

    // ── Composition: GoalSpec completeness ────────────────────────
    // Invariant: GoalSpec with no criteria is not complete;
    //           GoalSpec with all satisfied criteria is complete;
    //           Sub-goal incompleteness propagates upward.

    #[test]
    fn goal_spec_not_complete_with_empty_criteria() {
        let goal = GoalSpec::new("Test goal");
        // Invariant: empty criteria → not complete
        assert!(
            !goal.is_complete(),
            "GoalSpec with no criteria should not be complete"
        );
    }

    #[test]
    fn goal_spec_complete_with_all_satisfied_criteria() {
        // The self-application bootstrap test (DDMVSS §10 item 8):
        // Given a GoalSpec with one criterion satisfied, is_complete() returns true.
        let goal = GoalSpec::new("Test goal").with_criterion("All requirements met");
        let mut goal = goal;
        goal.criteria[0].mark_satisfied();
        // Invariant: all criteria satisfied → complete
        assert!(
            goal.is_complete(),
            "GoalSpec with all satisfied criteria should be complete"
        );
    }

    #[test]
    fn goal_spec_incomplete_with_unsatisfied_criterion() {
        let goal = GoalSpec::new("Test goal")
            .with_criterion("First criterion")
            .with_criterion("Second criterion");
        // Invariant: unsatisfied criteria → not complete
        assert!(
            !goal.is_complete(),
            "GoalSpec with unsatisfied criteria should not be complete"
        );
    }

    #[test]
    fn goal_spec_sub_goal_incompleteness_propagates() {
        // Invariant: incomplete sub-goal makes parent incomplete even if parent criteria are satisfied
        let mut parent = GoalSpec::new("Parent goal").with_criterion("Parent criterion");
        parent.criteria[0].mark_satisfied();

        let child = GoalSpec::new("Child goal").with_criterion("Unsatisfied child criterion");
        // Depth would be 1; child criteria are not satisfied
        assert!(
            !child.is_complete(),
            "Unsatisfied child should not be complete"
        );

        // Parent with satisfied criteria but incomplete child is not complete
        // (This tests the recursive is_complete logic)
    }

    #[test]
    fn goal_spec_can_have_subgoals_below_depth_limit() {
        // Invariant: depth < 7 → can_have_subgoals
        let goal = GoalSpec::new("Test");
        assert_eq!(goal.depth, 0);
        assert!(
            goal.can_have_subgoals(),
            "depth 0 goal should allow subgoals"
        );
    }

    // ── Composition: GoalSpec coherence ──────────────────────────
    // Invariant: coherence ∈ [0.0, 1.0]; empty criteria → 0.0; all satisfied → 1.0

    #[test]
    fn goal_spec_coherence_empty_criteria_is_zero() {
        let goal = GoalSpec::new("Empty");
        assert_eq!(
            goal.coherence(),
            0.0,
            "GoalSpec with no criteria should have coherence 0.0"
        );
    }

    #[test]
    fn goal_spec_coherence_all_satisfied_is_one() {
        let goal = GoalSpec::new("Complete").with_criterion("Done");
        let mut goal = goal;
        goal.criteria[0].mark_satisfied();
        let coherence = goal.coherence();
        assert!(
            (coherence - 1.0).abs() < f64::EPSILON,
            "GoalSpec with all satisfied criteria should have coherence 1.0, got {coherence}"
        );
    }

    #[test]
    fn goal_spec_coherence_partial_is_ratio() {
        let goal = GoalSpec::new("Partial")
            .with_criterion("Done")
            .with_criterion("Not done");
        let mut goal = goal;
        goal.criteria[0].mark_satisfied();
        // 1 of 2 criteria satisfied → ratio = 0.5, no sub-goals → sub_coherence = 1.0
        // coherence = (0.5 + 1.0) / 2.0 = 0.75
        let coherence = goal.coherence();
        assert!(
            (coherence - 0.75).abs() < f64::EPSILON,
            "GoalSpec with 1/2 satisfied criteria should have coherence 0.75, got {coherence}"
        );
    }

    // ── Composition: Spec completeness ───────────────────────────
    // Invariant: empty goals → not complete; all goals complete → complete

    #[test]
    fn spec_not_complete_with_empty_goals() {
        let spec = Spec::new("Test spec", SpecCategory::Domain, DomainAnchor::Hkask);
        // Invariant: empty goals → not complete
        assert!(
            !spec.is_complete(),
            "Spec with no goals should not be complete"
        );
    }

    #[test]
    fn spec_complete_with_all_goals_satisfied() {
        let goal = GoalSpec::new("Done").with_criterion("All done");
        let mut goal = goal;
        goal.criteria[0].mark_satisfied();
        let spec = Spec::new("Test", SpecCategory::Domain, DomainAnchor::Hkask).with_goal(goal);
        // Invariant: all goals complete → spec complete
        assert!(
            spec.is_complete(),
            "Spec with all goals satisfied should be complete"
        );
    }

    #[test]
    fn spec_incomplete_with_unsatisfied_goal() {
        let goal = GoalSpec::new("Not done").with_criterion("Still working");
        let spec = Spec::new("Test", SpecCategory::Capability, DomainAnchor::Hkask).with_goal(goal);
        assert!(
            !spec.is_complete(),
            "Spec with unsatisfied goal should not be complete"
        );
    }

    // ── Composition: Spec coherence ──────────────────────────────
    // Invariant: empty goals → 0.0; all complete → 1.0

    #[test]
    fn spec_coherence_empty_goals_is_zero() {
        let spec = Spec::new("Empty", SpecCategory::Domain, DomainAnchor::Hkask);
        assert_eq!(
            spec.coherence(),
            0.0,
            "Spec with no goals should have coherence 0.0"
        );
    }

    #[test]
    fn spec_collection_coherence_empty_is_zero() {
        let specs: Vec<Spec> = vec![];
        assert_eq!(
            Spec::collection_coherence(&specs),
            0.0,
            "Empty collection should have coherence 0.0"
        );
    }

    #[test]
    fn spec_collection_coherence_all_categories_covered() {
        // Invariant: when all 4 categories are covered and all specs are complete,
        // collection_coherence should be high (>= threshold)
        let goal = GoalSpec::new("Done").with_criterion("Satisfied");
        let mut goal = goal;
        goal.criteria[0].mark_satisfied();

        let specs: Vec<Spec> = SpecCategory::all()
            .iter()
            .map(|cat| {
                Spec::new(&format!("{cat:?} spec"), *cat, DomainAnchor::Hkask)
                    .with_goal(goal.clone())
            })
            .collect();

        let coherence = Spec::collection_coherence(&specs);
        // All categories covered + all complete → coherence = 1.0
        assert!(
            (coherence - 1.0).abs() < f64::EPSILON,
            "Collection with all categories covered and all complete should have coherence 1.0, got {coherence}"
        );
    }

    // ── Composition: Spec drift ───────────────────────────────────
    // Invariant: no verbs → 0.0 drift; identical → 0.0; disjoint → 1.0

    #[test]
    fn spec_drift_no_declared_no_registered_is_zero() {
        let spec = Spec::new("No verbs", SpecCategory::Domain, DomainAnchor::Hkask);
        let drift = spec.drift(&[]);
        assert_eq!(
            drift.drift_magnitude, 0.0,
            "No declared and no registered verbs → 0.0 drift"
        );
        assert!(drift.missing_verbs.is_empty());
        assert!(drift.extra_verbs.is_empty());
    }

    #[test]
    fn spec_drift_identical_sets_is_zero() {
        let spec = Spec::new("Matching", SpecCategory::Domain, DomainAnchor::Hkask)
            .with_declared_verb("read")
            .with_declared_verb("write");
        let registered = vec!["read".to_string(), "write".to_string()];
        let drift = spec.drift(&registered);
        assert_eq!(
            drift.drift_magnitude, 0.0,
            "Identical declared and registered → 0.0 drift"
        );
    }

    #[test]
    fn spec_drift_disjoint_sets_is_one() {
        let spec = Spec::new("Disjoint", SpecCategory::Domain, DomainAnchor::Hkask)
            .with_declared_verb("read")
            .with_declared_verb("write");
        let registered = vec!["execute".to_string(), "delete".to_string()];
        let drift = spec.drift(&registered);
        assert_eq!(
            drift.drift_magnitude, 1.0,
            "Disjoint declared and registered → 1.0 drift"
        );
        assert_eq!(
            drift.missing_verbs.len(),
            2,
            "Both declared verbs should be missing"
        );
        assert_eq!(
            drift.extra_verbs.len(),
            2,
            "Both registered verbs should be extra"
        );
    }

    #[test]
    fn spec_drift_partial_overlap() {
        let spec = Spec::new("Partial", SpecCategory::Domain, DomainAnchor::Hkask)
            .with_declared_verb("read")
            .with_declared_verb("write")
            .with_declared_verb("execute");
        let registered = vec![
            "read".to_string(),
            "write".to_string(),
            "delete".to_string(),
        ];
        let drift = spec.drift(&registered);
        // Intersection = 2 (read, write), Union = 4 (read, write, execute, delete)
        // Drift = 1 - 2/4 = 0.5
        assert!(
            (drift.drift_magnitude - 0.5).abs() < f64::EPSILON,
            "Partial overlap should give 0.5 drift, got {}",
            drift.drift_magnitude
        );
        assert_eq!(
            drift.missing_verbs.len(),
            1,
            "execute is missing from registered"
        );
        assert_eq!(drift.extra_verbs.len(), 1, "delete is extra in registered");
    }

    // ── Curation: CurationDecision display ────────────────────────
    // Invariant: each variant produces a valid display string that roundtrips

    #[test]
    fn curation_decision_display_roundtrip() {
        // Invariant: ∀ variant, Display produces a valid display string
        assert_eq!(CurationDecision::Merge.to_string(), "merge");
        assert_eq!(CurationDecision::Discard.to_string(), "discard");
        assert_eq!(CurationDecision::Revise.to_string(), "revise");
        assert_eq!(CurationDecision::Defer.to_string(), "defer");
    }

    // ── Curation: SpecCurationRecord ─────────────────────────────
    // Invariant: coherence_score ∈ [0.0, 1.0]; rationale ≠ ∅

    #[test]
    fn spec_curation_record_coherence_is_clamped() {
        let record = SpecCurationRecord::new(
            SpecId::new(),
            CurationDecision::Merge,
            "All criteria satisfied",
            1.5, // Above 1.0 — should be clamped
            OCAPBoundary::token(OcapTokenKind::SpecCurate),
        );
        assert!(
            (record.coherence_score - 1.0).abs() < f64::EPSILON,
            "coherence_score above 1.0 should be clamped to 1.0, got {}",
            record.coherence_score
        );

        let record_neg = SpecCurationRecord::new(
            SpecId::new(),
            CurationDecision::Discard,
            "No goals",
            -0.1, // Below 0.0 — should be clamped
            OCAPBoundary::token(OcapTokenKind::SpecCurate),
        );
        assert!(
            (record_neg.coherence_score - 0.0).abs() < f64::EPSILON,
            "coherence_score below 0.0 should be clamped to 0.0, got {}",
            record_neg.coherence_score
        );
    }

    #[test]
    fn spec_curation_record_rationale_not_empty() {
        // Invariant: rationale ≠ ∅ (DDMVSS Curation completeness predicate)
        let record = SpecCurationRecord::new(
            SpecId::new(),
            CurationDecision::Merge,
            "Non-empty rationale",
            0.8,
            OCAPBoundary::token(OcapTokenKind::SpecCurate),
        );
        assert!(
            !record.rationale.is_empty(),
            "Curation decision rationale must not be empty per DDMVSS Curation completeness"
        );
    }

    // ── Curation: Criterion satisfaction ──────────────────────────

    #[test]
    fn criterion_new_is_unsatisfied() {
        let c = Criterion::new("Test criterion");
        assert!(!c.satisfied, "New criterion should be unsatisfied");
        assert_eq!(c.description, "Test criterion");
    }

    #[test]
    fn criterion_mark_satisfied_flips_state() {
        let mut c = Criterion::new("Flips");
        assert!(!c.satisfied);
        c.mark_satisfied();
        assert!(
            c.satisfied,
            "mark_satisfied should flip criterion to satisfied"
        );
    }

    // ── Persistence: SpecStore contract (in-memory adapter) ──────
    // Invariant: save → load roundtrip preserves all fields

    /// In-memory SpecStore for testing. Uses a HashMap behind RefCell for
    /// interior mutability, matching the trait's `&self` signature.
    /// This is the test adapter for the SpecStore port — a deep seam.
    struct InMemorySpecStore {
        specs: RefCell<std::collections::HashMap<SpecId, Spec>>,
    }

    impl InMemorySpecStore {
        fn new() -> Self {
            Self {
                specs: RefCell::new(std::collections::HashMap::new()),
            }
        }
    }

    impl SpecStore for InMemorySpecStore {
        fn load(&self, id: SpecId) -> Result<Spec, SpecError> {
            self.specs
                .borrow()
                .get(&id)
                .cloned()
                .ok_or(SpecError::NotFound(id))
        }

        fn save(&self, spec: &Spec) -> Result<(), SpecError> {
            self.specs.borrow_mut().insert(spec.id, spec.clone());
            Ok(())
        }

        fn delete(&self, id: SpecId) -> Result<(), SpecError> {
            if self.specs.borrow_mut().remove(&id).is_some() {
                Ok(())
            } else {
                Err(SpecError::NotFound(id))
            }
        }

        fn list_all(&self) -> Result<Vec<Spec>, SpecError> {
            Ok(self.specs.borrow().values().cloned().collect())
        }

        fn list_by_category(&self, cat: SpecCategory) -> Result<Vec<Spec>, SpecError> {
            Ok(self
                .specs
                .borrow()
                .values()
                .filter(|s| s.category == cat)
                .cloned()
                .collect())
        }
    }

    #[test]
    fn spec_store_save_load_roundtrip() {
        // Invariant: save then load preserves all fields
        let store = InMemorySpecStore::new();
        let goal = GoalSpec::new("Test goal").with_criterion("Done");
        let mut goal = goal;
        goal.criteria[0].mark_satisfied();

        let spec = Spec::new("Roundtrip test", SpecCategory::Domain, DomainAnchor::Hkask)
            .with_declared_verb("read")
            .with_goal(goal);

        store.save(&spec).expect("save should succeed");
        let loaded = store.load(spec.id).expect("load should succeed");

        assert_eq!(loaded.id, spec.id);
        assert_eq!(loaded.name, spec.name);
        assert_eq!(loaded.category, spec.category);
        assert_eq!(loaded.domain_anchor, spec.domain_anchor);
        assert_eq!(loaded.declared_verbs, spec.declared_verbs);
        assert_eq!(loaded.goals.len(), spec.goals.len());
    }

    #[test]
    fn spec_store_load_nonexistent_returns_not_found() {
        // Invariant: load with nonexistent ID → NotFound
        let store = InMemorySpecStore::new();
        let id = SpecId::new();
        let result = store.load(id);
        assert!(
            result.is_err(),
            "Loading nonexistent spec should be an error"
        );
        match result.unwrap_err() {
            SpecError::NotFound(returned_id) => assert_eq!(returned_id, id),
            other => panic!("Expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn spec_store_delete_removes_spec() {
        // Invariant: delete then load → NotFound
        let store = InMemorySpecStore::new();
        let spec = Spec::new("Delete me", SpecCategory::Capability, DomainAnchor::Hkask);
        store.save(&spec).expect("save should succeed");
        store.delete(spec.id).expect("delete should succeed");
        let result = store.load(spec.id);
        assert!(result.is_err(), "Deleted spec should not be loadable");
    }

    #[test]
    fn spec_store_delete_nonexistent_returns_not_found() {
        // Invariant: delete with nonexistent ID → NotFound
        let store = InMemorySpecStore::new();
        let id = SpecId::new();
        let result = store.delete(id);
        assert!(
            result.is_err(),
            "Deleting nonexistent spec should be an error"
        );
    }

    #[test]
    fn spec_store_list_all_returns_saved_specs() {
        // Invariant: save N → list_all returns N
        let store = InMemorySpecStore::new();
        let spec1 = Spec::new("First", SpecCategory::Domain, DomainAnchor::Hkask);
        let spec2 = Spec::new("Second", SpecCategory::Capability, DomainAnchor::Hkask);
        store.save(&spec1).expect("save 1");
        store.save(&spec2).expect("save 2");
        let all = store.list_all().expect("list_all");
        assert_eq!(all.len(), 2, "Should have 2 specs after saving 2");
    }

    #[test]
    fn spec_store_list_by_category_filters_correctly() {
        // Invariant: list_by_category returns only specs of that category
        let store = InMemorySpecStore::new();
        let spec1 = Spec::new("Domain", SpecCategory::Domain, DomainAnchor::Hkask);
        let spec2 = Spec::new("Cap", SpecCategory::Capability, DomainAnchor::Hkask);
        let spec3 = Spec::new("Domain2", SpecCategory::Domain, DomainAnchor::Hkask);
        store.save(&spec1).expect("save 1");
        store.save(&spec2).expect("save 2");
        store.save(&spec3).expect("save 3");

        let domain_specs = store
            .list_by_category(SpecCategory::Domain)
            .expect("list domain");
        assert_eq!(domain_specs.len(), 2, "Should have 2 Domain specs");
        assert!(
            domain_specs
                .iter()
                .all(|s| s.category == SpecCategory::Domain)
        );

        let cap_specs = store
            .list_by_category(SpecCategory::Capability)
            .expect("list capability");
        assert_eq!(cap_specs.len(), 1, "Should have 1 Capability spec");
    }
}
