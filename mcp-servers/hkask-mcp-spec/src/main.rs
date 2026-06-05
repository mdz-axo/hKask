//! hKask MCP Spec — Specification authoring, curation, and validation
//!
//! 8 tools following DDMVSS §6.3:
//! - spec/goal/capture — Elicit user intent as binding requirement
//! - spec/goal/decompose — Break goal into ordered sub-goals
//! - spec/require/bind — Attach OCAP boundaries to a goal
//! - spec/curate/evaluate — Assess artifact against collection coherence
//! - spec/curate/reconcile — Resolve goal tensions
//! - spec/curate/cultivate — Grow collection toward coherence
//! - spec/graph/query — Query specification graph
//! - spec/graph/validate — Validate collection coherence

use hkask_mcp::server::{
    McpToolError, ServerContext, ToolSpanGuard, run_stdio_server, validate_identifier,
};
use hkask_storage::spec_types::{
    DomainAnchor, GoalSpec, Spec, SpecCategory, SpecError, SpecId, SpecStore,
};
use hkask_types::{
    CapabilityChecker, CurationDecision, DelegationAction, DelegationResource, DelegationToken,
    McpErrorKind, OCAPBoundary, WebID,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── Response types ───────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct GoalCaptureResponse {
    pub spec_id: String,
    pub category: String,
    pub domain_anchor: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct GoalDecomposeResponse {
    pub spec_id: String,
    pub goal_index: usize,
    pub sub_goals_added: usize,
}

#[derive(Debug, Serialize)]
pub struct RequireBindResponse {
    pub spec_id: String,
    pub goal_index: usize,
    pub capability: String,
    pub authority: String,
    pub enforced: bool,
}

#[derive(Debug, Serialize)]
pub struct CurateEvaluateResponse {
    pub spec_id: String,
    pub decision: String,
    pub rationale: String,
    pub coherence_score: f64,
}

#[derive(Debug, Serialize)]
pub struct CurateReconcileResponse {
    pub resolution: String,
    pub spec_ids: Vec<String>,
    pub tension: String,
    pub tensions: Vec<TensionReport>,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct TensionReport {
    pub spec_a: String,
    pub spec_b: String,
    pub overlapping_goals: Vec<String>,
    pub jaccard_score: f64,
}

#[derive(Debug, Serialize)]
pub struct CurateCultivateResponse {
    pub coherence_score: f64,
    pub threshold: f64,
    pub above_threshold: bool,
    pub spec_count: usize,
    pub categories_covered: Vec<String>,
    pub categories_missing: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct GraphNodeResponse {
    pub id: String,
    pub name: String,
    pub category: String,
    pub complete: bool,
}

#[derive(Debug, Serialize)]
pub struct GraphQueryResponse {
    pub count: usize,
    pub specs: Vec<GraphNodeResponse>,
}

#[derive(Debug, Serialize)]
pub struct GraphValidateResponse {
    pub valid: bool,
    pub coherence_score: f64,
    pub threshold: f64,
    pub violations: Vec<String>,
    pub suggestions: Vec<String>,
    pub spec_count: usize,
}

// ── Request types ────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GoalCaptureRequest {
    pub description: String,
    pub category: String,
    pub domain_anchor: String,
    pub criteria: Option<Vec<String>>,
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GoalDecomposeRequest {
    pub spec_id: String,
    pub goal_index: usize,
    pub sub_goals: Vec<String>,
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RequireBindRequest {
    pub spec_id: String,
    pub goal_index: usize,
    pub capability: String,
    pub authority: String,
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CurateEvaluateRequest {
    pub spec_id: String,
    pub rationale_hint: Option<String>,
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CurateReconcileRequest {
    pub spec_ids: Vec<String>,
    pub tension_description: String,
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CurateCultivateRequest {
    pub coherence_threshold: Option<f64>,
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphQueryRequest {
    pub category: Option<String>,
    pub domain_anchor: Option<String>,
    pub capability_token: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphValidateRequest {
    pub coherence_threshold: Option<f64>,
    pub capability_token: Option<String>,
}

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

        if !self.capability_checker.verify(&token) {
            return Err(McpToolError::permission_denied(
                "Token signature verification failed".to_string(),
            ));
        }
        let now = chrono::Utc::now().timestamp();
        if token.is_expired(now) {
            return Err(McpToolError::permission_denied("Token expired".to_string()));
        }
        if !token.is_valid_for(DelegationResource::Registry, resource_id, action) {
            return Err(McpToolError::permission_denied(format!(
                "Token does not grant spec:{}:{}",
                resource_id,
                action.as_str()
            )));
        }
        Ok(())
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
        let span = ToolSpanGuard::new("spec:goal_capture", &self.webid);

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
        let span = ToolSpanGuard::new("spec:goal_decompose", &self.webid);

        if let Err(e) = validate_identifier("spec_id", &spec_id, 256) {
            return span.error(e.kind, e.to_json_string());
        }

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
        let span = ToolSpanGuard::new("spec:require_bind", &self.webid);

        if let Err(e) = validate_identifier("spec_id", &spec_id, 256) {
            return span.error(e.kind, e.to_json_string());
        }

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

        let boundary = match authority.as_str() {
            "denied" => OCAPBoundary::denied(capability.clone()),
            _ => OCAPBoundary::explicit(capability.clone()),
        };

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
        let span = ToolSpanGuard::new("spec:curate_evaluate", &self.webid);

        if let Err(e) = validate_identifier("spec_id", &spec_id, 256) {
            return span.error(e.kind, e.to_json_string());
        }

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
        let span = ToolSpanGuard::new("spec:curate_reconcile", &self.webid);

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
        let span = ToolSpanGuard::new("spec:curate_cultivate", &self.webid);

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
        let span = ToolSpanGuard::new("spec:graph_query", &self.webid);

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
        let span = ToolSpanGuard::new("spec:graph_validate", &self.webid);

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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_stdio_server(
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
