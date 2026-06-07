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
        // When there are sub_goals, coherence averages criteria satisfaction
        // with sub_goal coherence (both must be met). When there are no
        // sub_goals, coherence is just the criteria satisfaction ratio —
        // defaulting to 1.0 would inflate scores for incomplete specs.
        if self.sub_goals.is_empty() {
            ratio
        } else {
            let sub_coherence = self.sub_goals.iter().map(|g| g.coherence()).sum::<f64>()
                / self.sub_goals.len() as f64;
            ((ratio + sub_coherence) / 2.0).clamp(0.0, 1.0)
        }
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
    /// Git SHA of the last modification to this spec's manifest.
    /// Enables version tracking per DDMVSS §5.8 (Lifecycle category).
    /// Set to `None` for specs created programmatically without a manifest.
    pub version: Option<String>,
    /// Ed25519 signature over the canonical JSON of this spec.
    /// Produced by `Ed25519SpecSigner::sign_spec()` during spec registration.
    /// Verified by `Ed25519SpecSigner::verify_spec()` before spec consumption.
    /// DDMVSS §7 (Trust): specs are curated, not governed — the signature
    /// authenticates the spec's provenance, not its authority.
    pub signature: Option<String>,
    pub signed_by: Option<WebID>,
    pub created_at: DateTime<Utc>,
    /// Valid-from time for bitemporal semantics (DDMVSS §5.7 Persistence).
    /// When the fact described by this spec became true in the domain.
    /// `None` means valid since creation (`created_at`).
    pub valid_from: Option<DateTime<Utc>>,
    /// Valid-to time for bitemporal semantics (DDMVSS §5.7 Persistence).
    /// When the fact described by this spec ceased to be true in the domain.
    /// `None` means still currently valid.
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

    /// Add a declared verb to this spec.
    pub fn with_declared_verb(mut self, verb: &str) -> Self {
        self.declared_verbs.push(verb.to_string());
        self
    }

    /// Set the version (Git SHA) for this spec.
    pub fn with_version(mut self, sha: &str) -> Self {
        self.version = Some(sha.to_string());
        self
    }

    /// Set the Ed25519 signature for this spec.
    pub fn with_signature(mut self, sig: &str) -> Self {
        self.signature = Some(sig.to_string());
        self
    }

    /// Set the valid-from time for bitemporal semantics.
    pub fn with_valid_from(mut self, dt: DateTime<Utc>) -> Self {
        self.valid_from = Some(dt);
        self
    }

    /// Set the valid-to time for bitemporal semantics.
    pub fn with_valid_to(mut self, dt: DateTime<Utc>) -> Self {
        self.valid_to = Some(dt);
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
