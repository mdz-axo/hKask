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

use hkask_types::{
    CapabilityAction, CapabilityChecker, CapabilityResource, CapabilityToken, CollectionCoherence,
    CompletenessCheck, CurationDecision, DomainAnchor, GoalSpec, OCAPBoundary, Spec, SpecCategory,
    SpecError, SpecId, SpecObserver, SpecStore,
};
use rmcp::{ServiceExt, handler::server::wrapper::Parameters, tool, tool_router, transport::stdio};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

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

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
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
    capability_checker: Option<Arc<CapabilityChecker>>,
    observer: Option<Arc<dyn SpecObserver + Send + Sync>>,
}

impl std::fmt::Debug for SpecServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpecServer")
            .field("store", &"<dyn SpecStore>")
            .field("capability_checker", &self.capability_checker.is_some())
            .field("observer", &self.observer.is_some())
            .finish()
    }
}

impl SpecServer {
    pub fn new(store: Arc<dyn SpecStore + Send + Sync>) -> Self {
        Self {
            store,
            capability_checker: None,
            observer: None,
        }
    }

    pub fn with_capability_checker(mut self, checker: CapabilityChecker) -> Self {
        self.capability_checker = Some(Arc::new(checker));
        self
    }

    pub fn with_observer(mut self, observer: Arc<dyn SpecObserver + Send + Sync>) -> Self {
        self.observer = Some(observer);
        self
    }

    fn save_and_observe(&self, spec: &Spec, operation: &str) -> Result<(), SpecError> {
        match self.store.save(spec) {
            Ok(()) => Ok(()),
            Err(e) => {
                if let Some(obs) = &self.observer {
                    obs.emit_span(
                        spec.id,
                        &format!("{}.error", operation),
                        &serde_json::json!({"error": e.to_string()}),
                    );
                }
                Err(e)
            }
        }
    }

    fn verify_capability(
        &self,
        token_b64: Option<&str>,
        resource_id: &str,
        action: CapabilityAction,
    ) -> Result<(), SpecError> {
        match (&self.capability_checker, token_b64) {
            (None, None) => {
                tracing::warn!(
                    resource_id,
                    action = action.as_str(),
                    "OCAP open mode: no capability checker configured"
                );
                Ok(())
            }
            (Some(_checker), None) => Err(SpecError::CapabilityDenied(format!(
                "No capability token provided for spec:{}:{}",
                resource_id,
                action.as_str()
            ))),
            (_, Some(b64)) => {
                let checker = self.capability_checker.as_ref().unwrap();
                let token = CapabilityToken::from_base64(b64).map_err(|e| {
                    SpecError::CapabilityDenied(format!("Invalid token encoding: {}", e))
                })?;
                if !checker.verify(&token) {
                    return Err(SpecError::CapabilityDenied(
                        "Token signature verification failed".to_string(),
                    ));
                }
                let now = chrono::Utc::now().timestamp();
                if token.is_expired(now) {
                    return Err(SpecError::CapabilityDenied("Token expired".to_string()));
                }
                if !token.is_valid_for(CapabilityResource::Spec, resource_id, action) {
                    return Err(SpecError::CapabilityDenied(format!(
                        "Token does not grant spec:{}:{}",
                        resource_id,
                        action.as_str()
                    )));
                }
                Ok(())
            }
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
        if let Err(e) = self.verify_capability(
            capability_token.as_deref(),
            "capture",
            CapabilityAction::Write,
        ) {
            return serde_json::to_string(&ErrorResponse {
                error: e.to_string(),
            })
            .unwrap_or_else(|_| "{}".into());
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

        if let Err(e) = self.save_and_observe(&spec, "spec.capture") {
            return serde_json::to_string(&ErrorResponse {
                error: format!("Failed to persist spec: {}", e),
            })
            .unwrap_or_else(|_| "{}".into());
        }

        serde_json::to_string(&GoalCaptureResponse {
            spec_id: id.to_string(),
            category: cat.as_str().to_string(),
            domain_anchor: anchor.as_str().to_string(),
            status: "captured".to_string(),
        })
        .unwrap_or_else(|_| "{}".into())
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
        if let Err(e) = self.verify_capability(
            capability_token.as_deref(),
            &spec_id,
            CapabilityAction::Write,
        ) {
            return serde_json::to_string(&ErrorResponse {
                error: e.to_string(),
            })
            .unwrap_or_else(|_| "{}".into());
        }

        let spec_id_parsed = SpecId::from_string(&spec_id).unwrap_or(SpecId::new());
        let mut spec = match self.store.load(spec_id_parsed) {
            Ok(s) => s,
            Err(_) => {
                return serde_json::to_string(&ErrorResponse {
                    error: format!("Spec not found: {}", spec_id),
                })
                .unwrap_or_else(|_| "{}".into());
            }
        };

        let Some(goal) = spec.goals.get_mut(goal_index) else {
            return serde_json::to_string(&ErrorResponse {
                error: format!("Goal index {} out of range", goal_index),
            })
            .unwrap_or_else(|_| "{}".into());
        };

        if !goal.can_have_subgoals() {
            return serde_json::to_string(&ErrorResponse {
                error: "Depth limit reached (max 7)".to_string(),
            })
            .unwrap_or_else(|_| "{}".into());
        }

        let added = sub_goals.len();
        for text in sub_goals {
            let mut child = GoalSpec::new(&text);
            child.depth = goal.depth + 1;
            goal.sub_goals.push(child);
        }

        if let Err(e) = self.save_and_observe(&spec, "spec.decompose") {
            return serde_json::to_string(&ErrorResponse {
                error: format!("Failed to persist spec: {}", e),
            })
            .unwrap_or_else(|_| "{}".into());
        }

        serde_json::to_string(&GoalDecomposeResponse {
            spec_id,
            goal_index,
            sub_goals_added: added,
        })
        .unwrap_or_else(|_| "{}".into())
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
        if let Err(e) = self.verify_capability(
            capability_token.as_deref(),
            &spec_id,
            CapabilityAction::Write,
        ) {
            return serde_json::to_string(&ErrorResponse {
                error: e.to_string(),
            })
            .unwrap_or_else(|_| "{}".into());
        }

        let spec_id_parsed = SpecId::from_string(&spec_id).unwrap_or(SpecId::new());
        let mut spec = match self.store.load(spec_id_parsed) {
            Ok(s) => s,
            Err(_) => {
                return serde_json::to_string(&ErrorResponse {
                    error: format!("Spec not found: {}", spec_id),
                })
                .unwrap_or_else(|_| "{}".into());
            }
        };
        if goal_index >= spec.goals.len() {
            return serde_json::to_string(&ErrorResponse {
                error: format!("Goal index {} out of range", goal_index),
            })
            .unwrap_or_else(|_| "{}".into());
        }

        let boundary = match authority.as_str() {
            "denied" => OCAPBoundary::denied(capability.clone()),
            _ => OCAPBoundary::explicit(capability.clone()),
        };

        spec.goals[goal_index].constraints.push(boundary);

        if let Err(e) = self.save_and_observe(&spec, "spec.bind") {
            return serde_json::to_string(&ErrorResponse {
                error: format!("Failed to persist spec: {}", e),
            })
            .unwrap_or_else(|_| "{}".into());
        }

        serde_json::to_string(&RequireBindResponse {
            spec_id,
            goal_index,
            capability,
            authority,
            enforced: true,
        })
        .unwrap_or_else(|_| "{}".into())
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
        if let Err(e) = self.verify_capability(
            capability_token.as_deref(),
            &spec_id,
            CapabilityAction::Read,
        ) {
            return serde_json::to_string(&ErrorResponse {
                error: e.to_string(),
            })
            .unwrap_or_else(|_| "{}".into());
        }

        let spec_id_parsed = SpecId::from_string(&spec_id).unwrap_or(SpecId::new());
        let spec = match self.store.load(spec_id_parsed) {
            Ok(s) => s,
            Err(_) => {
                return serde_json::to_string(&ErrorResponse {
                    error: format!("Spec not found: {}", spec_id),
                })
                .unwrap_or_else(|_| "{}".into());
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

        serde_json::to_string(&CurateEvaluateResponse {
            spec_id,
            decision: decision.to_string(),
            rationale,
            coherence_score: coherence,
        })
        .unwrap_or_else(|_| "{}".into())
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
        if let Err(e) = self.verify_capability(
            capability_token.as_deref(),
            "reconcile",
            CapabilityAction::Compose,
        ) {
            return serde_json::to_string(&ErrorResponse {
                error: e.to_string(),
            })
            .unwrap_or_else(|_| "{}".into());
        }

        let mut found = Vec::new();
        let mut missing = Vec::new();

        for id in &spec_ids {
            let parsed = SpecId::from_string(id).unwrap_or(SpecId::new());
            if self.store.load(parsed).is_ok() {
                found.push(id.clone());
            } else {
                missing.push(id.as_str());
            }
        }

        if !missing.is_empty() {
            return serde_json::to_string(&ErrorResponse {
                error: format!("Specs not found: {:?}", missing),
            })
            .unwrap_or_else(|_| "{}".into());
        }

        let mut loaded_specs = Vec::new();
        for id in &found {
            let parsed = SpecId::from_string(id).unwrap_or(SpecId::new());
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

        serde_json::to_string(&CurateReconcileResponse {
            resolution: resolution.to_string(),
            spec_ids: found,
            tension: tension_description,
            tensions,
            status: "reconciled".to_string(),
        })
        .unwrap_or_else(|_| "{}".into())
    }

    #[tool(description = "Cultivate the specification collection toward coherence")]
    async fn spec_curate_cultivate(
        &self,
        Parameters(CurateCultivateRequest {
            coherence_threshold,
            capability_token,
        }): Parameters<CurateCultivateRequest>,
    ) -> String {
        if let Err(e) = self.verify_capability(
            capability_token.as_deref(),
            "cultivate",
            CapabilityAction::Compose,
        ) {
            return serde_json::to_string(&ErrorResponse {
                error: e.to_string(),
            })
            .unwrap_or_else(|_| "{}".into());
        }

        let threshold = coherence_threshold.unwrap_or(0.7);
        let all_specs: Vec<Spec> = match self.store.list_all() {
            Ok(specs) => specs,
            Err(e) => {
                return serde_json::to_string(&ErrorResponse {
                    error: format!("Failed to load specs: {}", e),
                })
                .unwrap_or_else(|_| "{}".into());
            }
        };
        let coherence = all_specs.as_slice().collection_coherence();
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

        serde_json::to_string(&CurateCultivateResponse {
            coherence_score: coherence,
            threshold,
            above_threshold,
            spec_count: all_specs.len(),
            categories_covered,
            categories_missing,
        })
        .unwrap_or_else(|_| "{}".into())
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
        if let Err(e) =
            self.verify_capability(capability_token.as_deref(), "query", CapabilityAction::Read)
        {
            return serde_json::to_string(&ErrorResponse {
                error: e.to_string(),
            })
            .unwrap_or_else(|_| "{}".into());
        }

        let all_specs: Vec<Spec> = match self.store.list_all() {
            Ok(specs) => specs,
            Err(e) => {
                return serde_json::to_string(&ErrorResponse {
                    error: format!("Failed to load specs: {}", e),
                })
                .unwrap_or_else(|_| "{}".into());
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

        serde_json::to_string(&GraphQueryResponse {
            count: nodes.len(),
            specs: nodes,
        })
        .unwrap_or_else(|_| "{}".into())
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
        if let Err(e) = self.verify_capability(
            capability_token.as_deref(),
            "validate",
            CapabilityAction::Validate,
        ) {
            return serde_json::to_string(&ErrorResponse {
                error: e.to_string(),
            })
            .unwrap_or_else(|_| "{}".into());
        }

        let threshold = coherence_threshold.unwrap_or(0.7);
        let all_specs: Vec<Spec> = match self.store.list_all() {
            Ok(specs) => specs,
            Err(e) => {
                return serde_json::to_string(&ErrorResponse {
                    error: format!("Failed to load specs: {}", e),
                })
                .unwrap_or_else(|_| "{}".into());
            }
        };
        let coherence = all_specs.as_slice().collection_coherence();

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

        serde_json::to_string(&GraphValidateResponse {
            valid: violations.is_empty(),
            coherence_score: coherence,
            threshold,
            violations,
            suggestions,
            spec_count: all_specs.len(),
        })
        .unwrap_or_else(|_| "{}".into())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let conn = rusqlite::Connection::open_in_memory()?;
    let conn = std::sync::Arc::new(std::sync::Mutex::new(conn));
    let store = std::sync::Arc::new(hkask_storage::SqliteSpecStore::new(conn));
    store.init_schema().map_err(|e| anyhow::anyhow!("{}", e))?;

    let server = SpecServer::new(store);
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-spec started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
