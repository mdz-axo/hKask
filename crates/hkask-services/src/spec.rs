//! Spec domain operations — specification capture, validation, and cultivation.
//!
//! Encapsulates the multi-step spec construction pipeline (parse inputs → build
//! goal → build spec → persist) and the evaluation pipeline (load → curator
//! evaluate → record). Both CLI and API delegate to these operations instead
//! of duplicating the construction and evaluation logic.

use chrono::{DateTime, Utc};
use hkask_agents::DefaultSpecCurator;
use hkask_storage::spec_types::{DomainAnchor, GoalSpec, Spec, SpecCategory, SpecCurator, SpecId};
use hkask_storage::{SpecStore, SqliteSpecStore};

use crate::error::ServiceError;

/// Result of capturing a new specification.
#[derive(Debug)]
pub struct CapturedSpec {
    /// The captured specification.
    pub spec: Spec,
    /// Whether the spec is complete (all goals have criteria).
    pub is_complete: bool,
}

/// Result of evaluating a specification (validation or cultivation).
#[derive(Debug)]
pub struct EvaluatedSpec {
    /// The spec ID that was evaluated.
    pub spec_id: SpecId,
    /// The curator's decision.
    pub decision: hkask_types::curation::CurationDecision,
    /// The curator's rationale.
    pub rationale: String,
    /// Coherence score from evaluation.
    pub coherence_score: f64,
    /// When the evaluation was performed.
    pub curated_at: DateTime<Utc>,
}

/// Spec domain service — specification capture, validation, and cultivation.
pub struct SpecService;

impl SpecService {
    /// Capture a new specification: parse inputs, build spec, persist.
    ///
    /// Constructs a `Spec` from raw string inputs (category, domain anchor),
    /// builds a `GoalSpec` with the given name and criteria, and saves
    /// the resulting spec to the store.
    pub fn capture(
        name: &str,
        category: &str,
        domain: &str,
        criteria: Option<&str>,
        store: &SqliteSpecStore,
    ) -> Result<CapturedSpec, ServiceError> {
        let cat = SpecCategory::parse_str(category).unwrap_or(SpecCategory::Domain);
        let anchor = DomainAnchor::parse_str(domain).unwrap_or(DomainAnchor::Hkask);
        let mut goal = GoalSpec::new(name);
        if let Some(crits) = criteria {
            for c in crits.split(',') {
                goal = goal.with_criterion(c.trim());
            }
        }
        let spec = Spec::new(name, cat, anchor).with_goal(goal);
        let is_complete = spec.is_complete();

        store.save(&spec).map_err(ServiceError::Spec)?;

        Ok(CapturedSpec { spec, is_complete })
    }

    /// Build a specification from raw inputs without persisting.
    ///
    /// Used by surfaces that want to construct a spec for display
    /// without saving it (e.g., API capture that returns the spec as JSON).
    pub fn build_spec(name: &str, category: &str, domain: &str, criteria: &[String]) -> Spec {
        let cat = SpecCategory::parse_str(category).unwrap_or(SpecCategory::Domain);
        let anchor = DomainAnchor::parse_str(domain).unwrap_or(DomainAnchor::Hkask);
        let mut goal = GoalSpec::new(name);
        for c in criteria {
            goal = goal.with_criterion(c);
        }
        Spec::new(name, cat, anchor).with_goal(goal)
    }

    /// Validate a specification: load from store and evaluate via curator.
    pub fn validate(
        spec_id: SpecId,
        store: &SqliteSpecStore,
    ) -> Result<EvaluatedSpec, ServiceError> {
        let spec = store.load(spec_id).map_err(ServiceError::Spec)?;
        let curator = DefaultSpecCurator::default();
        let record = curator.evaluate(&spec, &[])?;

        Ok(EvaluatedSpec {
            spec_id: record.spec_id,
            decision: record.decision,
            rationale: record.rationale,
            coherence_score: record.coherence_score,
            curated_at: record.curated_at,
        })
    }

    /// Cultivate a specification: load from store and evaluate via curator.
    ///
    /// Same underlying operation as validate, but named differently to
    /// match the domain vocabulary. The caller decides how to present
    /// the result (validation report vs cultivation guidance).
    pub fn cultivate(
        spec_id: SpecId,
        store: &SqliteSpecStore,
    ) -> Result<EvaluatedSpec, ServiceError> {
        Self::validate(spec_id, store)
    }

    /// List all spec categories as string identifiers.
    ///
    /// Convenience for surfaces that need to display the category catalog.
    pub fn list_categories() -> Vec<&'static str> {
        SpecCategory::all().iter().map(|c| c.as_str()).collect()
    }
}

