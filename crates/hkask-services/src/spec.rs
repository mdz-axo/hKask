//! Specification capture, validation, and cultivation.

use chrono::{DateTime, Utc};
use hkask_agents::DefaultSpecCurator;
use hkask_storage::spec_types::{DomainAnchor, GoalSpec, Spec, SpecCategory, SpecCurator, SpecId};
use hkask_storage::{SpecStore, SqliteSpecStore};

use crate::error::ServiceError;

#[derive(Debug)]
pub struct CapturedSpec {
    pub spec: Spec,
    pub is_complete: bool,
}

#[derive(Debug)]
pub struct EvaluatedSpec {
    pub spec_id: SpecId,
    pub decision: hkask_types::curation::CurationDecision,
    pub rationale: String,
    pub coherence_score: f64,
    pub curated_at: DateTime<Utc>,
}

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

pub fn build_spec(name: &str, category: &str, domain: &str, criteria: &[String]) -> Spec {
    let cat = SpecCategory::parse_str(category).unwrap_or(SpecCategory::Domain);
    let anchor = DomainAnchor::parse_str(domain).unwrap_or(DomainAnchor::Hkask);
    let mut goal = GoalSpec::new(name);
    for c in criteria {
        goal = goal.with_criterion(c);
    }
    Spec::new(name, cat, anchor).with_goal(goal)
}

pub fn validate(spec_id: SpecId, store: &SqliteSpecStore) -> Result<EvaluatedSpec, ServiceError> {
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

pub fn cultivate(spec_id: SpecId, store: &SqliteSpecStore) -> Result<EvaluatedSpec, ServiceError> {
    validate(spec_id, store)
}

pub fn list_categories() -> Vec<&'static str> {
    SpecCategory::all().iter().map(|c| c.as_str()).collect()
}
