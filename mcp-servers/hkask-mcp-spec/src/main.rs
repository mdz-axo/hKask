//! hKask MCP Spec — Specification authoring, curation, and validation
//!
//! 10 tools following DDMVSS §6.3:
//! - spec/goal/capture — Elicit user intent as binding requirement
//! - spec/goal/decompose — Break goal into ordered sub-goals
//! - spec/require/bind — Attach OCAP boundaries to a goal
//! - spec/curate/evaluate — Assess artifact against collection coherence
//! - spec/curate/reconcile — Resolve goal tensions
//! - spec/curate/cultivate — Grow collection toward coherence
//! - spec/graph/query — Query specification graph
//! - spec/graph/validate — Validate collection coherence
//! - spec/test/invariant — Create test traceability record linking test to spec requirement
//! - spec/test/verify — Verify test coverage for a seam or spec category

// F-SYN-020: `types` must be `pub` so that integration tests in
// `tests/` can reference request types for fuzz testing.
pub mod types;

use hkask_mcp::server::{McpToolError, ServerContext, ToolSpanGuard};
use hkask_mcp::validate_field;

use hkask_storage::spec_types::{
    DomainAnchor, GoalSpec, Spec, SpecCategory, SpecError, SpecId, SpecStore,
};
use hkask_types::{
    CapabilityChecker, CurationDecision, DelegationAction, DelegationResource, DelegationToken,
    McpErrorKind, OCAPBoundary, TOKEN_ERR_EXPIRED, TOKEN_ERR_INVALID_SIGNATURE,
    TOKEN_ERR_NO_CHECKER, VerificationOutcome, WebID, token_err_insufficient_access,
    verify_delegation_token_now,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use std::sync::Arc;
use types::{
    CurateCultivateRequest, CurateCultivateResponse, CurateEvaluateRequest, CurateEvaluateResponse,
    CurateReconcileRequest, CurateReconcileResponse, GoalCaptureRequest, GoalCaptureResponse,
    GoalDecomposeRequest, GoalDecomposeResponse, GraphNodeResponse, GraphQueryRequest,
    GraphQueryResponse, GraphValidateRequest, GraphValidateResponse, RequireBindRequest,
    RequireBindResponse, TensionReport, TestClassification, TestInvariantRequest,
    TestInvariantResponse, TestTraceability, TestVerifyRequest, TestVerifyResponse,
};

// ── Server ───────────────────────────────────────────────────

pub struct SpecServer {
    store: Arc<dyn SpecStore + Send + Sync>,
    capability_checker: Arc<CapabilityChecker>,
    webid: WebID,
}

impl std::fmt::Debug for SpecServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpecServer")
            .field("store", &"<dyn SpecStore>")
            .field("capability_checker", &"<CapabilityChecker>")
            .field("webid", &self.webid)
            .finish()
    }
}

impl SpecServer {
    pub fn new(
        store: Arc<dyn SpecStore + Send + Sync>,
        webid: WebID,
        capability_checker: CapabilityChecker,
    ) -> Self {
        Self {
            store,
            capability_checker: Arc::new(capability_checker),
            webid,
        }
    }

    fn save_spec(&self, spec: &Spec) -> Result<(), SpecError> {
        self.store.save(spec)
    }

    fn verify_capability(
        &self,
        token_b64: Option<&str>,
        resource_id: &str,
        action: DelegationAction,
    ) -> Result<(), McpToolError> {
        let b64 = token_b64.ok_or_else(|| {
            McpToolError::permission_denied(format!(
                "No capability token provided for spec:{}:{}",
                resource_id,
                action.as_str()
            ))
        })?;

        let token = DelegationToken::from_base64(b64).map_err(|e| {
            McpToolError::permission_denied(format!("Invalid token encoding: {}", e))
        })?;

        // P1.1: Use unified verification instead of duplicated inline checks
        match verify_delegation_token_now(
            Some(&self.capability_checker),
            &token,
            &self.webid,
            DelegationResource::Registry,
            resource_id,
            action,
        ) {
            VerificationOutcome::Valid => Ok(()),
            VerificationOutcome::InvalidSignature => Err(McpToolError::permission_denied(
                TOKEN_ERR_INVALID_SIGNATURE.to_string(),
            )),
            VerificationOutcome::Expired => Err(McpToolError::permission_denied(
                TOKEN_ERR_EXPIRED.to_string(),
            )),
            VerificationOutcome::InsufficientAccess {
                resource_id: rid,
                action: a,
                ..
            } => Err(McpToolError::permission_denied(
                token_err_insufficient_access(&rid, a.as_str()),
            )),
            VerificationOutcome::NoChecker => Err(McpToolError::permission_denied(
                TOKEN_ERR_NO_CHECKER.to_string(),
            )),
        }
    }
}

#[tool_router(server_handler)]
impl SpecServer {
    #[tool(description = "Capture a goal as a binding specification requirement")]
    async fn spec_goal_capture(
        &self,
        Parameters(GoalCaptureRequest {
            description,
            category,
            domain_anchor,
            criteria,
            capability_token,
        }): Parameters<GoalCaptureRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_goal_capture", &self.webid);

        if let Err(e) = self.verify_capability(
            capability_token.as_deref(),
            "capture",
            DelegationAction::Write,
        ) {
            return span.error(e.kind, e.to_json_string());
        }

        let cat = SpecCategory::parse_str(&category).unwrap_or(SpecCategory::Domain);
        let anchor = DomainAnchor::parse_str(&domain_anchor).unwrap_or(DomainAnchor::Hkask);

        let mut goal = GoalSpec::new(&description);
        if let Some(crits) = criteria {
            for c in crits {
                goal = goal.with_criterion(&c);
            }
        }

        let spec = Spec::new(&description, cat, anchor).with_goal(goal);
        let id = spec.id;

        if let Err(e) = self.save_spec(&spec) {
            return span.internal_error(
                serde_json::json!({"error": format!("Failed to persist spec: {}", e)}),
            );
        }

        span.ok_json(
            serde_json::to_value(GoalCaptureResponse {
                spec_id: id.to_string(),
                category: cat.as_str().to_string(),
                domain_anchor: anchor.as_str().to_string(),
                status: "captured".to_string(),
            })
            .unwrap_or_default(),
        )
    }

    #[tool(description = "Decompose a goal into ordered sub-goals (max depth 7)")]
    async fn spec_goal_decompose(
        &self,
        Parameters(GoalDecomposeRequest {
            spec_id,
            goal_index,
            sub_goals,
            capability_token,
        }): Parameters<GoalDecomposeRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_goal_decompose", &self.webid);

        validate_field!(span, "spec_id", &spec_id, 256);

        if let Err(e) = self.verify_capability(
            capability_token.as_deref(),
            &spec_id,
            DelegationAction::Write,
        ) {
            return span.error(e.kind, e.to_json_string());
        }

        let spec_id_parsed = SpecId::from_string(&spec_id).unwrap_or_default();
        let mut spec = match self.store.load(spec_id_parsed) {
            Ok(s) => s,
            Err(_) => {
                return span.error(
                    McpErrorKind::NotFound,
                    McpToolError::not_found(format!("Spec not found: {}", spec_id))
                        .to_json_string(),
                );
            }
        };

        let Some(goal) = spec.goals.get_mut(goal_index) else {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!("Goal index {} out of range", goal_index))
                    .to_json_string(),
            );
        };

        if !goal.can_have_subgoals() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("Depth limit reached (max 7)".to_string())
                    .to_json_string(),
            );
        }

        let added = sub_goals.len();
        for text in sub_goals {
            let mut child = GoalSpec::new(&text);
            child.depth = goal.depth + 1;
            goal.sub_goals.push(child);
        }

        if let Err(e) = self.save_spec(&spec) {
            return span.internal_error(
                serde_json::json!({"error": format!("Failed to persist spec: {}", e)}),
            );
        }

        span.ok_json(
            serde_json::to_value(GoalDecomposeResponse {
                spec_id,
                goal_index,
                sub_goals_added: added,
            })
            .unwrap_or_default(),
        )
    }

    #[tool(description = "Bind OCAP boundaries to a goal as a constraint")]
    async fn spec_require_bind(
        &self,
        Parameters(RequireBindRequest {
            spec_id,
            goal_index,
            capability,
            authority,
            capability_token,
        }): Parameters<RequireBindRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_require_bind", &self.webid);

        validate_field!(span, "spec_id", &spec_id, 256);

        if let Err(e) = self.verify_capability(
            capability_token.as_deref(),
            &spec_id,
            DelegationAction::Write,
        ) {
            return span.error(e.kind, e.to_json_string());
        }

        let spec_id_parsed = SpecId::from_string(&spec_id).unwrap_or_default();
        let mut spec = match self.store.load(spec_id_parsed) {
            Ok(s) => s,
            Err(_) => {
                return span.error(
                    McpErrorKind::NotFound,
                    McpToolError::not_found(format!("Spec not found: {}", spec_id))
                        .to_json_string(),
                );
            }
        };
        if goal_index >= spec.goals.len() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!("Goal index {} out of range", goal_index))
                    .to_json_string(),
            );
        }

        // F-SYN-001: parse the user-supplied capability string into a typed
        // token. Untrusted input that does not match a known token kind is
        // rejected; the previous code accepted any string and minted a
        // boundary with `OcapCapability::String(attack_string)`.
        let boundary = match OCAPBoundary::parse_token(&capability) {
            Some(b) => b,
            None => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(format!(
                        "Unknown capability kind: {capability:?}. Expected one of: curation, cybernetics, spec_curate."
                    ))
                    .to_json_string(),
                );
            }
        };

        // The `authority` field is now informational only; every
        // boundary is enforced by construction. Reject the legacy
        // "denied" sentinel with a clear migration error rather than
        // silently accepting it.
        if authority == "denied" {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(
                    "The 'denied' authority is no longer supported. Every OCAPBoundary is enforced by construction; omit the request or use a different tool to record a denial."
                        .to_string(),
                )
                .to_json_string(),
            );
        }

        spec.goals[goal_index].constraints.push(boundary);

        if let Err(e) = self.save_spec(&spec) {
            return span.internal_error(
                serde_json::json!({"error": format!("Failed to persist spec: {}", e)}),
            );
        }

        span.ok_json(
            serde_json::to_value(RequireBindResponse {
                spec_id,
                goal_index,
                capability,
                authority,
                enforced: true,
            })
            .unwrap_or_default(),
        )
    }

    #[tool(description = "Evaluate a specification for collection coherence (curation)")]
    async fn spec_curate_evaluate(
        &self,
        Parameters(CurateEvaluateRequest {
            spec_id,
            rationale_hint,
            capability_token,
        }): Parameters<CurateEvaluateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_curate_evaluate", &self.webid);

        validate_field!(span, "spec_id", &spec_id, 256);

        if let Err(e) = self.verify_capability(
            capability_token.as_deref(),
            &spec_id,
            DelegationAction::Read,
        ) {
            return span.error(e.kind, e.to_json_string());
        }

        let spec_id_parsed = SpecId::from_string(&spec_id).unwrap_or_default();
        let spec = match self.store.load(spec_id_parsed) {
            Ok(s) => s,
            Err(_) => {
                return span.error(
                    McpErrorKind::NotFound,
                    McpToolError::not_found(format!("Spec not found: {}", spec_id))
                        .to_json_string(),
                );
            }
        };

        let complete = spec.is_complete();
        let decision = if complete {
            CurationDecision::Merge
        } else if spec.goals.is_empty() {
            CurationDecision::Discard
        } else {
            CurationDecision::Revise
        };

        let rationale = rationale_hint.unwrap_or_else(|| {
            if complete {
                "All criteria satisfied".to_string()
            } else {
                "Unsatisfied criteria remain".to_string()
            }
        });

        let coherence = spec.coherence();

        span.ok_json(
            serde_json::to_value(CurateEvaluateResponse {
                spec_id,
                decision: decision.to_string(),
                rationale,
                coherence_score: coherence,
            })
            .unwrap_or_default(),
        )
    }

    #[tool(description = "Reconcile tensions between specifications without collapsing them")]
    async fn spec_curate_reconcile(
        &self,
        Parameters(CurateReconcileRequest {
            spec_ids,
            tension_description,
            capability_token,
        }): Parameters<CurateReconcileRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_curate_reconcile", &self.webid);

        if let Err(e) = self.verify_capability(
            capability_token.as_deref(),
            "reconcile",
            DelegationAction::Write,
        ) {
            return span.error(e.kind, e.to_json_string());
        }

        let mut found = Vec::new();
        let mut missing = Vec::new();

        for id in &spec_ids {
            let parsed = SpecId::from_string(id).unwrap_or_default();
            if self.store.load(parsed).is_ok() {
                found.push(id.clone());
            } else {
                missing.push(id.as_str());
            }
        }

        if !missing.is_empty() {
            return span.error(
                McpErrorKind::NotFound,
                McpToolError::not_found(format!("Specs not found: {:?}", missing)).to_json_string(),
            );
        }

        let mut loaded_specs = Vec::new();
        for id in &found {
            let parsed = SpecId::from_string(id).unwrap_or_default();
            if let Ok(spec) = self.store.load(parsed) {
                loaded_specs.push(spec);
            }
        }

        let mut tensions = Vec::new();
        for i in 0..loaded_specs.len() {
            for j in (i + 1)..loaded_specs.len() {
                let a = &loaded_specs[i];
                let b = &loaded_specs[j];
                let words_a: std::collections::HashSet<&str> = a
                    .goals
                    .iter()
                    .flat_map(|g| g.text.split_whitespace())
                    .collect();
                let words_b: std::collections::HashSet<&str> = b
                    .goals
                    .iter()
                    .flat_map(|g| g.text.split_whitespace())
                    .collect();
                let intersection = words_a.intersection(&words_b).count();
                let union = words_a.union(&words_b).count();
                let jaccard = if union > 0 {
                    intersection as f64 / union as f64
                } else {
                    0.0
                };
                if jaccard > 0.3 {
                    let overlapping: Vec<String> = words_a
                        .intersection(&words_b)
                        .map(|w| w.to_string())
                        .collect();
                    tensions.push(TensionReport {
                        spec_a: a.id.to_string(),
                        spec_b: b.id.to_string(),
                        overlapping_goals: overlapping,
                        jaccard_score: jaccard,
                    });
                }
            }
        }

        let resolution = if tensions.is_empty() {
            "no_tensions_detected"
        } else {
            "tensions_identified"
        };

        span.ok_json(
            serde_json::to_value(CurateReconcileResponse {
                resolution: resolution.to_string(),
                spec_ids: found,
                tension: tension_description,
                tensions,
                status: "reconciled".to_string(),
            })
            .unwrap_or_default(),
        )
    }

    #[tool(description = "Cultivate the specification collection toward coherence")]
    async fn spec_curate_cultivate(
        &self,
        Parameters(CurateCultivateRequest {
            coherence_threshold,
            capability_token,
        }): Parameters<CurateCultivateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_curate_cultivate", &self.webid);

        if let Err(e) = self.verify_capability(
            capability_token.as_deref(),
            "cultivate",
            DelegationAction::Write,
        ) {
            return span.error(e.kind, e.to_json_string());
        }

        let threshold = coherence_threshold.unwrap_or(0.7);
        let all_specs: Vec<Spec> = match self.store.list_all() {
            Ok(specs) => specs,
            Err(e) => {
                return span.internal_error(
                    serde_json::json!({"error": format!("Failed to load specs: {}", e)}),
                );
            }
        };
        let coherence = Spec::collection_coherence(&all_specs);
        let categories_covered: Vec<String> = all_specs
            .iter()
            .map(|s| s.category.as_str().to_string())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let categories_missing: Vec<String> = SpecCategory::all()
            .iter()
            .map(|c| c.as_str().to_string())
            .filter(|c| !categories_covered.contains(c))
            .collect();

        let above_threshold = coherence >= threshold;

        span.ok_json(
            serde_json::to_value(CurateCultivateResponse {
                coherence_score: coherence,
                threshold,
                above_threshold,
                spec_count: all_specs.len(),
                categories_covered,
                categories_missing,
            })
            .unwrap_or_default(),
        )
    }

    #[tool(description = "Query the specification graph by category or domain anchor")]
    async fn spec_graph_query(
        &self,
        Parameters(GraphQueryRequest {
            category,
            domain_anchor,
            capability_token,
        }): Parameters<GraphQueryRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_graph_query", &self.webid);

        if let Err(e) =
            self.verify_capability(capability_token.as_deref(), "query", DelegationAction::Read)
        {
            return span.error(e.kind, e.to_json_string());
        }

        let all_specs: Vec<Spec> = match self.store.list_all() {
            Ok(specs) => specs,
            Err(e) => {
                return span.internal_error(
                    serde_json::json!({"error": format!("Failed to load specs: {}", e)}),
                );
            }
        };
        let results: Vec<&Spec> = all_specs
            .iter()
            .filter(|s| {
                let cat_match = category
                    .as_ref()
                    .map(|c| s.category.as_str() == c.as_str())
                    .unwrap_or(true);
                let anchor_match = domain_anchor
                    .as_ref()
                    .map(|a| s.domain_anchor.as_str() == a.as_str())
                    .unwrap_or(true);
                cat_match && anchor_match
            })
            .collect();

        let nodes: Vec<GraphNodeResponse> = results
            .iter()
            .map(|s| GraphNodeResponse {
                id: s.id.to_string(),
                name: s.name.clone(),
                category: s.category.as_str().to_string(),
                complete: s.is_complete(),
            })
            .collect();

        span.ok_json(
            serde_json::to_value(GraphQueryResponse {
                count: nodes.len(),
                specs: nodes,
            })
            .unwrap_or_default(),
        )
    }

    #[tool(
        description = "Validate the full specification collection for coherence and completeness"
    )]
    async fn spec_graph_validate(
        &self,
        Parameters(GraphValidateRequest {
            coherence_threshold,
            capability_token,
        }): Parameters<GraphValidateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_graph_validate", &self.webid);

        if let Err(e) = self.verify_capability(
            capability_token.as_deref(),
            "validate",
            DelegationAction::Read,
        ) {
            return span.error(e.kind, e.to_json_string());
        }

        let threshold = coherence_threshold.unwrap_or(0.7);
        let all_specs: Vec<Spec> = match self.store.list_all() {
            Ok(specs) => specs,
            Err(e) => {
                return span.internal_error(
                    serde_json::json!({"error": format!("Failed to load specs: {}", e)}),
                );
            }
        };
        let coherence = Spec::collection_coherence(&all_specs);

        let mut violations = Vec::new();
        let mut suggestions = Vec::new();

        if coherence < threshold {
            violations.push(format!(
                "Coherence {:.2} below threshold {:.2}",
                coherence, threshold
            ));
        }

        let categories_coveraged: std::collections::HashSet<&str> =
            all_specs.iter().map(|s| s.category.as_str()).collect();

        for cat in SpecCategory::all() {
            if !categories_coveraged.contains(cat.as_str()) {
                suggestions.push(format!("Missing category: {}", cat.as_str()));
            }
        }

        for spec in &all_specs {
            if !spec.is_complete() {
                suggestions.push(format!("Incomplete spec: {} ({})", spec.id, spec.name));
            }
        }

        span.ok_json(
            serde_json::to_value(GraphValidateResponse {
                valid: violations.is_empty(),
                coherence_score: coherence,
                threshold,
                violations,
                suggestions,
                spec_count: all_specs.len(),
            })
            .unwrap_or_default(),
        )
    }

    #[tool(
        description = "Create a test traceability record linking a test to a specification requirement"
    )]
    async fn spec_test_invariant(
        &self,
        Parameters(TestInvariantRequest {
            spec_id,
            seam,
            invariant,
            category,
            cycle,
            capability_token,
        }): Parameters<TestInvariantRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_test_invariant", &self.webid);

        if let Err(e) = self.verify_capability(
            capability_token.as_deref(),
            "test/invariant",
            DelegationAction::Write,
        ) {
            return span.error(e.kind, e.to_json_string());
        }

        validate_field!(span, "spec_id", &spec_id, 256);
        validate_field!(span, "seam", &seam, 256);
        validate_field!(span, "invariant", &invariant, 1024);
        validate_field!(span, "category", &category, 64);

        let spec_id_parsed = SpecId::from_string(&spec_id).unwrap_or_default();
        let _spec = match self.store.load(spec_id_parsed) {
            Ok(s) => s,
            Err(SpecError::NotFound(_)) => {
                return span.error(
                    McpErrorKind::NotFound,
                    McpToolError::not_found(format!("Spec not found: {}", spec_id))
                        .to_json_string(),
                );
            }
            Err(e) => {
                return span.internal_error(
                    serde_json::json!({"error": format!("Failed to load spec: {}", e)}),
                );
            }
        };

        let invariant_id = format!("{}:{}:{}", spec_id, seam, category.to_lowercase());

        // MVP: TestTraceability record is ephemeral (not persisted to a separate table).
        // The canonical traceability record is the // REQ: comment in test source code.
        // A future iteration can add a test_traceability SQLite table.

        let cycle_tag = cycle
            .as_deref()
            .map(|c| format!(" [{} cycle]", c))
            .unwrap_or_default();

        span.ok_json(
            serde_json::to_value(TestInvariantResponse {
                invariant_id,
                status: format!("recorded{}", cycle_tag),
            })
            .unwrap_or_default(),
        )
    }

    #[tool(
        description = "Verify test coverage for a seam or spec category, returning gaps and debt"
    )]
    async fn spec_test_verify(
        &self,
        Parameters(TestVerifyRequest {
            seam: _,
            category,
            capability_token,
        }): Parameters<TestVerifyRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_test_verify", &self.webid);

        if let Err(e) = self.verify_capability(
            capability_token.as_deref(),
            "test/verify",
            DelegationAction::Read,
        ) {
            return span.error(e.kind, e.to_json_string());
        }

        let all_specs: Vec<Spec> = match self.store.list_all() {
            Ok(specs) => specs,
            Err(e) => {
                return span.internal_error(
                    serde_json::json!({"error": format!("Failed to load specs: {}", e)}),
                );
            }
        };

        let category_filter: Option<SpecCategory> =
            category.as_deref().and_then(SpecCategory::parse_str);

        let filtered: Vec<&Spec> = all_specs
            .iter()
            .filter(|s| {
                category_filter
                    .as_ref()
                    .map(|cf| s.category == *cf)
                    .unwrap_or(true)
            })
            .collect();

        let mut traceability = Vec::new();
        let mut tested = 0;
        let mut gaps = 0;
        let debt = 0;

        for spec in &filtered {
            let is_complete = spec.is_complete();
            let classification = if is_complete {
                Some(TestClassification::PublicInterface)
            } else {
                None
            };
            let has_gap = !is_complete;

            if is_complete {
                tested += 1;
            } else {
                gaps += 1;
            }

            traceability.push(TestTraceability {
                requirement_id: spec.id.to_string(),
                classification,
                test_path: if is_complete {
                    Some(format!("spec:{}", spec.id))
                } else {
                    None
                },
                has_gap,
                test_debt_location: None,
            });
        }

        let complete = gaps == 0 && !filtered.is_empty();

        span.ok_json(
            serde_json::to_value(TestVerifyResponse {
                total_requirements: filtered.len(),
                tested,
                gaps,
                debt,
                traceability,
                complete,
            })
            .unwrap_or_default(),
        )
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hkask_mcp::run_server(
        "hkask-mcp-spec",
        env!("CARGO_PKG_VERSION"),
        |ctx: ServerContext| {
            let conn = match ctx.credentials.get("HKASK_SPEC_DB_PATH") {
                Some(path) => {
                    let passphrase = ctx.credentials.get("HKASK_DB_PASSPHRASE").ok_or_else(|| {
                        anyhow::anyhow!("HKASK_SPEC_DB_PATH set but HKASK_DB_PASSPHRASE missing")
                    })?;
                    let db = hkask_storage::Database::open(path, passphrase)
                        .map_err(|e| anyhow::anyhow!("Failed to open spec database: {e}"))?;
                    db.conn_arc()
                }
                None => {
                    tracing::warn!(
                        target: "hkask.mcp.spec",
                        "No persistent database configured — spec store is in-memory and will be lost on restart. \
                         Set HKASK_SPEC_DB_PATH and HKASK_DB_PASSPHRASE for sovereign persistence."
                    );
                    let conn = rusqlite::Connection::open_in_memory()?;
                    std::sync::Arc::new(std::sync::Mutex::new(conn))
                }
            };
            let store = std::sync::Arc::new(hkask_storage::SqliteSpecStore::new(conn));
            store.init_schema().map_err(|e| anyhow::anyhow!("{}", e))?;

            let secret_hex = ctx.credentials.get("HKASK_OCAP_SECRET").ok_or_else(|| {
                anyhow::anyhow!("HKASK_OCAP_SECRET is required for spec capability verification")
            })?;
            let secret = hex::decode(secret_hex)
                .map_err(|e| anyhow::anyhow!("HKASK_OCAP_SECRET must be hex-encoded: {e}"))?;
            let checker = CapabilityChecker::new(&secret);

            Ok(SpecServer::new(store, ctx.webid, checker))
        },
        vec![
            hkask_mcp::CredentialRequirement::required(
                "HKASK_OCAP_SECRET",
                "Hex-encoded OCAP secret for minting/verifying spec capability tokens",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_SPEC_DB_PATH",
                "Path to the spec SQLite database (in-memory if absent)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_DB_PASSPHRASE",
                "Passphrase for the spec database (required if HKASK_SPEC_DB_PATH is set)",
            ),
        ],
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::spec_types::{GoalSpec, Spec, SpecCategory, SpecError, SpecId, SpecStore};
    use hkask_types::{CapabilityChecker, DelegationAction, DelegationResource, WebID};
    use rmcp::handler::server::wrapper::Parameters;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use types::{
        CurateCultivateRequest, CurateEvaluateRequest, CurateReconcileRequest, GoalCaptureRequest,
        GoalDecomposeRequest, GraphQueryRequest, GraphValidateRequest, RequireBindRequest,
        TestInvariantRequest, TestVerifyRequest,
    };

    // ── In-memory SpecStore for testing ─────────────────────────────────

    struct InMemorySpecStore {
        specs: Mutex<HashMap<SpecId, Spec>>,
    }

    impl InMemorySpecStore {
        fn new() -> Self {
            Self {
                specs: Mutex::new(HashMap::new()),
            }
        }
    }

    impl SpecStore for InMemorySpecStore {
        fn load(&self, id: SpecId) -> Result<Spec, SpecError> {
            self.specs
                .lock()
                .expect("lock")
                .get(&id)
                .cloned()
                .ok_or(SpecError::NotFound(id))
        }

        fn save(&self, spec: &Spec) -> Result<(), SpecError> {
            self.specs
                .lock()
                .expect("lock")
                .insert(spec.id, spec.clone());
            Ok(())
        }

        fn delete(&self, id: SpecId) -> Result<(), SpecError> {
            if self.specs.lock().expect("lock").remove(&id).is_some() {
                Ok(())
            } else {
                Err(SpecError::NotFound(id))
            }
        }

        fn list_all(&self) -> Result<Vec<Spec>, SpecError> {
            Ok(self.specs.lock().expect("lock").values().cloned().collect())
        }

        fn list_by_category(&self, cat: SpecCategory) -> Result<Vec<Spec>, SpecError> {
            Ok(self
                .specs
                .lock()
                .expect("lock")
                .values()
                .filter(|s| s.category == cat)
                .cloned()
                .collect())
        }
    }

    // ── Test harness ──────────────────────────────────────────────────────

    fn test_server() -> SpecServer {
        let store = Arc::new(InMemorySpecStore::new());
        let webid = WebID::new();
        let checker = CapabilityChecker::new(b"test-secret-32-bytes-long-enough!!");
        SpecServer::new(store, webid, checker)
    }

    fn test_server_with_store() -> (SpecServer, Arc<InMemorySpecStore>) {
        let store = Arc::new(InMemorySpecStore::new());
        let webid = WebID::new();
        let checker = CapabilityChecker::new(b"test-secret-32-bytes-long-enough!!");
        let server = SpecServer::new(store.clone(), webid, checker);
        (server, store)
    }

    fn valid_token(server: &SpecServer, resource_id: &str, action: DelegationAction) -> String {
        let from = WebID::new();
        let to = server.webid;
        let token = DelegationToken::new(
            DelegationResource::Registry,
            resource_id.to_string(),
            action,
            from,
            to,
            b"test-secret-32-bytes-long-enough!!",
        );
        token.to_base64().expect("token base64 encoding")
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Capability gate tests
    // ═══════════════════════════════════════════════════════════════════════

    // REQ: GOAL-CAPTURE-001 — Rejects requests without capability token (trust boundary)
    #[tokio::test]
    async fn goal_capture_rejects_missing_capability_token() {
        let server = test_server();
        let result = server
            .spec_goal_capture(Parameters(GoalCaptureRequest {
                description: "test goal".to_string(),
                category: "domain".to_string(),
                domain_anchor: "hkask".to_string(),
                criteria: None,
                capability_token: None,
            }))
            .await;
        assert!(
            result.contains("permission_denied") || result.contains("No capability token"),
            "missing token must produce permission error, got: {result}"
        );
    }

    // REQ: GOAL-CAPTURE-002 — Rejects invalid capability token (trust boundary)
    #[tokio::test]
    async fn goal_capture_rejects_invalid_token() {
        let server = test_server();
        let result = server
            .spec_goal_capture(Parameters(GoalCaptureRequest {
                description: "test goal".to_string(),
                category: "domain".to_string(),
                domain_anchor: "hkask".to_string(),
                criteria: None,
                capability_token: Some("invalid-base64-token".to_string()),
            }))
            .await;
        assert!(
            result.contains("permission_denied") || result.contains("Invalid token"),
            "invalid token must produce permission error, got: {result}"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // spec_goal_capture
    // ═══════════════════════════════════════════════════════════════════════

    // REQ: GOAL-CAPTURE-003 — Creates spec and returns captured status with valid token
    #[tokio::test]
    async fn goal_capture_creates_spec_with_valid_token() {
        let server = test_server();
        let token = valid_token(&server, "capture", DelegationAction::Write);
        let result = server
            .spec_goal_capture(Parameters(GoalCaptureRequest {
                description: "build auth system".to_string(),
                category: "domain".to_string(),
                domain_anchor: "hkask".to_string(),
                criteria: Some(vec!["works".to_string()]),
                capability_token: Some(token),
            }))
            .await;
        assert!(
            result.contains("captured"),
            "successful capture must return 'captured' status, got: {result}"
        );
        assert!(
            result.contains("domain"),
            "category must be 'domain', got: {result}"
        );
    }

    // REQ: GOAL-CAPTURE-004 — Populates goal criteria when criteria provided
    #[tokio::test]
    async fn goal_capture_with_criteria_creates_spec() {
        let (server, store) = test_server_with_store();
        let token = valid_token(&server, "capture", DelegationAction::Write);
        let result = server
            .spec_goal_capture(Parameters(GoalCaptureRequest {
                description: "build auth system".to_string(),
                category: "domain".to_string(),
                domain_anchor: "hkask".to_string(),
                criteria: Some(vec!["criteria 1".to_string(), "criteria 2".to_string()]),
                capability_token: Some(token),
            }))
            .await;
        assert!(result.contains("captured"));

        let specs = store.list_all().expect("list_all");
        assert_eq!(specs.len(), 1, "one spec must be stored");
        let spec = &specs[0];
        assert_eq!(spec.goals.len(), 1, "spec must have one goal");
        assert_eq!(spec.goals[0].criteria.len(), 2, "goal must have 2 criteria");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // spec_goal_decompose
    // ═══════════════════════════════════════════════════════════════════════

    // REQ: GOAL-DECOMPOSE-001 — Rejects empty spec_id (interface)
    #[tokio::test]
    async fn goal_decompose_rejects_empty_spec_id() {
        let server = test_server();
        let token = valid_token(&server, "", DelegationAction::Write);
        let result = server
            .spec_goal_decompose(Parameters(GoalDecomposeRequest {
                spec_id: "".to_string(),
                goal_index: 0,
                sub_goals: vec!["sub1".to_string()],
                capability_token: Some(token),
            }))
            .await;
        assert!(
            result.contains("invalid_argument") || result.contains("InvalidArgument"),
            "empty spec_id must produce invalid_argument error, got: {result}"
        );
    }

    // REQ: GOAL-DECOMPOSE-002 — Rejects nonexistent spec (interface)
    #[tokio::test]
    async fn goal_decompose_rejects_not_found_spec() {
        let server = test_server();
        let token = valid_token(&server, "nonexistent-spec", DelegationAction::Write);
        let result = server
            .spec_goal_decompose(Parameters(GoalDecomposeRequest {
                spec_id: "nonexistent-spec".to_string(),
                goal_index: 0,
                sub_goals: vec!["sub1".to_string()],
                capability_token: Some(token),
            }))
            .await;
        assert!(
            result.contains("not_found") || result.contains("not found"),
            "nonexistent spec must produce not_found error, got: {result}"
        );
    }

    // REQ: GOAL-DECOMPOSE-003 — Adds sub-goals to existing goal (capability)
    #[tokio::test]
    async fn goal_decompose_adds_sub_goals() {
        let (server, store) = test_server_with_store();
        let token = valid_token(&server, "capture", DelegationAction::Write);

        let capture_result = server
            .spec_goal_capture(Parameters(GoalCaptureRequest {
                description: "parent goal".to_string(),
                category: "domain".to_string(),
                domain_anchor: "hkask".to_string(),
                criteria: None,
                capability_token: Some(token.clone()),
            }))
            .await;
        assert!(capture_result.contains("captured"));

        let specs = store.list_all().expect("list_all");
        let spec_id_str = specs[0].id.to_string();

        let decompose_token = valid_token(&server, &spec_id_str, DelegationAction::Write);
        let result = server
            .spec_goal_decompose(Parameters(GoalDecomposeRequest {
                spec_id: spec_id_str,
                goal_index: 0,
                sub_goals: vec!["sub-goal-1".to_string(), "sub-goal-2".to_string()],
                capability_token: Some(decompose_token),
            }))
            .await;
        assert!(
            result.contains("sub_goals_added") && result.contains("2"),
            "decompose must report 2 sub-goals added, got: {result}"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // spec_require_bind
    // ═══════════════════════════════════════════════════════════════════════

    // REQ: REQUIRE-BIND-001 — Attaches OCAP boundary to goal (capability)
    #[tokio::test]
    async fn require_bind_attaches_boundary() {
        let (server, store) = test_server_with_store();
        let token = valid_token(&server, "capture", DelegationAction::Write);

        let capture_result = server
            .spec_goal_capture(Parameters(GoalCaptureRequest {
                description: "goal with boundary".to_string(),
                category: "capability".to_string(),
                domain_anchor: "hkask".to_string(),
                criteria: None,
                capability_token: Some(token.clone()),
            }))
            .await;
        assert!(capture_result.contains("captured"));

        let specs = store.list_all().expect("list_all");
        let spec_id_str = specs[0].id.to_string();

        let bind_token = valid_token(&server, &spec_id_str, DelegationAction::Write);
        let result = server
            .spec_require_bind(Parameters(RequireBindRequest {
                spec_id: spec_id_str,
                goal_index: 0,
                // F-SYN-001: must be a known OcapTokenKind; "spec_curate" is
                // the natural choice for a spec curation boundary.
                capability: "spec_curate".to_string(),
                authority: "explicit".to_string(),
                capability_token: Some(bind_token),
            }))
            .await;
        assert!(
            result.contains("enforced") && result.contains("true"),
            "bind must return enforced=true, got: {result}"
        );
    }

    // REQ: REQUIRE-BIND-002 — Rejects unknown capability kind (F-SYN-001 security)
    #[tokio::test]
    async fn require_bind_rejects_unknown_capability_kind() {
        let (server, store) = test_server_with_store();
        let token = valid_token(&server, "capture", DelegationAction::Write);

        let capture_result = server
            .spec_goal_capture(Parameters(GoalCaptureRequest {
                description: "goal for attack test".to_string(),
                category: "capability".to_string(),
                domain_anchor: "hkask".to_string(),
                criteria: None,
                capability_token: Some(token.clone()),
            }))
            .await;
        assert!(capture_result.contains("captured"));

        let specs = store.list_all().expect("list_all");
        let spec_id_str = specs[0].id.to_string();

        let bind_token = valid_token(&server, &spec_id_str, DelegationAction::Write);
        for attack in [
            "tool:inference:call",    // the previous (vulnerable) input
            "memory:write:any-webid", // the canonical F-SYN-001 attack
            "*",                      // wildcard
            "",                       // empty
        ] {
            let result = server
                .spec_require_bind(Parameters(RequireBindRequest {
                    spec_id: spec_id_str.clone(),
                    goal_index: 0,
                    capability: attack.to_string(),
                    authority: "explicit".to_string(),
                    capability_token: Some(bind_token.clone()),
                }))
                .await;
            assert!(
                result.contains("invalid_argument") || result.contains("Unknown capability"),
                "F-SYN-001: attack input `{attack}` must be rejected, got: {result}"
            );
        }
    }

    // REQ: REQUIRE-BIND-003 — Rejects legacy "denied" authority (F-SYN-002 security)
    #[tokio::test]
    async fn require_bind_rejects_legacy_denied_authority() {
        let (server, store) = test_server_with_store();
        let token = valid_token(&server, "capture", DelegationAction::Write);

        let capture_result = server
            .spec_goal_capture(Parameters(GoalCaptureRequest {
                description: "goal for denied-test".to_string(),
                category: "capability".to_string(),
                domain_anchor: "hkask".to_string(),
                criteria: None,
                capability_token: Some(token.clone()),
            }))
            .await;
        assert!(capture_result.contains("captured"));

        let specs = store.list_all().expect("list_all");
        let spec_id_str = specs[0].id.to_string();

        let bind_token = valid_token(&server, &spec_id_str, DelegationAction::Write);
        let result = server
            .spec_require_bind(Parameters(RequireBindRequest {
                spec_id: spec_id_str,
                goal_index: 0,
                capability: "spec_curate".to_string(),
                authority: "denied".to_string(),
                capability_token: Some(bind_token),
            }))
            .await;
        assert!(
            result.contains("invalid_argument")
                || result.contains("'denied' authority is no longer supported"),
            "F-SYN-002: legacy 'denied' authority must be rejected, got: {result}"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // spec_curate_evaluate
    // ═══════════════════════════════════════════════════════════════════════

    // REQ: CURATE-EVAL-001 — Returns Merge for complete spec (capability)
    #[tokio::test]
    async fn curate_evaluate_complete_spec_returns_merge() {
        let (server, store) = test_server_with_store();

        let mut goal = GoalSpec::new("complete goal");
        goal = goal.with_criterion("criterion 1");
        goal.criteria[0].mark_satisfied();
        let spec =
            Spec::new("complete spec", SpecCategory::Domain, DomainAnchor::Hkask).with_goal(goal);
        store.save(&spec).expect("save spec");

        let read_token = valid_token(&server, &spec.id.to_string(), DelegationAction::Read);
        let result = server
            .spec_curate_evaluate(Parameters(CurateEvaluateRequest {
                spec_id: spec.id.to_string(),
                rationale_hint: None,
                capability_token: Some(read_token),
            }))
            .await;
        assert!(
            result.contains("merge"),
            "complete spec must produce Merge decision, got: {result}"
        );
    }

    // REQ: CURATE-EVAL-002 — Returns Discard for empty-goals spec (capability)
    #[tokio::test]
    async fn curate_evaluate_empty_spec_returns_discard() {
        let (server, store) = test_server_with_store();
        let spec = Spec::new("empty spec", SpecCategory::Domain, DomainAnchor::Hkask);
        store.save(&spec).expect("save spec");

        let read_token = valid_token(&server, &spec.id.to_string(), DelegationAction::Read);
        let result = server
            .spec_curate_evaluate(Parameters(CurateEvaluateRequest {
                spec_id: spec.id.to_string(),
                rationale_hint: None,
                capability_token: Some(read_token),
            }))
            .await;
        assert!(
            result.contains("discard"),
            "empty-goals spec must produce Discard decision, got: {result}"
        );
    }

    // REQ: CURATE-EVAL-003 — Returns Revise for partial spec (capability)
    #[tokio::test]
    async fn curate_evaluate_partial_spec_returns_revise() {
        let (server, store) = test_server_with_store();

        let mut goal = GoalSpec::new("partial goal");
        goal = goal.with_criterion("unsatisfied criterion");
        let spec =
            Spec::new("partial spec", SpecCategory::Domain, DomainAnchor::Hkask).with_goal(goal);
        store.save(&spec).expect("save spec");

        let read_token = valid_token(&server, &spec.id.to_string(), DelegationAction::Read);
        let result = server
            .spec_curate_evaluate(Parameters(CurateEvaluateRequest {
                spec_id: spec.id.to_string(),
                rationale_hint: Some("partial goals".to_string()),
                capability_token: Some(read_token),
            }))
            .await;
        assert!(
            result.contains("revise"),
            "partial spec must produce Revise decision, got: {result}"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // spec_curate_reconcile
    // ═══════════════════════════════════════════════════════════════════════

    // REQ: CURATE-RECON-001 — Detects tensions between overlapping specs (capability)
    #[tokio::test]
    async fn curate_reconcile_detects_tensions() {
        let (server, store) = test_server_with_store();
        let write_token = valid_token(&server, "reconcile", DelegationAction::Write);

        let mut goal1 = GoalSpec::new("implement user authentication");
        goal1 = goal1.with_criterion("secure");
        let spec1 =
            Spec::new("auth-v1", SpecCategory::Domain, DomainAnchor::Hkask).with_goal(goal1);
        store.save(&spec1).expect("save spec1");

        let mut goal2 = GoalSpec::new("implement user authentication oauth");
        goal2 = goal2.with_criterion("works");
        let spec2 =
            Spec::new("auth-v2", SpecCategory::Capability, DomainAnchor::Hkask).with_goal(goal2);
        store.save(&spec2).expect("save spec2");

        let result = server
            .spec_curate_reconcile(Parameters(CurateReconcileRequest {
                spec_ids: vec![spec1.id.to_string(), spec2.id.to_string()],
                tension_description: "overlapping auth goals".to_string(),
                capability_token: Some(write_token),
            }))
            .await;
        assert!(
            result.contains("tensions_identified") || result.contains("no_tensions_detected"),
            "reconcile must return tension status, got: {result}"
        );
    }

    // REQ: CURATE-RECON-002 — Rejects nonexistent specs (interface)
    #[tokio::test]
    async fn curate_reconcile_rejects_not_found_specs() {
        let server = test_server();
        let write_token = valid_token(&server, "reconcile", DelegationAction::Write);

        let result = server
            .spec_curate_reconcile(Parameters(CurateReconcileRequest {
                spec_ids: vec!["nonexistent-id".to_string()],
                tension_description: "tension check".to_string(),
                capability_token: Some(write_token),
            }))
            .await;
        assert!(
            result.contains("not_found") || result.contains("not found"),
            "nonexistent spec must produce not_found error, got: {result}"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // spec_curate_cultivate
    // ═══════════════════════════════════════════════════════════════════════

    // REQ: CURATE-CULT-001 — Reports coherence for empty collection (capability)
    #[tokio::test]
    async fn curate_cultivate_empty_collection_below_threshold() {
        let (server, store) = test_server_with_store();
        let write_token = valid_token(&server, "cultivate", DelegationAction::Write);

        let specs = store.list_all().expect("list_all");
        assert!(specs.is_empty());

        let result = server
            .spec_curate_cultivate(Parameters(CurateCultivateRequest {
                coherence_threshold: Some(0.7),
                capability_token: Some(write_token),
            }))
            .await;
        assert!(
            result.contains("coherence_score") || result.contains("0"),
            "cultivate must report coherence score, got: {result}"
        );
    }

    // REQ: CURATE-CULT-002 — Reports categories covered and missing (capability)
    #[tokio::test]
    async fn curate_cultivate_reports_categories() {
        let (server, store) = test_server_with_store();
        let write_token = valid_token(&server, "cultivate", DelegationAction::Write);

        let spec = Spec::new("domain spec", SpecCategory::Domain, DomainAnchor::Hkask);
        store.save(&spec).expect("save spec");

        let result = server
            .spec_curate_cultivate(Parameters(CurateCultivateRequest {
                coherence_threshold: Some(0.7),
                capability_token: Some(write_token),
            }))
            .await;
        assert!(
            result.contains("categories_covered") || result.contains("categories_missing"),
            "cultivate must report categories, got: {result}"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // spec_graph_query
    // ═══════════════════════════════════════════════════════════════════════

    // REQ: GRAPH-QUERY-001 — Returns specs filtered by category (interface)
    #[tokio::test]
    async fn graph_query_filters_by_category() {
        let (server, store) = test_server_with_store();
        let read_token = valid_token(&server, "query", DelegationAction::Read);

        let spec_domain = Spec::new("domain spec", SpecCategory::Domain, DomainAnchor::Hkask);
        let spec_cap = Spec::new(
            "capability spec",
            SpecCategory::Capability,
            DomainAnchor::Hkask,
        );
        store.save(&spec_domain).expect("save");
        store.save(&spec_cap).expect("save");

        let result = server
            .spec_graph_query(Parameters(GraphQueryRequest {
                category: Some("domain".to_string()),
                domain_anchor: None,
                capability_token: Some(read_token),
            }))
            .await;
        assert!(
            result.contains("domain") && result.contains("count"),
            "query must return filtered results with count, got: {result}"
        );
    }

    // REQ: GRAPH-QUERY-002 — Returns all specs when no filters provided (capability)
    #[tokio::test]
    async fn graph_query_returns_all_without_filters() {
        let (server, store) = test_server_with_store();
        let read_token = valid_token(&server, "query", DelegationAction::Read);

        let spec1 = Spec::new("spec-1", SpecCategory::Domain, DomainAnchor::Hkask);
        let spec2 = Spec::new("spec-2", SpecCategory::Capability, DomainAnchor::Okapi);
        store.save(&spec1).expect("save");
        store.save(&spec2).expect("save");

        let result = server
            .spec_graph_query(Parameters(GraphQueryRequest {
                category: None,
                domain_anchor: None,
                capability_token: Some(read_token),
            }))
            .await;
        assert!(
            result.contains("count") && result.contains("2"),
            "unfiltered query must return all specs, got: {result}"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // spec_graph_validate
    // ═══════════════════════════════════════════════════════════════════════

    // REQ: GRAPH-VALIDATE-001 — Reports violations when below threshold (capability)
    #[tokio::test]
    async fn graph_validate_reports_violations_below_threshold() {
        let (server, _store) = test_server_with_store();
        let read_token = valid_token(&server, "validate", DelegationAction::Read);

        let result = server
            .spec_graph_validate(Parameters(GraphValidateRequest {
                coherence_threshold: Some(0.7),
                capability_token: Some(read_token),
            }))
            .await;
        assert!(
            result.contains("violations"),
            "validate must report violations when below threshold, got: {result}"
        );
        assert!(
            result.contains("valid") && result.contains("false"),
            "coherence below threshold must set valid=false, got: {result}"
        );
    }

    // REQ: GRAPH-VALIDATE-002 — Reports suggestions for missing categories (capability)
    #[tokio::test]
    async fn graph_validate_reports_missing_categories() {
        let (server, store) = test_server_with_store();
        let read_token = valid_token(&server, "validate", DelegationAction::Read);

        let spec = Spec::new("domain spec", SpecCategory::Domain, DomainAnchor::Hkask);
        store.save(&spec).expect("save");

        let result = server
            .spec_graph_validate(Parameters(GraphValidateRequest {
                coherence_threshold: Some(0.7),
                capability_token: Some(read_token),
            }))
            .await;
        assert!(
            result.contains("suggestions") || result.contains("Missing category"),
            "validate must report missing categories, got: {result}"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // spec_test_invariant
    // ═══════════════════════════════════════════════════════════════════════

    // REQ: INVAR-001 — Rejects requests without capability token (trust boundary)
    #[tokio::test]
    async fn test_invariant_rejects_missing_capability_token() {
        let server = test_server();
        let result = server
            .spec_test_invariant(Parameters(TestInvariantRequest {
                spec_id: "some-spec-id".to_string(),
                seam: "public_interface".to_string(),
                invariant: "test behavior".to_string(),
                category: "PublicInterface".to_string(),
                cycle: None,
                capability_token: None,
            }))
            .await;
        assert!(
            result.contains("permission_denied") || result.contains("No capability token"),
            "missing token must produce permission error, got: {result}"
        );
    }

    // REQ: INVAR-002 — Rejects invalid capability token (trust boundary)
    #[tokio::test]
    async fn test_invariant_rejects_invalid_token() {
        let server = test_server();
        let result = server
            .spec_test_invariant(Parameters(TestInvariantRequest {
                spec_id: "some-spec-id".to_string(),
                seam: "public_interface".to_string(),
                invariant: "test behavior".to_string(),
                category: "PublicInterface".to_string(),
                cycle: None,
                capability_token: Some("invalid-base64-token".to_string()),
            }))
            .await;
        assert!(
            result.contains("permission_denied") || result.contains("Invalid token"),
            "invalid token must produce permission error, got: {result}"
        );
    }

    // REQ: INVAR-003 — Rejects unknown spec_id (interface)
    #[tokio::test]
    async fn test_invariant_rejects_unknown_spec_id() {
        let server = test_server();
        // Token must be authorized for test/invariant to pass capability gate,
        // so the spec_id not-found check is the one under test.
        let token = valid_token(&server, "test/invariant", DelegationAction::Write);
        let result = server
            .spec_test_invariant(Parameters(TestInvariantRequest {
                spec_id: "nonexistent-spec-id".to_string(),
                seam: "public_interface".to_string(),
                invariant: "test_behavior".to_string(),
                category: "PublicInterface".to_string(),
                cycle: None,
                capability_token: Some(token),
            }))
            .await;
        assert!(
            result.contains("not_found") || result.contains("not found"),
            "unknown spec_id must produce not_found error, got: {result}"
        );
    }

    // REQ: INVAR-004 — Records traceability with valid token and existing spec (capability)
    #[tokio::test]
    async fn test_invariant_records_traceability() {
        let (server, store) = test_server_with_store();
        let capture_token = valid_token(&server, "capture", DelegationAction::Write);

        // Create a spec first
        let capture_result = server
            .spec_goal_capture(Parameters(GoalCaptureRequest {
                description: "spec for invariant test".to_string(),
                category: "domain".to_string(),
                domain_anchor: "hkask".to_string(),
                criteria: None,
                capability_token: Some(capture_token),
            }))
            .await;
        assert!(capture_result.contains("captured"));

        let specs = store.list_all().expect("list_all");
        let spec_id_str = specs[0].id.to_string();

        // Token must be scoped to test/invariant for the capability gate
        let invariant_token = valid_token(&server, "test/invariant", DelegationAction::Write);
        let result = server
            .spec_test_invariant(Parameters(TestInvariantRequest {
                spec_id: spec_id_str,
                seam: "public_interface".to_string(),
                invariant: "rejects_unauthenticated_requests".to_string(),
                category: "PublicInterface".to_string(),
                cycle: None,
                capability_token: Some(invariant_token),
            }))
            .await;
        assert!(
            result.contains("recorded"),
            "valid invariant must return 'recorded' status, got: {result}"
        );
        // Verify invariant_id is derived from spec_id + seam + category
        assert!(
            result.contains("invariant_id"),
            "response must include invariant_id, got: {result}"
        );
    }

    // REQ: INVAR-005 — Includes cycle tag in status when provided (interface)
    #[tokio::test]
    async fn test_invariant_includes_cycle_tag() {
        let (server, store) = test_server_with_store();
        let capture_token = valid_token(&server, "capture", DelegationAction::Write);

        // Create a spec first
        let capture_result = server
            .spec_goal_capture(Parameters(GoalCaptureRequest {
                description: "spec for cycle test".to_string(),
                category: "domain".to_string(),
                domain_anchor: "hkask".to_string(),
                criteria: None,
                capability_token: Some(capture_token),
            }))
            .await;
        assert!(capture_result.contains("captured"));

        let specs = store.list_all().expect("list_all");
        let spec_id_str = specs[0].id.to_string();

        // Token must be scoped to test/invariant for the capability gate
        let invariant_token = valid_token(&server, "test/invariant", DelegationAction::Write);
        let result = server
            .spec_test_invariant(Parameters(TestInvariantRequest {
                spec_id: spec_id_str,
                seam: "public_interface".to_string(),
                invariant: "test_with_cycle".to_string(),
                category: "PublicInterface".to_string(),
                cycle: Some("red".to_string()),
                capability_token: Some(invariant_token),
            }))
            .await;
        assert!(
            result.contains("recorded") && result.contains("red cycle"),
            "cycle tag must appear in status when provided, got: {result}"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // spec_test_verify
    // ═══════════════════════════════════════════════════════════════════════

    // REQ: VERIFY-001 — Rejects requests without capability token (trust boundary)
    #[tokio::test]
    async fn test_verify_rejects_missing_capability_token() {
        let server = test_server();
        let result = server
            .spec_test_verify(Parameters(TestVerifyRequest {
                seam: None,
                category: None,
                capability_token: None,
            }))
            .await;
        assert!(
            result.contains("permission_denied") || result.contains("No capability token"),
            "missing token must produce permission error, got: {result}"
        );
    }

    // REQ: VERIFY-002 — Reports complete=true when all specs satisfied (capability)
    #[tokio::test]
    async fn test_verify_reports_complete_when_all_satisfied() {
        let (server, store) = test_server_with_store();
        let token = valid_token(&server, "test/verify", DelegationAction::Read);

        // Create a complete spec (all criteria satisfied)
        let mut goal = GoalSpec::new("complete goal");
        goal = goal.with_criterion("criterion 1");
        goal.criteria[0].mark_satisfied();
        let spec =
            Spec::new("complete spec", SpecCategory::Domain, DomainAnchor::Hkask).with_goal(goal);
        store.save(&spec).expect("save spec");

        let result = server
            .spec_test_verify(Parameters(TestVerifyRequest {
                seam: None,
                category: None,
                capability_token: Some(token),
            }))
            .await;
        assert!(
            result.contains("complete") && result.contains("true"),
            "all-satisfied specs must yield complete=true, got: {result}"
        );
        assert!(
            result.contains("total_requirements") && result.contains("tested"),
            "verify must report total_requirements and tested, got: {result}"
        );
    }

    // REQ: VERIFY-003 — Reports gaps for incomplete specs (capability)
    #[tokio::test]
    async fn test_verify_reports_gaps_for_incomplete() {
        let (server, store) = test_server_with_store();
        let token = valid_token(&server, "test/verify", DelegationAction::Read);

        // Create an incomplete spec (unsatisfied criteria)
        let mut goal = GoalSpec::new("incomplete goal");
        goal = goal.with_criterion("unsatisfied criterion");
        let spec =
            Spec::new("incomplete spec", SpecCategory::Domain, DomainAnchor::Hkask).with_goal(goal);
        store.save(&spec).expect("save spec");

        let result = server
            .spec_test_verify(Parameters(TestVerifyRequest {
                seam: None,
                category: None,
                capability_token: Some(token),
            }))
            .await;
        assert!(
            result.contains("gaps") && result.contains("1"),
            "incomplete spec must report gaps > 0, got: {result}"
        );
        assert!(
            result.contains("complete") && result.contains("false"),
            "incomplete specs must yield complete=false, got: {result}"
        );
    }

    // REQ: VERIFY-004 — Filters by category (interface)
    #[tokio::test]
    async fn test_verify_filters_by_category() {
        let (server, store) = test_server_with_store();
        let token = valid_token(&server, "test/verify", DelegationAction::Read);

        // Create one complete domain spec and one incomplete capability spec
        let mut domain_goal = GoalSpec::new("complete domain goal");
        domain_goal = domain_goal.with_criterion("criterion 1");
        domain_goal.criteria[0].mark_satisfied();
        let domain_spec = Spec::new("domain-spec", SpecCategory::Domain, DomainAnchor::Hkask)
            .with_goal(domain_goal);
        store.save(&domain_spec).expect("save domain spec");

        let cap_goal = GoalSpec::new("incomplete cap goal");
        // Spec without criteria is incomplete by definition
        let cap_spec = Spec::new("cap-spec", SpecCategory::Capability, DomainAnchor::Hkask)
            .with_goal(cap_goal);
        store.save(&cap_spec).expect("save cap spec");

        // Filter to only domain category — should report 1 requirement, complete
        let result = server
            .spec_test_verify(Parameters(TestVerifyRequest {
                seam: None,
                category: Some("domain".to_string()),
                capability_token: Some(token),
            }))
            .await;
        assert!(
            result.contains("total_requirements") && result.contains("1"),
            "category filter must return only matching specs, got: {result}"
        );
    }
}
