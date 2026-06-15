//! SpecService — specification capture, listing, and coherence for CLI and API surfaces.
//!
//! Unifies the divergent capture semantics: CLI uses explicit name + category +
//! domain + comma-separated criteria; API uses description + context (auto-inferred
//! category via keyword matching). Both paths produce a `Spec` stored via
//! `AgentService::spec_store()`.
//!
//! The `infer_category()` helper (moved from `hkask-api/src/routes/spec.rs`)
//! is the single source of truth for context-keyword → MDS category mapping.

use hkask_agents::DefaultSpecCurator;
use hkask_storage::SpecStore;
use hkask_storage::spec_types::SpecCurator;
use hkask_storage::spec_types::{
    DomainAnchor, GoalSpec, Spec, SpecCategory, SpecCurationRecord, SpecId,
};

use crate::AgentService;
use crate::error::ServiceError;

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
    pub fn capture(
        ctx: &AgentService,
        req: SpecCaptureRequest,
    ) -> Result<SpecCaptureResponse, ServiceError> {
        let cat = match req.category.as_deref() {
            Some(c) => SpecCategory::parse_str(c).unwrap_or(SpecCategory::Domain),
            None => infer_category(req.context.as_deref()),
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
        store.save(&spec).map_err(ServiceError::Spec)?;

        Ok(SpecCaptureResponse {
            spec_id: spec.id.to_string(),
            name: spec.name,
            category: spec.category.as_str().to_string(),
            domain_anchor: spec.domain_anchor.as_str().to_string(),
            complete: is_complete,
        })
    }

    /// List all specs, optionally filtered by category.
    pub fn list(
        ctx: &AgentService,
        category_filter: Option<&str>,
    ) -> Result<Vec<SpecListEntry>, ServiceError> {
        let store = ctx.spec_store();
        let specs = match category_filter {
            Some(cat_str) => {
                let cat = SpecCategory::parse_str(cat_str).ok_or_else(|| {
                    ServiceError::ValidationError(format!(
                        "Unknown category '{}': valid: domain, composition, trust, lifecycle, curation",
                        cat_str
                    ))
                })?;
                store.list_by_category(cat).map_err(ServiceError::Spec)?
            }
            None => store.list_all().map_err(ServiceError::Spec)?,
        };
        Ok(specs.into_iter().map(SpecListEntry::from).collect())
    }

    /// Get a single spec by ID.
    pub fn get_by_id(ctx: &AgentService, spec_id_str: &str) -> Result<SpecDetail, ServiceError> {
        let id = parse_spec_id(spec_id_str)?;
        let store = ctx.spec_store();
        let spec = store.load(id).map_err(ServiceError::Spec)?;
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

    /// Compute collection coherence — category coverage ratio across all specs.
    pub fn coherence(ctx: &AgentService) -> Result<CoherenceResult, ServiceError> {
        let store = ctx.spec_store();
        let specs = store.list_all().map_err(ServiceError::Spec)?;

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

        let category_coverage: std::collections::HashSet<SpecCategory> =
            specs.iter().map(|s| s.category).collect();
        let missing_categories: Vec<String> = SpecCategory::all()
            .iter()
            .filter(|c| !category_coverage.contains(c))
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

    /// Writing quality assessment for a spec.
    pub fn writing_quality(
        ctx: &AgentService,
        spec_id_str: &str,
    ) -> Result<WritingQualityResult, ServiceError> {
        let id = parse_spec_id(spec_id_str)?;
        let store = ctx.spec_store();
        let spec = store.load(id).map_err(ServiceError::Spec)?;

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

    /// Evaluate (validate) a specification against the default curator's criteria.
    ///
    /// Loads the spec by ID, then delegates to `DefaultSpecCurator::evaluate()`.
    pub fn validate(
        ctx: &AgentService,
        spec_id_str: &str,
    ) -> Result<SpecCurationRecord, ServiceError> {
        let id = parse_spec_id(spec_id_str)?;
        let store = ctx.spec_store();
        let spec = store.load(id).map_err(ServiceError::Spec)?;
        let curator = DefaultSpecCurator::default();
        curator.evaluate(&spec, &[]).map_err(ServiceError::Spec)
    }

    /// Cultivate a specification — same evaluation path as validate.
    ///
    /// Cultivation and validation share the same curator pipeline;
    /// separate methods exist for semantic clarity in call sites.
    pub fn cultivate(
        ctx: &AgentService,
        spec_id_str: &str,
    ) -> Result<SpecCurationRecord, ServiceError> {
        Self::validate(ctx, spec_id_str)
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Infer spec category from context string keywords.
///
/// Moved from `hkask-api/src/routes/spec.rs` — this is the single source
/// of truth for context-keyword → MDS category mapping.
fn infer_category(context: Option<&str>) -> SpecCategory {
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

/// Parse a spec ID string into an `hkask_storage::spec_types::SpecId`.
fn parse_spec_id(s: &str) -> Result<SpecId, ServiceError> {
    use uuid::Uuid;
    Uuid::parse_str(s)
        .map(SpecId)
        .map_err(|_| ServiceError::ValidationError(format!("Invalid spec ID '{}'", s)))
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: MDS-spec-svc-001 — infer_category maps context keywords to MDS categories
    #[test]
    fn infer_category_maps_trust_context() {
        assert_eq!(infer_category(Some("trust")), SpecCategory::Trust);
        assert_eq!(infer_category(Some("Security review")), SpecCategory::Trust);
        assert_eq!(infer_category(Some("threat model")), SpecCategory::Trust);
    }

    #[test]
    fn infer_category_maps_composition_context() {
        assert_eq!(
            infer_category(Some("compose interface")),
            SpecCategory::Composition
        );
        assert_eq!(
            infer_category(Some("API design")),
            SpecCategory::Composition
        );
    }

    #[test]
    fn infer_category_maps_lifecycle_context() {
        assert_eq!(
            infer_category(Some("lifecycle bootstrap")),
            SpecCategory::Lifecycle
        );
        assert_eq!(infer_category(Some("evolve spec")), SpecCategory::Lifecycle);
    }

    #[test]
    fn infer_category_maps_curation_context() {
        assert_eq!(
            infer_category(Some("curation review")),
            SpecCategory::Curation
        );
        assert_eq!(
            infer_category(Some("coherence check")),
            SpecCategory::Curation
        );
    }

    #[test]
    fn infer_category_defaults_to_domain() {
        assert_eq!(infer_category(None), SpecCategory::Domain);
        assert_eq!(infer_category(Some("unknown stuff")), SpecCategory::Domain);
    }

    // REQ: MDS-spec-svc-002 — parse_spec_id validates UUID format
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
