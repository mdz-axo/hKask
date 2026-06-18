//! SpecService — specification capture, listing, and coherence for CLI and API surfaces.
//!
//! Unifies the divergent capture semantics: CLI uses explicit name + category +
//! domain + comma-separated criteria; API uses description + context (auto-inferred
//! category via keyword matching). Both paths produce a `Spec` stored via
//! `AgentService::spec_store()`.
//!
//! Category inference delegates to `hkask_storage::spec_types::infer_spec_category` —
//! the single source of truth for context-keyword → MDS category mapping.

use hkask_agents::DefaultSpecCurator;
use hkask_storage::SpecStore;
use hkask_storage::spec_types::SpecCurator;
use hkask_storage::spec_types::{
    DomainAnchor, GoalSpec, Spec, SpecCategory, SpecCurationRecord, SpecId, infer_spec_category,
};

use crate::AgentService;
use crate::ServiceError;

/// Request to capture a new specification.
///
/// Two input styles are supported:
/// - **CLI path**: `name`, `category`, `domain`, `criteria` (optional comma-separated list)
/// - **API path**: `description`, `context` (category auto-inferred from context keywords)
pub struct SpecCaptureRequest {
    /// Explicit spec name (CLI) or used as the description (API).
    pub name_or_description: String,
    /// Explicit category override (CLI). When `None`, inferred from context.
    pub category: Option<String>,
    /// Explicit domain anchor override (CLI). Defaults to Hkask.
    pub domain: Option<String>,
    /// Comma-separated criteria (CLI). When `None`, auto-seed from description sentences (API).
    pub criteria: Option<String>,
    /// Context string for category inference (API). When `None`, uses default Domain.
    pub context: Option<String>,
}

/// Response after spec capture.
pub struct SpecCaptureResponse {
    pub spec_id: String,
    pub name: String,
    pub category: String,
    pub domain_anchor: String,
    pub complete: bool,
}

/// Summary of a spec list entry.
pub struct SpecListEntry {
    pub spec_id: String,
    pub name: String,
    pub category: String,
    pub complete: bool,
}

impl From<Spec> for SpecListEntry {
    fn from(s: Spec) -> Self {
        let complete = s.is_complete();
        let cat = s.category.as_str().to_string();
        let id = s.id.to_string();
        Self {
            spec_id: id,
            name: s.name,
            category: cat,
            complete,
        }
    }
}

/// Detailed spec response.
pub struct SpecDetail {
    pub spec_id: String,
    pub name: String,
    pub category: String,
    pub domain_anchor: String,
    pub requirements: Vec<String>,
}

/// Coherence assessment.
pub struct CoherenceResult {
    pub coherence_score: f64,
    pub violations: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Writing quality assessment.
pub struct WritingQualityResult {
    pub dimensions_passing: usize,
    pub meets_publication_standard: bool,
}

/// Service for specification management — delegates to SpecStore.
pub struct SpecService;

impl SpecService {
    /// Capture a new specification.
    ///
    /// Unifies the CLI and API capture paths. When `category` is provided,
    /// uses it directly (CLI path). Otherwise, infers from `context` keywords
    /// (API path). Criteria are parsed from the comma-separated `criteria`
    /// field when present; otherwise auto-seeded from description sentences.
    ///
    /// REQ: P8-svc-spec-081
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx.spec_store() must be initialized; req.name_or_description must be non-empty
    /// post: spec is persisted to the spec store; returns SpecCaptureResponse with spec_id, name, category, domain_anchor, and complete flag
    pub fn capture(
        ctx: &AgentService,
        req: SpecCaptureRequest,
    ) -> Result<SpecCaptureResponse, ServiceError> {
        let cat = match req.category.as_deref() {
            Some(c) => SpecCategory::parse_str(c).unwrap_or(SpecCategory::Domain),
            None => infer_spec_category(req.context.as_deref()),
        };
        let anchor = match req.domain.as_deref() {
            Some(d) => DomainAnchor::parse_str(d).unwrap_or(DomainAnchor::Hkask),
            None => DomainAnchor::Hkask,
        };

        let mut goal = GoalSpec::new(&req.name_or_description);
        match req.criteria.as_deref() {
            Some(crits) if !crits.is_empty() => {
                for c in crits.split(',') {
                    let trimmed = c.trim();
                    if !trimmed.is_empty() {
                        goal = goal.with_criterion(trimmed);
                    }
                }
            }
            _ => {
                // Auto-seed from description sentences (API path)
                for sentence in req.name_or_description.split('.') {
                    let trimmed = sentence.trim();
                    if !trimmed.is_empty() && trimmed.len() < 200 {
                        goal = goal.with_criterion(trimmed);
                    }
                }
            }
        }

        let spec = Spec::new(&req.name_or_description, cat, anchor).with_goal(goal);
        let is_complete = spec.is_complete();
        let store = ctx.spec_store();
        store.save(&spec).map_err(|e| ServiceError::Spec { message: e.to_string() })?;

        Ok(SpecCaptureResponse {
            spec_id: spec.id.to_string(),
            name: spec.name,
            category: spec.category.as_str().to_string(),
            domain_anchor: spec.domain_anchor.as_str().to_string(),
            complete: is_complete,
        })
    }

    /// List all specs, optionally filtered by category.
    ///
    /// REQ: P8-svc-spec-082
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx.spec_store() must be initialized; category_filter if Some must be a valid SpecCategory string
    /// post: returns Vec<SpecListEntry> for all matching specs; Err(ValidationError) on invalid category
    pub fn list(
        ctx: &AgentService,
        category_filter: Option<&str>,
    ) -> Result<Vec<SpecListEntry>, ServiceError> {
        let store = ctx.spec_store();
        let specs = match category_filter {
            Some(cat_str) => {
                let cat = SpecCategory::parse_str(cat_str).ok_or_else(|| {
                    ServiceError::ValidationError {
                        message: format!(
                            "Unknown category '{}': valid: domain, composition, trust, lifecycle, curation",
                            cat_str
                        ),
                    }
                })?;
                store.list_by_category(cat).map_err(|e| ServiceError::Spec { message: e.to_string() })?
            }
            None => store.list_all().map_err(|e| ServiceError::Spec { message: e.to_string() })?,
        };
        Ok(specs.into_iter().map(SpecListEntry::from).collect())
    }

    /// Get a single spec by ID (full struct with goals).
    ///
    /// REQ: P8-svc-spec-083
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  spec_id_str must be a valid UUID; ctx.spec_store() must be initialized
    /// post: returns the full Spec with goals on success; Err(ValidationError) on invalid UUID; Err(Spec) on store error
    pub fn get_full(ctx: &AgentService, spec_id_str: &str) -> Result<Spec, ServiceError> {
        let id = parse_spec_id(spec_id_str)?;
        let store = ctx.spec_store();
        store.load(id).map_err(|e| ServiceError::Spec { message: e.to_string() })
    }

    /// Get a single spec by ID (summary detail).
    ///
    /// REQ: P8-svc-spec-084
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  spec_id_str must be a valid UUID; ctx.spec_store() must be initialized
    /// post: returns SpecDetail with spec_id, name, category, domain_anchor, and flattened requirements; Err on invalid ID or store error
    pub fn get_by_id(ctx: &AgentService, spec_id_str: &str) -> Result<SpecDetail, ServiceError> {
        let id = parse_spec_id(spec_id_str)?;
        let store = ctx.spec_store();
        let spec = store.load(id).map_err(|e| ServiceError::Spec { message: e.to_string() })?;
        let requirements: Vec<String> = spec
            .goals
            .iter()
            .flat_map(|g| g.criteria.iter().map(|c| c.description.clone()))
            .collect();
        Ok(SpecDetail {
            spec_id: spec.id.to_string(),
            name: spec.name,
            category: spec.category.as_str().to_string(),
            domain_anchor: spec.domain_anchor.as_str().to_string(),
            requirements,
        })
    }

    /// Compute category coverage ratio across all specs — fast check for CLI/API.
    ///
    /// This is distinct from the MCP server's `spec_graph_coherence` which uses
    /// Jaccard similarity via `Spec::collection_coherence` for agent-driven assessment.
    ///
    /// REQ: P8-svc-spec-085
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx.spec_store() must be initialized
    /// post: returns CoherenceResult with coherence_score (0.0–1.0), missing category violations, and suggestions; score=0.0 when store is empty
    pub fn category_coverage(ctx: &AgentService) -> Result<CoherenceResult, ServiceError> {
        let store = ctx.spec_store();
        let specs = store.list_all().map_err(|e| ServiceError::Spec { message: e.to_string() })?;

        if specs.is_empty() {
            return Ok(CoherenceResult {
                coherence_score: 0.0,
                violations: vec!["No specifications in collection".to_string()],
                suggestions: SpecCategory::all()
                    .iter()
                    .map(|c| format!("Missing category: {}", c.as_str()))
                    .collect(),
            });
        }

        let covered_categories: std::collections::HashSet<SpecCategory> =
            specs.iter().map(|s| s.category).collect();
        let missing_categories: Vec<String> = SpecCategory::all()
            .iter()
            .filter(|c| !covered_categories.contains(c))
            .map(|c| format!("Missing category: {}", c.as_str()))
            .collect();
        let covered = SpecCategory::all().len() - missing_categories.len();
        let coherence_score = covered as f64 / SpecCategory::all().len() as f64;

        Ok(CoherenceResult {
            coherence_score,
            violations: missing_categories,
            suggestions: if coherence_score < 1.0 {
                vec!["Add at least one specification per MDS category".to_string()]
            } else {
                vec![]
            },
        })
    }

    /// Structural quality check for a spec — fast boolean assessment for CLI/API.
    ///
    /// Checks four dimensions: has_name, has_category, has_criteria, has_completeness.
    /// This is distinct from the MCP server's `assess_writing_quality` which performs
    /// embedding-based comparison against persona centroids for agent-driven assessment.
    ///
    /// REQ: P8-svc-spec-086
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  spec_id_str must be a valid UUID; ctx.spec_store() must be initialized
    /// post: returns WritingQualityResult with dimensions_passing count and meets_publication_standard flag (true when all 4 dimensions pass)
    pub fn structural_quality_check(
        ctx: &AgentService,
        spec_id_str: &str,
    ) -> Result<WritingQualityResult, ServiceError> {
        let id = parse_spec_id(spec_id_str)?;
        let store = ctx.spec_store();
        let spec = store.load(id).map_err(|e| ServiceError::Spec { message: e.to_string() })?;

        let dimensions = [
            ("has_name", !spec.name.is_empty()),
            ("has_category", true),
            (
                "has_criteria",
                !spec.goals.iter().all(|g| g.criteria.is_empty()),
            ),
            ("has_completeness", spec.is_complete()),
        ];
        let dimensions_passing = dimensions.iter().filter(|(_, pass)| *pass).count();

        Ok(WritingQualityResult {
            dimensions_passing,
            meets_publication_standard: dimensions_passing == dimensions.len(),
        })
    }

    /// Validate a specification against the default curator's criteria.
    ///
    /// Loads the spec by ID, then delegates to `DefaultSpecCurator::evaluate()`.
    /// This is the single method for spec evaluation; former `cultivate` call sites
    /// should use `validate` directly (the methods were identical).
    ///
    /// REQ: P8-svc-spec-087
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  spec_id_str must be a valid UUID; ctx.spec_store() must be initialized
    /// post: returns SpecCurationRecord from DefaultSpecCurator evaluation; Err on invalid ID or store/curation error
    pub fn validate(
        ctx: &AgentService,
        spec_id_str: &str,
    ) -> Result<SpecCurationRecord, ServiceError> {
        let id = parse_spec_id(spec_id_str)?;
        let store = ctx.spec_store();
        let spec = store.load(id).map_err(|e| ServiceError::Spec { message: e.to_string() })?;
        let curator = DefaultSpecCurator::default();
        curator.evaluate(&spec, &[]).map_err(|e| ServiceError::Spec { message: e.to_string() })
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Parse a spec ID string into an `hkask_storage::spec_types::SpecId`.
fn parse_spec_id(s: &str) -> Result<SpecId, ServiceError> {
    use uuid::Uuid;
    Uuid::parse_str(s)
        .map(SpecId)
        .map_err(|_| ServiceError::ValidationError {
            message: format!("Invalid spec ID '{}'", s),
        })
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: P8-svc-spec-001 — infer_spec_category maps context keywords to MDS categories
    #[test]
    fn infer_category_maps_trust_context() {
        assert_eq!(infer_spec_category(Some("trust")), SpecCategory::Trust);
        assert_eq!(
            infer_spec_category(Some("Security review")),
            SpecCategory::Trust
        );
        assert_eq!(
            infer_spec_category(Some("threat model")),
            SpecCategory::Trust
        );
    }

    // REQ: P8-svc-spec-001-1 — infer category maps composition context
    #[test]
    fn infer_category_maps_composition_context() {
        assert_eq!(
            infer_spec_category(Some("compose interface")),
            SpecCategory::Composition
        );
        assert_eq!(
            infer_spec_category(Some("API design")),
            SpecCategory::Composition
        );
    }

    // REQ: P8-svc-spec-001-2 — infer category maps lifecycle context
    #[test]
    fn infer_category_maps_lifecycle_context() {
        assert_eq!(
            infer_spec_category(Some("lifecycle bootstrap")),
            SpecCategory::Lifecycle
        );
        assert_eq!(
            infer_spec_category(Some("evolve spec")),
            SpecCategory::Lifecycle
        );
    }

    // REQ: P8-svc-spec-001-3 — infer category maps curation context
    #[test]
    fn infer_category_maps_curation_context() {
        assert_eq!(
            infer_spec_category(Some("curation review")),
            SpecCategory::Curation
        );
        assert_eq!(
            infer_spec_category(Some("coherence check")),
            SpecCategory::Curation
        );
    }

    // REQ: P8-svc-spec-001-4 — infer category defaults to domain
    #[test]
    fn infer_category_defaults_to_domain() {
        assert_eq!(infer_spec_category(None), SpecCategory::Domain);
        assert_eq!(
            infer_spec_category(Some("unknown stuff")),
            SpecCategory::Domain
        );
    }

    // REQ: P8-svc-spec-002 — parse_spec_id validates UUID format
    #[test]
    fn parse_spec_id_rejects_invalid() {
        assert!(parse_spec_id("not-a-uuid").is_err());
    }

    #[test]
    fn parse_spec_id_accepts_valid_uuid() {
        let valid = uuid::Uuid::new_v4().to_string();
        assert!(parse_spec_id(&valid).is_ok());
    }
}
