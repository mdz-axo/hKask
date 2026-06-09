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

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: svc-spec-001 — build_spec produces a spec with the given name
    #[test]
    fn build_spec_preserves_name() {
        let spec = SpecService::build_spec("my-spec", "domain", "hkask", &[]);
        assert_eq!(spec.name, "my-spec");
    }

    // REQ: svc-spec-002 — build_spec parses category and domain
    #[test]
    fn build_spec_parses_category_and_domain() {
        let spec = SpecService::build_spec("test", "capability", "okapi", &[]);
        assert_eq!(spec.category.as_str(), "capability");
        assert_eq!(spec.domain_anchor.as_str(), "okapi");
    }

    // REQ: svc-spec-003 — build_spec applies criteria to the goal
    #[test]
    fn build_spec_applies_criteria() {
        let spec = SpecService::build_spec(
            "test",
            "domain",
            "hkask",
            &["crit-1".to_string(), "crit-2".to_string()],
        );
        assert!(!spec.goals.is_empty(), "should have at least one goal");
        assert_eq!(
            spec.goals[0].criteria.len(),
            2,
            "goal should have 2 criteria"
        );
    }

    // REQ: svc-spec-004 — list_categories returns all spec categories
    #[test]
    fn list_categories_returns_all() {
        let cats = SpecService::list_categories();
        assert!(!cats.is_empty(), "should have at least one category");
    }

    // REQ: svc-spec-005 — build_spec falls back to defaults for invalid category
    #[test]
    fn build_spec_falls_back_for_invalid_category() {
        let spec = SpecService::build_spec("test", "nonexistent", "hkask", &[]);
        assert_eq!(
            spec.category.as_str(),
            "domain",
            "invalid category falls back to domain"
        );
    }

    // ── Parity tests: MCP spec server vs SpecService ──
    // The spec MCP server has 11 tools, of which 8 are MCP-only (OCAP verification,
    // Writing Excellence, test traceability, reconciliation, etc.). 3 tools
    // partially duplicate SpecService operations. These tests verify that both
    // paths produce the same domain types from the same string inputs.

    // PARITY: Both SpecCategory::parse_str paths match
    #[test]
    fn parity_spec_category_parsing_matches_service() {
        // MCP server uses SpecCategory::parse_str() directly (same as service)
        for &cat in SpecCategory::all() {
            let service_result: Option<SpecCategory> = SpecCategory::parse_str(cat.as_str());
            assert_eq!(
                service_result,
                Some(cat),
                "service parity: '{}' should parse to {:?}",
                cat.as_str(),
                cat
            );
        }
        // Invalid category falls back to Domain (both paths)
        assert_eq!(
            SpecCategory::parse_str("nonexistent"),
            None,
            "invalid category should not parse"
        );
        // Service falls back to Domain, MCP also falls back to Domain
        let service_spec = SpecService::build_spec("test", "nonexistent", "hkask", &[]);
        assert_eq!(service_spec.category, SpecCategory::Domain);
    }

    // PARITY: Both DomainAnchor::parse_str paths match
    #[test]
    fn parity_domain_anchor_parsing_matches_service() {
        // DomainAnchor has two variants: Okapi and Hkask
        for anchor in [DomainAnchor::Okapi, DomainAnchor::Hkask] {
            let result = DomainAnchor::parse_str(anchor.as_str());
            assert_eq!(
                result,
                Some(anchor),
                "domain anchor parity: '{}' should parse",
                anchor.as_str()
            );
        }
        assert_eq!(
            DomainAnchor::parse_str("nonexistent"),
            None,
            "invalid anchor should not parse"
        );
    }

    // PARITY: Both paths build the same Spec from equivalent inputs
    #[test]
    fn parity_spec_construction_matches_service() {
        // Service path: SpecService::build_spec
        let service_spec = SpecService::build_spec(
            "parity-test",
            "capability",
            "okapi",
            &["criterion-1".to_string()],
        );

        // MCP path: manual construction (same as spec_goal_capture does)
        let cat = SpecCategory::parse_str("capability").unwrap_or(SpecCategory::Domain);
        let anchor = DomainAnchor::parse_str("okapi").unwrap_or(DomainAnchor::Hkask);
        let goal = GoalSpec::new("parity-test").with_criterion("criterion-1");
        let mcp_spec = Spec::new("parity-test", cat, anchor).with_goal(goal);

        // Both produce the same domain type
        assert_eq!(service_spec.name, mcp_spec.name);
        assert_eq!(service_spec.category, mcp_spec.category);
        assert_eq!(service_spec.domain_anchor, mcp_spec.domain_anchor);
        assert_eq!(service_spec.is_complete(), mcp_spec.is_complete());
    }
}
