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
    CompletenessCheck, CurationDecision, DomainAnchor, GoalSpec, OCAPBoundary, Spec,
    SpecCategory,
};
use rmcp::{ServiceExt, handler::server::wrapper::Parameters, tool, tool_router, transport::stdio};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Request types ────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GoalCaptureRequest {
    pub description: String,
    pub category: String,
    pub domain_anchor: String,
    pub criteria: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GoalDecomposeRequest {
    pub spec_id: String,
    pub goal_index: usize,
    pub sub_goals: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RequireBindRequest {
    pub spec_id: String,
    pub goal_index: usize,
    pub capability: String,
    pub authority: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CurateEvaluateRequest {
    pub spec_id: String,
    pub rationale_hint: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CurateReconcileRequest {
    pub spec_ids: Vec<String>,
    pub tension_description: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CurateCultivateRequest {
    pub coherence_threshold: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphQueryRequest {
    pub category: Option<String>,
    pub domain_anchor: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GraphValidateRequest {
    pub coherence_threshold: Option<f64>,
}

// ── Server ───────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct SpecServer {
    specs: Arc<RwLock<HashMap<String, Spec>>>,
}

impl SpecServer {
    pub fn new() -> Self {
        Self::default()
    }

    fn compute_coherence(specs: &[&Spec]) -> f64 {
        if specs.is_empty() {
            return 0.0;
        }
        let complete_count = specs.iter().filter(|s| s.is_complete()).count();
        let categories_coveraged = specs
            .iter()
            .map(|s| s.category.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len();
        let coverage = categories_coveraged as f64 / SpecCategory::all().len() as f64;
        let completeness = complete_count as f64 / specs.len() as f64;
        ((coverage + completeness) / 2.0).clamp(0.0, 1.0)
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
        }): Parameters<GoalCaptureRequest>,
    ) -> String {
        let cat = SpecCategory::parse_str(&category).unwrap_or(SpecCategory::Domain);
        let anchor = DomainAnchor::parse_str(&domain_anchor).unwrap_or(DomainAnchor::Hkask);

        let mut goal = GoalSpec::new(&description);
        if let Some(crits) = criteria {
            for c in crits {
                goal = goal.with_criterion(&c);
            }
        }

        let spec = Spec::new(&description, cat, anchor).with_goal(goal);
        let id = spec.id.to_string();

        let mut specs = self.specs.write().await;
        specs.insert(id.clone(), spec);

        format!(
            r#"{{"spec_id":"{}","category":"{}","domain_anchor":"{}","status":"captured"}}"#,
            id,
            cat.as_str(),
            anchor.as_str()
        )
    }

    #[tool(description = "Decompose a goal into ordered sub-goals (max depth 7)")]
    async fn spec_goal_decompose(
        &self,
        Parameters(GoalDecomposeRequest {
            spec_id,
            goal_index,
            sub_goals,
        }): Parameters<GoalDecomposeRequest>,
    ) -> String {
        let mut specs = self.specs.write().await;
        let Some(spec) = specs.get_mut(&spec_id) else {
            return format!(r#"{{"error":"Spec not found: {}"}}"#, spec_id);
        };
        let Some(goal) = spec.goals.get_mut(goal_index) else {
            return format!(r#"{{"error":"Goal index {} out of range"}}"#, goal_index);
        };

        if !goal.can_have_subgoals() {
            return r#"{"error":"Depth limit reached (max 7)"}"#.to_string();
        }

        let added = sub_goals.len();
        for text in sub_goals {
            let mut child = GoalSpec::new(&text);
            child.depth = goal.depth + 1;
            goal.sub_goals.push(child);
        }

        format!(
            r#"{{"spec_id":"{}","goal_index":{},"sub_goals_added":{}}}"#,
            spec_id, goal_index, added
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
        }): Parameters<RequireBindRequest>,
    ) -> String {
        let specs = self.specs.read().await;
        let Some(spec) = specs.get(&spec_id) else {
            return format!(r#"{{"error":"Spec not found: {}"}}"#, spec_id);
        };
        if goal_index >= spec.goals.len() {
            return format!(r#"{{"error":"Goal index {} out of range"}}"#, goal_index);
        }

        let boundary = match authority.as_str() {
            "denied" => OCAPBoundary::denied(capability.clone()),
            _ => OCAPBoundary::explicit(capability.clone()),
        };

        format!(
            r#"{{"spec_id":"{}","goal_index":{},"capability":"{}","authority":"{}","enforced":{}}}"#,
            spec_id, goal_index, capability, authority, boundary.enforced
        )
    }

    #[tool(description = "Evaluate a specification for collection coherence (curation)")]
    async fn spec_curate_evaluate(
        &self,
        Parameters(CurateEvaluateRequest {
            spec_id,
            rationale_hint,
        }): Parameters<CurateEvaluateRequest>,
    ) -> String {
        let specs = self.specs.read().await;
        let Some(spec) = specs.get(&spec_id) else {
            return format!(r#"{{"error":"Spec not found: {}"}}"#, spec_id);
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

        let coherence = if complete { 1.0 } else { 0.5 };

        format!(
            r#"{{"spec_id":"{}","decision":"{}","rationale":"{}","coherence_score":{}}}"#,
            spec_id, decision, rationale, coherence
        )
    }

    #[tool(description = "Reconcile tensions between specifications without collapsing them")]
    async fn spec_curate_reconcile(
        &self,
        Parameters(CurateReconcileRequest {
            spec_ids,
            tension_description,
        }): Parameters<CurateReconcileRequest>,
    ) -> String {
        let specs = self.specs.read().await;
        let mut found = Vec::new();
        let mut missing = Vec::new();

        for id in &spec_ids {
            if specs.contains_key(id) {
                found.push(id.as_str());
            } else {
                missing.push(id.as_str());
            }
        }

        if !missing.is_empty() {
            return format!(r#"{{"error":"Specs not found: {:?}"}}"#, missing);
        }

        format!(
            r#"{{"resolution":"tensions_preserved","spec_ids":{:?},"tension":"{}","status":"reconciled"}}"#,
            found, tension_description
        )
    }

    #[tool(description = "Cultivate the specification collection toward coherence")]
    async fn spec_curate_cultivate(
        &self,
        Parameters(CurateCultivateRequest {
            coherence_threshold,
        }): Parameters<CurateCultivateRequest>,
    ) -> String {
        let specs = self.specs.read().await;
        let threshold = coherence_threshold.unwrap_or(0.7);
        let all_specs: Vec<&Spec> = specs.values().collect();
        let coherence = Self::compute_coherence(&all_specs);
        let categories_covered: Vec<&str> = all_specs
            .iter()
            .map(|s| s.category.as_str())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let categories_missing: Vec<&str> = SpecCategory::all()
            .iter()
            .map(|c| c.as_str())
            .filter(|c| !categories_covered.contains(c))
            .collect();

        let above_threshold = coherence >= threshold;

        format!(
            r#"{{"coherence_score":{},"threshold":{},"above_threshold":{},"spec_count":{},"categories_covered":{:?},"categories_missing":{:?}}}"#,
            coherence,
            threshold,
            above_threshold,
            all_specs.len(),
            categories_covered,
            categories_missing
        )
    }

    #[tool(description = "Query the specification graph by category or domain anchor")]
    async fn spec_graph_query(
        &self,
        Parameters(GraphQueryRequest {
            category,
            domain_anchor,
        }): Parameters<GraphQueryRequest>,
    ) -> String {
        let specs = self.specs.read().await;
        let results: Vec<&Spec> = specs
            .values()
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

        let nodes: Vec<String> = results
            .iter()
            .map(|s| {
                format!(
                    r#"{{"id":"{}","name":"{}","category":"{}","complete":{}}}"#,
                    s.id,
                    s.name,
                    s.category.as_str(),
                    s.is_complete()
                )
            })
            .collect();

        format!(
            r#"{{"count":{},"specs":[{}]}}"#,
            nodes.len(),
            nodes.join(",")
        )
    }

    #[tool(
        description = "Validate the full specification collection for coherence and completeness"
    )]
    async fn spec_graph_validate(
        &self,
        Parameters(GraphValidateRequest {
            coherence_threshold,
        }): Parameters<GraphValidateRequest>,
    ) -> String {
        let specs = self.specs.read().await;
        let threshold = coherence_threshold.unwrap_or(0.7);
        let all_specs: Vec<&Spec> = specs.values().collect();
        let coherence = Self::compute_coherence(&all_specs);

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

        format!(
            r#"{{"valid":{},"coherence_score":{},"threshold":{},"violations":{:?},"suggestions":{:?},"spec_count":{}}}"#,
            violations.is_empty(),
            coherence,
            threshold,
            violations,
            suggestions,
            all_specs.len()
        )
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = SpecServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-spec started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
