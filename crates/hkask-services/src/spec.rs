//! SpecService — specification capture, listing, decomposition, graph query,
//! coherence analysis, contract lifecycle, and test running.
//!
//! Shared by CLI, API, and MCP surfaces. Methods accept either `&AgentService`
//! (CLI/API convenience) or raw primitives (`&dyn SpecStore`, `&TripleStore`,
//! `&dyn NuEventSink`) for MCP server use.
//!
//! Pure business logic (no I/O) lives in `hkask_storage::spec_ops`.

use hkask_agents::DefaultSpecCurator;
use hkask_cns::{
    emit_contract_accepted, emit_contract_proposed, emit_contract_rejected, emit_contract_violated,
};
use hkask_storage::spec_ops::*;
use hkask_storage::spec_types::SpecCurator;
use hkask_storage::spec_types::{
    DomainAnchor, GoalSpec, Spec, SpecCategory, SpecCurationRecord, SpecId, infer_spec_category,
};
use hkask_storage::{NuEventStore, SpecStore, TripleStore};
use hkask_types::WebID;
use hkask_types::event::NuEventSink;

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
        store.save(&spec).map_err(|e| ServiceError::Spec {
            message: e.to_string(),
        })?;

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
                        source: None,
                        message: format!(
                            "Unknown category '{}': valid: domain, composition, trust, lifecycle, curation",
                            cat_str
                        ),
                    }
                })?;
                store
                    .list_by_category(cat)
                    .map_err(|e| ServiceError::Spec {
                        message: e.to_string(),
                    })?
            }
            None => store.list_all().map_err(|e| ServiceError::Spec {
                message: e.to_string(),
            })?,
        };
        Ok(specs.into_iter().map(SpecListEntry::from).collect())
    }

    /// Get a single spec by ID (full struct with goals).
    ///
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  spec_id_str must be a valid UUID; ctx.spec_store() must be initialized
    /// post: returns the full Spec with goals on success; Err(ValidationError) on invalid UUID; Err(Spec) on store error
    pub fn get_full(ctx: &AgentService, spec_id_str: &str) -> Result<Spec, ServiceError> {
        let id = parse_spec_id(spec_id_str)?;
        let store = ctx.spec_store();
        store.load(id).map_err(|e| ServiceError::Spec {
            message: e.to_string(),
        })
    }

    /// Get a single spec by ID (summary detail).
    ///
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  spec_id_str must be a valid UUID; ctx.spec_store() must be initialized
    /// post: returns SpecDetail with spec_id, name, category, domain_anchor, and flattened requirements; Err on invalid ID or store error
    pub fn get_by_id(ctx: &AgentService, spec_id_str: &str) -> Result<SpecDetail, ServiceError> {
        let id = parse_spec_id(spec_id_str)?;
        let store = ctx.spec_store();
        let spec = store.load(id).map_err(|e| ServiceError::Spec {
            message: e.to_string(),
        })?;
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
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  ctx.spec_store() must be initialized
    /// post: returns CoherenceResult with coherence_score (0.0–1.0), missing category violations, and suggestions; score=0.0 when store is empty
    pub fn category_coverage(ctx: &AgentService) -> Result<CoherenceResult, ServiceError> {
        let store = ctx.spec_store();
        let specs = store.list_all().map_err(|e| ServiceError::Spec {
            message: e.to_string(),
        })?;

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
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  spec_id_str must be a valid UUID; ctx.spec_store() must be initialized
    /// post: returns WritingQualityResult with dimensions_passing count and meets_publication_standard flag (true when all 4 dimensions pass)
    pub fn structural_quality_check(
        ctx: &AgentService,
        spec_id_str: &str,
    ) -> Result<WritingQualityResult, ServiceError> {
        let id = parse_spec_id(spec_id_str)?;
        let store = ctx.spec_store();
        let spec = store.load(id).map_err(|e| ServiceError::Spec {
            message: e.to_string(),
        })?;

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
    /// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  spec_id_str must be a valid UUID; ctx.spec_store() must be initialized
    /// post: returns SpecCurationRecord from DefaultSpecCurator evaluation; Err on invalid ID or store/curation error
    pub fn validate(
        ctx: &AgentService,
        spec_id_str: &str,
    ) -> Result<SpecCurationRecord, ServiceError> {
        let id = parse_spec_id(spec_id_str)?;
        let store = ctx.spec_store();
        let spec = store.load(id).map_err(|e| ServiceError::Spec {
            message: e.to_string(),
        })?;
        let curator = DefaultSpecCurator::default();
        curator
            .evaluate(&spec, &[])
            .map_err(|e| ServiceError::Spec {
                message: e.to_string(),
            })
    }

    // ── Store-primitive variant: capture directly to a SpecStore ──────

    /// Capture a spec directly to a store.
    /// Same logic as `capture` but takes `&dyn SpecStore` for MCP server use.
    pub fn capture_to_store(
        store: &dyn SpecStore,
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
        store.save(&spec).map_err(|e| ServiceError::Spec {
            message: e.to_string(),
        })?;
        Ok(SpecCaptureResponse {
            spec_id: spec.id.to_string(),
            name: spec.name,
            category: spec.category.as_str().to_string(),
            domain_anchor: spec.domain_anchor.as_str().to_string(),
            complete: is_complete,
        })
    }

    /// Load a spec by ID string from a store.
    pub fn load_spec(store: &dyn SpecStore, spec_id_str: &str) -> Result<Spec, ServiceError> {
        let id = parse_spec_id(spec_id_str)?;
        store.load(id).map_err(|e| ServiceError::Spec {
            message: e.to_string(),
        })
    }

    // ── Goal decomposition ───────────────────────────────────────────

    pub fn decompose(
        store: &dyn SpecStore,
        spec_id_str: &str,
    ) -> Result<(Vec<String>, Vec<DependencyEdge>), ServiceError> {
        let mut spec = Self::load_spec(store, spec_id_str)?;
        decompose_spec_goals(&mut spec);
        let (sub_goals, dependencies) = collect_subgoals_and_deps(&spec);
        store.save(&spec).map_err(|e| ServiceError::Spec {
            message: e.to_string(),
        })?;
        Ok((sub_goals, dependencies))
    }

    // ── Writing quality (heuristic) ──────────────────────────────────

    pub fn writing_quality_heuristic(
        store: &dyn SpecStore,
        spec_id_str: &str,
    ) -> Result<HeuristicWritingQuality, ServiceError> {
        let spec = Self::load_spec(store, spec_id_str)?;
        Ok(assess_writing_quality_heuristic(&spec))
    }

    // ── Graph query ──────────────────────────────────────────────────

    pub fn graph_query(
        store: &dyn SpecStore,
        query: &str,
        max_depth: u8,
    ) -> Result<GraphQueryResult, ServiceError> {
        let specs = store.list_all().map_err(|e| ServiceError::Spec {
            message: e.to_string(),
        })?;
        Ok(query_spec_graph(&specs, query, max_depth))
    }

    // ── Collection coherence ─────────────────────────────────────────

    pub fn graph_coherence(
        store: &dyn SpecStore,
        threshold: f64,
    ) -> Result<CoherenceCheck, ServiceError> {
        let specs = store.list_all().map_err(|e| ServiceError::Spec {
            message: e.to_string(),
        })?;
        Ok(compute_collection_coherence(&specs, threshold))
    }

    // ── Contract lifecycle ───────────────────────────────────────────

    pub fn contract_propose(
        event_sink: &dyn NuEventSink,
        triple_store: &TripleStore,
        replicant: &str,
        crate_name: &str,
        function: &str,
        contract_id: &str,
        pre: &str,
        post: &str,
    ) -> Result<(), ServiceError> {
        emit_contract_proposed(event_sink, replicant, crate_name, function, contract_id);
        let value = serde_json::json!({
            "replicant": replicant,
            "crate": crate_name,
            "function": function,
            "contract_id": contract_id,
            "pre": pre,
            "post": post,
            "status": "proposed",
            "proposed_at": chrono::Utc::now().to_rfc3339(),
        });
        let triple = hkask_storage::Triple::new(
            "cns:contract_proposal",
            contract_id,
            value,
            WebID::from_persona(replicant.as_bytes()),
        );
        let _ = triple_store.insert(&triple);
        Ok(())
    }

    pub fn contract_accept(
        event_sink: &dyn NuEventSink,
        triple_store: &TripleStore,
        reviewer: &str,
        contract_id: &str,
    ) -> Result<(), ServiceError> {
        emit_contract_accepted(event_sink, reviewer, "", "", "", contract_id);
        if let Ok(mut existing) =
            triple_store.query_by_entity_attribute("cns:contract_proposal", contract_id)
        {
            if let Some(mut triple) = existing.pop() {
                let mut value = triple.value.clone();
                value["status"] = serde_json::json!("accepted");
                value["reviewer"] = serde_json::json!(reviewer);
                value["accepted_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
                triple.value = value.clone();
                let _ = triple_store.update(&triple.id, value, hkask_types::Confidence::full());
            }
        }
        Ok(())
    }

    pub fn contract_reject(
        event_sink: &dyn NuEventSink,
        triple_store: &TripleStore,
        reviewer: &str,
        contract_id: &str,
        reason: &str,
    ) -> Result<(), ServiceError> {
        emit_contract_rejected(event_sink, reviewer, "", "", "", contract_id, reason);
        if let Ok(mut existing) =
            triple_store.query_by_entity_attribute("cns:contract_proposal", contract_id)
        {
            if let Some(mut triple) = existing.pop() {
                let mut value = triple.value.clone();
                value["status"] = serde_json::json!("rejected");
                value["reviewer"] = serde_json::json!(reviewer);
                value["reason"] = serde_json::json!(reason);
                value["rejected_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
                triple.value = value.clone();
                let _ = triple_store.update(&triple.id, value, hkask_types::Confidence::full());
            }
        }
        Ok(())
    }

    pub fn contract_list(
        triple_store: &TripleStore,
    ) -> Result<
        Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
        )>,
        ServiceError,
    > {
        let proposals = triple_store
            .query_by_entity("cns:contract_proposal")
            .unwrap_or_default();
        let entries = proposals
            .iter()
            .map(|t| {
                (
                    t.value["contract_id"].as_str().unwrap_or("?").to_string(),
                    t.value["status"].as_str().unwrap_or("unknown").to_string(),
                    t.value["function"].as_str().unwrap_or("?").to_string(),
                    t.value["crate"].as_str().unwrap_or("?").to_string(),
                    t.value["pre"].as_str().unwrap_or("").to_string(),
                    t.value["post"].as_str().unwrap_or("").to_string(),
                    t.value["replicant"].as_str().map(|s| s.to_string()),
                )
            })
            .collect();
        Ok(entries)
    }

    // ── Contract audit ───────────────────────────────────────────────

    pub fn contract_audit(
        crate_name: Option<&str>,
        workspace_root: &str,
    ) -> Result<Vec<hkask_test_harness::test_runner::CrateAudit>, ServiceError> {
        let crates: Vec<String> = if let Some(c) = crate_name {
            vec![c.to_string()]
        } else {
            let crates_dir = std::path::Path::new(workspace_root).join("crates");
            let entries = std::fs::read_dir(&crates_dir).map_err(|e| ServiceError::Spec {
                message: format!("Cannot read crates dir {}: {}", crates_dir.display(), e),
            })?;
            entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                .filter(|s| s.starts_with("hkask-"))
                .collect()
        };

        let mut results = Vec::new();
        for name in &crates {
            if let Some(audit) = hkask_test_harness::test_runner::discover_uncontracted_functions(
                name,
                workspace_root,
            ) {
                results.push(audit);
            }
        }
        Ok(results)
    }

    // ── Test runner ──────────────────────────────────────────────────

    pub fn test_run(
        crate_name: &str,
        workspace_root: &str,
    ) -> Result<hkask_test_harness::test_runner::TestRunResult, ServiceError> {
        hkask_test_harness::test_runner::run_contract_tests(crate_name, workspace_root).ok_or_else(
            || ServiceError::Spec {
                message: format!("cargo test unavailable for crate '{}'", crate_name),
            },
        )
    }

    pub fn emit_test_violations(
        event_sink: &dyn NuEventSink,
        violations: &[hkask_test_harness::test_runner::TestViolation],
    ) {
        for v in violations {
            emit_contract_violated(event_sink, &v.test_name, &v.contract_id, &v.failure_reason);
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Parse a spec ID string into an `hkask_storage::spec_types::SpecId`.
fn parse_spec_id(s: &str) -> Result<SpecId, ServiceError> {
    use uuid::Uuid;
    Uuid::parse_str(s)
        .map(SpecId)
        .map_err(|_| ServiceError::ValidationError {
            source: None,
            message: format!("Invalid spec ID '{}'", s),
        })
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // contract: P8-svc-spec-001
    // expect: "Service infer_spec_category works correctly under test conditions" [P8]
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

    // contract: P8-svc-spec-001-1
    // expect: "Service infer_category works correctly under test conditions" [P8]
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

    // contract: P8-svc-spec-001-2
    // expect: "Service infer_category works correctly under test conditions" [P8]
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

    // contract: P8-svc-spec-001-3
    // expect: "Service infer_category works correctly under test conditions" [P8]
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

    // contract: P8-svc-spec-001-4
    // expect: "Service infer_category works correctly under test conditions" [P8]
    #[test]
    fn infer_category_defaults_to_domain() {
        assert_eq!(infer_spec_category(None), SpecCategory::Domain);
        assert_eq!(
            infer_spec_category(Some("unknown stuff")),
            SpecCategory::Domain
        );
    }

    // contract: P8-svc-spec-002
    // expect: "Service parse_spec_id works correctly under test conditions" [P8]
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
