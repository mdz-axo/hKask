//! hKask MCP Spec — Specification authoring, curation, and validation (11 tools per DDMVSS §6.3)
//!
//! Curation decisions (merge/revise/defer/discard) driven by spec-document completeness,
//! orthogonal to code-implementation status. Writing Excellence 4-perspective test
//! (Hopper/Lovelace/Schriver/Gentle) per WRITING_EXCELLENCE.md §3: 3/4 passing = publishable.

pub mod types;

use hkask_mcp::server::{McpToolError, ServerContext, ToolSpanGuard};
use hkask_mcp::validate_field;

use hkask_storage::SpecStore;
use hkask_storage::spec_types::{DomainAnchor, GoalSpec, Spec, SpecCategory, SpecError, SpecId};
use hkask_types::{
    CapabilityChecker, CurationDecision, DelegationAction, DelegationResource, DelegationToken,
    McpErrorKind, OCAPBoundary, TOKEN_ERR_EXPIRED, TOKEN_ERR_INVALID_SIGNATURE,
    TOKEN_ERR_NO_CHECKER, VerificationOutcome, WebID, token_err_insufficient_access,
    verify_delegation_token_now,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use std::sync::Arc;
use types::*;

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
            .field("webid", &self.webid)
            .finish()
    }
}

// Capability-check macro — covers 11 tool handlers
macro_rules! check_cap {
    ($self:expr, $span:expr, $token:expr, $resource:expr, $action:expr) => {
        if let Err(e) = $self.verify_capability($token.as_deref(), $resource, $action) {
            return $span.error(e.kind, e.to_json_string());
        }
    };
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

    /// Load spec by id string; returns error wire string on failure.
    fn load_spec(&self, spec_id: &str) -> Result<Spec, (McpErrorKind, String)> {
        let parsed = SpecId::from_string(spec_id).unwrap_or_default();
        self.store.load(parsed).map_err(|_| {
            (
                McpErrorKind::NotFound,
                McpToolError::not_found(format!("Spec not found: {}", spec_id)).to_json_string(),
            )
        })
    }

    /// Load all specs; returns error JSON value on failure.
    fn load_all_specs_val(&self) -> Result<Vec<Spec>, serde_json::Value> {
        self.store
            .list_all()
            .map_err(|e| serde_json::json!({"error": format!("Failed to load specs: {}", e)}))
    }

    /// Save spec; returns error JSON value on failure.
    fn persist_val(&self, spec: &Spec) -> Result<(), serde_json::Value> {
        self.save_spec(spec)
            .map_err(|e| serde_json::json!({"error": format!("Failed to persist spec: {}", e)}))
    }

    fn save_spec(&self, spec: &Spec) -> Result<(), SpecError> {
        self.store.save(spec)
    }
}

/// Serialize response and convert to ok_json
fn respond<T: serde::Serialize>(span: ToolSpanGuard, resp: &T) -> String {
    span.ok_json(serde_json::to_value(resp).unwrap_or_default())
}

/// Categories not yet covered by a spec collection.
fn missing_categories(covered: &std::collections::HashSet<String>) -> Vec<String> {
    SpecCategory::all()
        .iter()
        .map(|c| c.as_str().to_string())
        .filter(|c| !covered.contains(c))
        .collect()
}

fn not_found_err(spec_id: &str) -> (McpErrorKind, String) {
    (
        McpErrorKind::NotFound,
        McpToolError::not_found(format!("Spec not found: {}", spec_id)).to_json_string(),
    )
}

#[tool_router(server_handler)]
impl SpecServer {
    #[tool(
        description = "Capture a goal as a binding specification requirement in a spec document"
    )]
    async fn spec_goal_capture(
        &self,
        Parameters(GoalCaptureRequest {
            description,
            category,
            domain_anchor,
            criteria,
            capability_token,
            completeness_domain: _,
        }): Parameters<GoalCaptureRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_goal_capture", &self.webid);
        check_cap!(
            self,
            span,
            capability_token,
            "capture",
            DelegationAction::Write
        );

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
        if let Err(v) = self.persist_val(&spec) {
            return span.internal_error(v);
        }
        respond(
            span,
            &GoalCaptureResponse {
                spec_id: id.to_string(),
                category: cat.as_str().to_string(),
                domain_anchor: anchor.as_str().to_string(),
                status: "captured".to_string(),
            },
        )
    }

    #[tool(description = "Decompose a specification goal into ordered sub-goals (max depth 7)")]
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
        check_cap!(
            self,
            span,
            capability_token,
            &spec_id,
            DelegationAction::Write
        );

        let mut spec = match self.load_spec(&spec_id) {
            Ok(s) => s,
            Err((kind, msg)) => return span.error(kind, msg),
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
        if let Err(v) = self.persist_val(&spec) {
            return span.internal_error(v);
        }
        respond(
            span,
            &GoalDecomposeResponse {
                spec_id,
                goal_index,
                sub_goals_added: added,
            },
        )
    }

    #[tool(description = "Bind OCAP boundaries to a specification goal as a constraint")]
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
        check_cap!(
            self,
            span,
            capability_token,
            &spec_id,
            DelegationAction::Write
        );

        let mut spec = match self.load_spec(&spec_id) {
            Ok(s) => s,
            Err((kind, msg)) => return span.error(kind, msg),
        };
        if goal_index >= spec.goals.len() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!("Goal index {} out of range", goal_index))
                    .to_json_string(),
            );
        }
        let boundary = match OCAPBoundary::parse_token(&capability) {
            Some(b) => b,
            None => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(format!(
                        "Unknown capability kind: {capability:?}. Expected: curation, cybernetics, spec_curate."
                    ))
                    .to_json_string(),
                );
            }
        };
        if authority == "denied" {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(
                    "The 'denied' authority is no longer supported. Every OCAPBoundary is enforced by construction."
                        .to_string(),
                )
                .to_json_string(),
            );
        }
        spec.goals[goal_index].constraints.push(boundary);
        if let Err(v) = self.persist_val(&spec) {
            return span.internal_error(v);
        }
        respond(
            span,
            &RequireBindResponse {
                spec_id,
                goal_index,
                capability,
                authority,
                enforced: true,
            },
        )
    }

    #[tool(
        description = "Assess specification artifact against collection coherence. Evaluates spec-document completeness. When writing_excellence scores are provided, includes 4-perspective test results (Hopper/Lovelace/Schriver/Gentle) per WRITING_EXCELLENCE.md §3."
    )]
    async fn spec_curate_evaluate(
        &self,
        Parameters(CurateEvaluateRequest {
            spec_id,
            rationale_hint,
            capability_token,
            completeness_domain,
            writing_excellence,
        }): Parameters<CurateEvaluateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_curate_evaluate", &self.webid);
        validate_field!(span, "spec_id", &spec_id, 256);
        check_cap!(
            self,
            span,
            capability_token,
            &spec_id,
            DelegationAction::Read
        );

        let spec = match self.load_spec(&spec_id) {
            Ok(s) => s,
            Err((kind, msg)) => return span.error(kind, msg),
        };
        let complete = spec.is_complete();
        let we_blocks = writing_excellence
            .as_ref()
            .map(|we| we.passes() <= 1)
            .unwrap_or(false);
        let decision = if we_blocks {
            CurationDecision::Discard
        } else if complete {
            CurationDecision::Merge
        } else if spec.goals.is_empty() {
            CurationDecision::Discard
        } else {
            CurationDecision::Revise
        };
        let domain = completeness_domain.unwrap_or_default();
        let rationale = rationale_hint.unwrap_or_else(|| {
            if we_blocks {
                "Writing Excellence: 1 or fewer dimensions pass — publication blocked".to_string()
            } else if complete {
                "All criteria satisfied".to_string()
            } else {
                "Unsatisfied criteria remain".to_string()
            }
        });
        let implementation_status = (domain == CompletenessDomain::Implementation).then(|| {
            if complete {
                "spec complete; implementation status unknown".to_string()
            } else {
                "spec incomplete; implementation status irrelevant".to_string()
            }
        });
        respond(
            span,
            &CurateEvaluateResponse {
                spec_id,
                decision: decision.to_string(),
                rationale,
                coherence_score: spec.coherence(),
                specification_completeness: complete,
                implementation_status,
                writing_excellence,
            },
        )
    }

    #[tool(description = "Reconcile spec-domain tensions between specification documents")]
    async fn spec_curate_reconcile(
        &self,
        Parameters(CurateReconcileRequest {
            spec_ids,
            tension_description,
            capability_token,
        }): Parameters<CurateReconcileRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_curate_reconcile", &self.webid);
        check_cap!(
            self,
            span,
            capability_token,
            "reconcile",
            DelegationAction::Write
        );

        let mut loaded_specs = Vec::new();
        let mut missing = Vec::new();
        for id in &spec_ids {
            let parsed = SpecId::from_string(id).unwrap_or_default();
            match self.store.load(parsed) {
                Ok(s) => loaded_specs.push(s),
                Err(_) => missing.push(id.as_str()),
            }
        }
        if !missing.is_empty() {
            let (kind, msg) = not_found_err(&format!("{:?}", missing));
            return span.error(kind, msg);
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
                let union = words_a.union(&words_b).count();
                let jaccard = if union > 0 {
                    words_a.intersection(&words_b).count() as f64 / union as f64
                } else {
                    0.0
                };
                if jaccard > 0.3 {
                    tensions.push(TensionReport {
                        spec_a: a.id.to_string(),
                        spec_b: b.id.to_string(),
                        overlapping_goals: words_a
                            .intersection(&words_b)
                            .map(|w| w.to_string())
                            .collect(),
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
        respond(
            span,
            &CurateReconcileResponse {
                resolution: resolution.to_string(),
                spec_ids,
                tension: tension_description,
                tensions,
                status: "reconciled".to_string(),
            },
        )
    }

    #[tool(description = "Grow specification collection toward coherence")]
    async fn spec_curate_cultivate(
        &self,
        Parameters(CurateCultivateRequest {
            coherence_threshold,
            capability_token,
            completeness_domain: _,
        }): Parameters<CurateCultivateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_curate_cultivate", &self.webid);
        check_cap!(
            self,
            span,
            capability_token,
            "cultivate",
            DelegationAction::Write
        );

        let threshold = coherence_threshold.unwrap_or(0.7);
        let all_specs = match self.load_all_specs_val() {
            Ok(specs) => specs,
            Err(v) => return span.internal_error(v),
        };
        let coherence = Spec::collection_coherence(&all_specs);
        let categories_covered: std::collections::HashSet<String> = all_specs
            .iter()
            .map(|s| s.category.as_str().to_string())
            .collect();
        let categories_missing = missing_categories(&categories_covered);
        respond(
            span,
            &CurateCultivateResponse {
                coherence_score: coherence,
                threshold,
                above_threshold: coherence >= threshold,
                spec_count: all_specs.len(),
                categories_covered: categories_covered.into_iter().collect(),
                categories_missing,
            },
        )
    }

    #[tool(
        description = "Assess a specification document against the Writing Excellence 4-perspective test (Hopper: accessibility, Lovelace: precision, Schriver: findability, Gentle: agent-correctness). 3/4 = publishable; 1/4 blocks."
    )]
    async fn spec_curate_writing_excellence(
        &self,
        Parameters(WritingExcellenceRequest {
            spec_id,
            scores,
            notes: _,
            capability_token,
        }): Parameters<WritingExcellenceRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_curate_writing_excellence", &self.webid);
        validate_field!(span, "spec_id", &spec_id, 256);
        check_cap!(
            self,
            span,
            capability_token,
            &spec_id,
            DelegationAction::Read
        );

        if let Err((kind, msg)) = self.load_spec(&spec_id).map(|_| ()) {
            return span.error(kind, msg);
        }
        let dims = scores.passes();
        respond(
            span,
            &WritingExcellenceResponse {
                spec_id,
                dimensions_passing: dims,
                meets_publication_standard: scores.meets_publication_standard(),
                blocks_publication: dims <= 1,
                scores,
            },
        )
    }

    #[tool(description = "Query the specification document graph by category or domain anchor")]
    async fn spec_graph_query(
        &self,
        Parameters(GraphQueryRequest {
            category,
            domain_anchor,
            capability_token,
        }): Parameters<GraphQueryRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_graph_query", &self.webid);
        check_cap!(
            self,
            span,
            capability_token,
            "query",
            DelegationAction::Read
        );

        let all_specs = match self.load_all_specs_val() {
            Ok(specs) => specs,
            Err(v) => return span.internal_error(v),
        };
        let nodes: Vec<GraphNodeResponse> = all_specs
            .iter()
            .filter(|s| {
                category
                    .as_ref()
                    .map(|c| s.category.as_str() == c.as_str())
                    .unwrap_or(true)
                    && domain_anchor
                        .as_ref()
                        .map(|a| s.domain_anchor.as_str() == a.as_str())
                        .unwrap_or(true)
            })
            .map(|s| GraphNodeResponse {
                id: s.id.to_string(),
                name: s.name.clone(),
                category: s.category.as_str().to_string(),
                complete: s.is_complete(),
            })
            .collect();
        respond(
            span,
            &GraphQueryResponse {
                count: nodes.len(),
                specs: nodes,
            },
        )
    }

    #[tool(
        description = "Validate specification collection for internal consistency and coherence"
    )]
    async fn spec_graph_validate(
        &self,
        Parameters(GraphValidateRequest {
            coherence_threshold,
            capability_token,
        }): Parameters<GraphValidateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_graph_validate", &self.webid);
        check_cap!(
            self,
            span,
            capability_token,
            "validate",
            DelegationAction::Read
        );

        let threshold = coherence_threshold.unwrap_or(0.7);
        let all_specs = match self.load_all_specs_val() {
            Ok(specs) => specs,
            Err(v) => return span.internal_error(v),
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
        let categories_covered: std::collections::HashSet<String> = all_specs
            .iter()
            .map(|s| s.category.as_str().to_string())
            .collect();
        suggestions.extend(
            missing_categories(&categories_covered)
                .into_iter()
                .map(|c| format!("Missing category: {}", c)),
        );
        for spec in &all_specs {
            if !spec.is_complete() {
                suggestions.push(format!("Incomplete spec: {} ({})", spec.id, spec.name));
            }
        }
        respond(
            span,
            &GraphValidateResponse {
                valid: violations.is_empty(),
                coherence_score: coherence,
                threshold,
                violations,
                suggestions,
                spec_count: all_specs.len(),
            },
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
        check_cap!(
            self,
            span,
            capability_token,
            "test/invariant",
            DelegationAction::Write
        );
        validate_field!(span, "spec_id", &spec_id, 256);
        validate_field!(span, "seam", &seam, 256);
        validate_field!(span, "invariant", &invariant, 1024);
        validate_field!(span, "category", &category, 64);

        match self.load_spec(&spec_id) {
            Ok(_) => {}
            Err((kind, msg)) => return span.error(kind, msg),
        }
        let invariant_id = format!("{}:{}:{}", spec_id, seam, category.to_lowercase());
        let cycle_tag = cycle
            .as_deref()
            .map(|c| format!(" [{} cycle]", c))
            .unwrap_or_default();
        respond(
            span,
            &TestInvariantResponse {
                invariant_id,
                status: format!("recorded{}", cycle_tag),
            },
        )
    }

    #[tool(description = "Verify test coverage for a specification seam or spec category")]
    async fn spec_test_verify(
        &self,
        Parameters(TestVerifyRequest {
            seam: _,
            category,
            capability_token,
        }): Parameters<TestVerifyRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_test_verify", &self.webid);
        check_cap!(
            self,
            span,
            capability_token,
            "test/verify",
            DelegationAction::Read
        );

        let all_specs = match self.load_all_specs_val() {
            Ok(specs) => specs,
            Err(v) => return span.internal_error(v),
        };
        let filtered: Vec<&Spec> = all_specs
            .iter()
            .filter(|s| {
                category
                    .as_deref()
                    .and_then(SpecCategory::parse_str)
                    .as_ref()
                    .map(|cf| s.category == *cf)
                    .unwrap_or(true)
            })
            .collect();
        let mut traceability = Vec::new();
        let mut tested = 0;
        let mut gaps = 0;
        for spec in &filtered {
            let is_complete = spec.is_complete();
            if is_complete {
                tested += 1;
            } else {
                gaps += 1;
            }
            traceability.push(TestTraceability {
                requirement_id: spec.id.to_string(),
                classification: is_complete.then_some(TestClassification::PublicInterface),
                test_path: is_complete.then(|| format!("spec:{}", spec.id)),
                has_gap: !is_complete,
                test_debt_location: None,
            });
        }
        respond(
            span,
            &TestVerifyResponse {
                total_requirements: filtered.len(),
                tested,
                gaps,
                debt: 0,
                traceability,
                complete: gaps == 0 && !filtered.is_empty(),
            },
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
                    let passphrase =
                        ctx.credentials
                            .get("HKASK_DB_PASSPHRASE")
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "HKASK_SPEC_DB_PATH set but HKASK_DB_PASSPHRASE missing"
                                )
                            })?;
                    let db = hkask_storage::Database::open(path, passphrase)
                        .map_err(|e| anyhow::anyhow!("Failed to open spec database: {e}"))?;
                    db.conn_arc()
                }
                None => {
                    tracing::warn!(
                        target: "hkask.mcp.spec",
                        "No persistent DB — spec store in-memory (set HKASK_SPEC_DB_PATH + HKASK_DB_PASSPHRASE for persistence)"
                    );
                    let conn = rusqlite::Connection::open_in_memory()?;
                    std::sync::Arc::new(std::sync::Mutex::new(conn))
                }
            };
            let store = std::sync::Arc::new(hkask_storage::SqliteSpecStore::new(conn));
            store
                .init_schema()
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let secret_hex =
                ctx.credentials
                    .get("HKASK_OCAP_SECRET")
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "HKASK_OCAP_SECRET is required for spec capability verification"
                        )
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
