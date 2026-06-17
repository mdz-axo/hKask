//! hKask MCP Spec — Specification authoring, validation, graph analysis, and replica rewriting (6 tools per MDS §3)
//!
//! Curation (Accept/Revise/Reject) is external to the spec server — performed by
//! the Curator agent or human. The spec server handles capture, decompose,
//! writing-quality, graph query, graph coherence, and replica rewrite.

pub mod types;

use hkask_mcp::server::{McpToolError, ServerContext, ToolSpanGuard};
use hkask_mcp::validate_field;

use hkask_cns::{emit_contract_accepted, emit_contract_proposed, emit_contract_rejected};
use hkask_inference::{EmbeddingRouter, InferenceConfig};
use hkask_services::{
    CognitionConfig, ComposeRequest, ComposeService, EmbeddingSection, HkaskSettings,
    InferenceContext, RetrievalSection, ValidationSection, cosine_distance,
};
use hkask_storage::spec_types::{
    DomainAnchor, GoalSpec, Spec, SpecCategory, SpecError, SpecId, infer_spec_category,
};
use hkask_storage::{Database, EmbeddingStore, NuEventStore, SpecStore, TripleStore};
use hkask_types::time::now_rfc3339;
use hkask_types::{
    CapabilityChecker, DelegationAction, DelegationResource, DelegationToken, McpErrorKind,
    TOKEN_ERR_EXPIRED, TOKEN_ERR_INVALID_SIGNATURE, TOKEN_ERR_NO_CHECKER, VerificationOutcome,
    WebID, token_err_insufficient_access, verify_delegation_token_now,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use std::sync::Arc;
use std::time::Instant;
use types::*;

// ── Server ───────────────────────────────────────────────────

/// Spec MCP server — provides spec *mechanism*, not *governance*.
///
/// Six tools (MDS §3): capture, decompose, writing-quality, graph-query,
/// graph-coherence, and replica-rewrite. All tools are OCAP-gated.
///
/// **Governance boundary**: Curation decisions (Accept/Revise/Reject) are
/// external to this server — made by the Curator agent or human, never by
/// a tool call. This server handles mechanism (capture, decompose, query,
/// assess); the Curator handles governance (decide, accept, reject).
pub struct SpecServer {
    store: Arc<dyn SpecStore + Send + Sync>,
    capability_checker: Arc<CapabilityChecker>,
    webid: WebID,
    /// Replicant identity serving this MCP server (for narrative memory)
    replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    daemon: Option<hkask_mcp::DaemonClient>,
    /// Event sink for CNS span persistence — proposals flow to Curator's review queue.
    event_sink: Arc<dyn hkask_types::event::NuEventSink>,
    /// Triple store for proposal persistence.
    triple_store: Arc<TripleStore>,
}

impl std::fmt::Debug for SpecServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpecServer")
            .field("store", &"<dyn SpecStore>")
            .field("webid", &self.webid)
            .finish()
    }
}

// Capability-check macro — covers 5 tool handlers
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
        replicant: String,
        daemon: Option<hkask_mcp::DaemonClient>,
        event_sink: Arc<dyn hkask_types::event::NuEventSink>,
        triple_store: Arc<TripleStore>,
    ) -> Self {
        Self {
            store,
            capability_checker: Arc::new(capability_checker),
            webid,
            replicant,
            daemon,
            event_sink,
            triple_store,
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

    /// Extract OCAP boundary hints from context keywords.
    fn extract_ocap_boundaries(context: Option<&str>) -> Vec<String> {
        let ctx = match context {
            Some(c) => c.to_lowercase(),
            None => return vec![],
        };
        let mut boundaries = Vec::new();
        if ctx.contains("curation") || ctx.contains("curat") {
            boundaries.push("curation".to_string());
        }
        if ctx.contains("cybernetics") || ctx.contains("cns") {
            boundaries.push("cybernetics".to_string());
        }
        if ctx.contains("spec_curate") || ctx.contains("spec curate") {
            boundaries.push("spec_curate".to_string());
        }
        boundaries
    }

    /// Writing quality assessment for a spec.
    ///
    /// When `replica_persona` and DB credentials are provided, performs
    /// embedding-based comparison against the persona's dimension centroids.
    /// Otherwise falls back to the structural heuristic.
    async fn assess_writing_quality(
        &self,
        spec: &Spec,
        replica_persona: Option<&str>,
        db_path: Option<&str>,
        db_passphrase: Option<&str>,
    ) -> (WritingQualityScore, Option<Vec<DimensionScore>>) {
        // ── Structural heuristic (always computed) ─────────────────
        let has_description = !spec.name.is_empty();
        let has_goals = !spec.goals.is_empty();
        let has_criteria = spec.goals.iter().any(|g| !g.criteria.is_empty());
        let has_verbs = !spec.declared_verbs.is_empty();

        let heuristic = WritingQualityScore {
            hopper: has_goals && has_criteria,
            lovelace: has_criteria,
            schriver: has_description && has_goals,
            gentle: has_description && has_verbs,
        };

        // ── Embedding-based comparison (when replica + DB available) ─
        let dimension_scores = match (replica_persona, db_path, db_passphrase) {
            (Some(persona), Some(path), Some(passphrase)) => {
                match self
                    .compare_against_replica(spec, persona, path, passphrase)
                    .await
                {
                    Ok(scores) => Some(scores),
                    Err(e) => {
                        tracing::warn!(
                            target: "hkask.mcp.spec",
                            persona = %persona,
                            error = %e,
                            "Replica comparison failed, using heuristic only"
                        );
                        None
                    }
                }
            }
            _ => None,
        };

        (heuristic, dimension_scores)
    }

    /// Embed spec content and compute cosine distances against persona centroids.
    async fn compare_against_replica(
        &self,
        spec: &Spec,
        persona: &str,
        db_path: &str,
        db_passphrase: &str,
    ) -> Result<Vec<DimensionScore>, String> {
        let db = Database::open(db_path, db_passphrase).map_err(|e| e.to_string())?;
        let conn = db.conn_arc();
        let store = EmbeddingStore::new(conn);

        // Build document text from spec content
        let doc_text = format!(
            "{}: Goals: {}. Criteria: {}.",
            spec.name,
            spec.goals
                .iter()
                .map(|g| g.text.as_str())
                .collect::<Vec<_>>()
                .join("; "),
            spec.goals
                .iter()
                .flat_map(|g| &g.criteria)
                .map(|c| c.description.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        );

        // Embed the document
        let settings = HkaskSettings::load();
        let emb_model = settings.embedding_model();
        let inf_cfg = InferenceConfig::from_env();
        let embedder = EmbeddingRouter::new(inf_cfg);
        let vectors = embedder
            .embed_sentences(&emb_model, &[doc_text.as_str()])
            .await
            .map_err(|e| format!("Failed to embed document: {e}"))?;
        let doc_vec = vectors
            .first()
            .ok_or_else(|| "Embedding returned empty result".to_string())?;

        // Query centroids for this persona
        let prefix = format!("style:{}:", persona);
        let all_refs = store.query_by_prefix(&prefix).map_err(|e| e.to_string())?;

        let mut dimension_scores: Vec<DimensionScore> = Vec::new();

        for entity_ref in &all_refs {
            // Only process centroid entities
            let last_segment = entity_ref.rsplit(':').next().unwrap_or(entity_ref);
            if !last_segment.ends_with("-centroid") && last_segment != "centroid" {
                continue;
            }

            let emb = store.get(entity_ref).map_err(|e| e.to_string())?;
            let dist = cosine_distance(doc_vec, &emb.vector);

            // Derive dimension name from entity_ref
            let dimension = if last_segment == "centroid" {
                "composite".to_string()
            } else if let Some(dim) = last_segment.strip_suffix("-centroid") {
                dim.to_string()
            } else {
                continue;
            };

            let dim_passage_count = all_refs
                .iter()
                .filter(|r| {
                    let seg = r.rsplit(':').next().unwrap_or(r);
                    !seg.ends_with("-centroid")
                        && seg != "centroid"
                        && r.to_lowercase().contains(&dimension)
                })
                .count();

            dimension_scores.push(DimensionScore {
                dimension,
                centroid_ref: entity_ref.clone(),
                cosine_distance: dist,
                qualitative: if dist <= 0.2 {
                    "strong".to_string()
                } else if dist <= 0.4 {
                    "aligned".to_string()
                } else {
                    "divergent".to_string()
                },
                passage_count: dim_passage_count,
            });
        }

        Ok(dimension_scores)
    }

    /// Basic goal decomposition: split description into sentences as sub-goals.
    fn decompose_description(description: &str) -> (Vec<String>, Vec<DependencyEdge>) {
        let sub_goals: Vec<String> = description
            .split(['.', '\n'])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        // Sequential dependencies: each sub-goal depends on the previous
        let mut dependencies = Vec::new();
        for i in 1..sub_goals.len() {
            dependencies.push(DependencyEdge {
                from: sub_goals[i - 1].clone(),
                to: sub_goals[i].clone(),
            });
        }
        (sub_goals, dependencies)
    }

    /// Record a tool call as a narrative experience in the agent's memory.
    fn record_experience(
        &self,
        tool: &str,
        input_summary: &str,
        outcome: &str,
        detail: serde_json::Value,
    ) {
        if let Some(ref daemon) = self.daemon {
            let value = serde_json::json!({
                "tool": tool,
                "input": input_summary,
                "outcome": outcome,
                "detail": detail,
                "timestamp": now_rfc3339(),
            });
            let daemon_clone = daemon.clone();
            let replicant = self.replicant.clone();
            let tool_name = tool.to_string();
            tokio::spawn(async move {
                match daemon_clone
                    .store_experience(&replicant, "mcp_session", "observed", &value, Some(0.85))
                    .await
                {
                    Ok(hkask_mcp::DaemonResponse::StoreResponse { stored: true, .. }) => {
                        tracing::debug!(target: "hkask.mcp.spec.memory", tool = %tool_name, "Experience stored via daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.spec.memory", tool = %tool_name, response = ?other, "Unexpected daemon response")
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.spec.memory", tool = %tool_name, error = %e, "Failed to store experience")
                    }
                }
            });
        }
    }
}

/// Serialize response and convert to ok_json.
/// Returns an internal error span entry if serialization fails (e.g. NaN/Inf in an f64 field).
fn respond<T: serde::Serialize>(span: ToolSpanGuard, resp: &T) -> String {
    match serde_json::to_value(resp) {
        Ok(val) => span.ok_json(val),
        Err(e) => {
            span.internal_error(serde_json::json!({"error": format!("serialization failed: {e}")}))
        }
    }
}

#[tool_router(server_handler)]
impl SpecServer {
    /// MDS §3 tool 1: Capture a goal as a binding specification requirement.
    /// OCAP boundaries are declared inline from context.
    #[tool(
        description = "Capture a goal as a binding specification requirement. OCAP boundaries are declared inline from context per MDS §3."
    )]
    async fn spec_goal_capture(
        &self,
        Parameters(GoalCaptureRequest {
            description,
            context,
            capability_token,
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

        let category = infer_spec_category(context.as_deref());
        let anchor = DomainAnchor::Hkask;
        let ocap_boundaries = Self::extract_ocap_boundaries(context.as_deref());

        let mut goal = GoalSpec::new(&description);
        // Seed criteria from description sentences
        for sentence in description.split('.') {
            let trimmed = sentence.trim();
            if !trimmed.is_empty() && trimmed.len() < 200 {
                goal = goal.with_criterion(trimmed);
            }
        }

        let spec = Spec::new(&description, category, anchor).with_goal(goal);
        let spec_id = spec.id;

        if let Err(v) = self.persist_val(&spec) {
            return span.internal_error(v);
        }

        let requirements: Vec<String> = spec
            .goals
            .iter()
            .flat_map(|g| g.criteria.iter().map(|c| c.description.clone()))
            .collect();

        self.record_experience(
            "spec_goal_capture",
            &description,
            "captured",
            serde_json::json!({"goal_id": spec_id.to_string(), "category": category.as_str()}),
        );

        respond(
            span,
            &GoalCaptureResponse {
                goal_id: spec_id.to_string(),
                requirements,
                ocap_boundaries,
            },
        )
    }

    /// MDS §3 tool 2: Decompose a specification goal into ordered sub-goals with dependencies.
    #[tool(
        description = "Decompose a specification goal into ordered sub-goals with dependencies per MDS §3"
    )]
    async fn spec_goal_decompose(
        &self,
        Parameters(GoalDecomposeRequest {
            goal_id,
            capability_token,
        }): Parameters<GoalDecomposeRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_goal_decompose", &self.webid);
        validate_field!(span, "goal_id", &goal_id, 256);
        check_cap!(
            self,
            span,
            capability_token,
            &goal_id,
            DelegationAction::Write
        );

        let mut spec = match self.load_spec(&goal_id) {
            Ok(s) => s,
            Err((kind, msg)) => return span.error(kind, msg),
        };

        // Decompose each goal that can have subgoals
        for goal in &mut spec.goals {
            if !goal.can_have_subgoals() || !goal.sub_goals.is_empty() {
                continue;
            }
            let (sub_texts, _deps) = Self::decompose_description(&goal.text);
            if sub_texts.len() <= 1 {
                continue;
            }
            for text in &sub_texts {
                let mut child = GoalSpec::new(text);
                child.depth = goal.depth + 1;
                goal.sub_goals.push(child);
            }
        }

        let (sub_goals, dependencies) = {
            let mut all_subs = Vec::new();
            let mut all_deps = Vec::new();
            for goal in &spec.goals {
                for sub in &goal.sub_goals {
                    all_subs.push(sub.text.clone());
                }
            }
            if all_subs.len() > 1 {
                for i in 1..all_subs.len() {
                    all_deps.push(DependencyEdge {
                        from: all_subs[i - 1].clone(),
                        to: all_subs[i].clone(),
                    });
                }
            }
            (all_subs, all_deps)
        };

        if let Err(v) = self.persist_val(&spec) {
            return span.internal_error(v);
        }

        self.record_experience(
            "spec_goal_decompose",
            &goal_id,
            "decomposed",
            serde_json::json!({"sub_goal_count": sub_goals.len()}),
        );

        respond(
            span,
            &GoalDecomposeResponse {
                sub_goals,
                dependencies,
            },
        )
    }

    /// MDS §3 tool 3: Assess a specification's writing quality via the 4-perspective test.
    /// The server assesses, not the caller. 3 of 4 passing = meets publication standard.
    #[tool(
        description = "Assess a specification's writing quality via the 4-perspective test (Hopper/Lovelace/Schriver/Gentle) per MDS §3. 3/4 = publishable."
    )]
    async fn spec_require_writing_quality(
        &self,
        Parameters(WritingQualityRequest {
            spec_id,
            notes: _,
            replica_persona,
            db_path,
            db_passphrase,
            capability_token,
        }): Parameters<WritingQualityRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_require_writing_quality", &self.webid);
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

        let (quality, dimension_scores) = self
            .assess_writing_quality(
                &spec,
                replica_persona.as_deref(),
                db_path.as_deref(),
                db_passphrase.as_deref(),
            )
            .await;

        // When embedding scores are available, use them for the pass/fail decision.
        // A dimension passes if cosine_distance ≤ 0.4.
        let (dimensions_passing, meets_standard, weakest_dimension, rewrite_prompt) =
            match &dimension_scores {
                Some(scores) if !scores.is_empty() => {
                    let passing = scores.iter().filter(|s| s.cosine_distance <= 0.4).count();
                    // Find the weakest dimension (highest cosine distance, excluding composite)
                    let weakest = scores
                        .iter()
                        .filter(|s| s.dimension != "composite")
                        .max_by(|a, b| a.cosine_distance.total_cmp(&b.cosine_distance));
                    let weakest_dim = weakest.map(|s| s.dimension.clone());
                    let rewrite = weakest.and_then(|s| {
                        if s.cosine_distance > 0.4 {
                            Some(format!(
                                "Rewrite this specification to improve its {} dimension (current cosine distance: {:.2}, threshold: 0.40).\n\n=== SPECIFICATION TO REWRITE ===\n\nName: {}\nGoals: {}\nCriteria: {}",
                                s.dimension,
                                s.cosine_distance,
                                spec.name,
                                spec.goals.iter().map(|g| g.text.as_str()).collect::<Vec<_>>().join("; "),
                                spec.goals.iter().flat_map(|g| &g.criteria).map(|c| c.description.as_str()).collect::<Vec<_>>().join("; "),
                            ))
                        } else {
                            None
                        }
                    });
                    (passing, passing >= 3, weakest_dim, rewrite)
                }
                _ => (
                    quality.passes(),
                    quality.meets_publication_standard(),
                    None,
                    None,
                ),
            };

        respond(
            span,
            &WritingQualityResponse {
                dimensions_passing,
                meets_publication_standard: meets_standard,
                replica_persona: replica_persona.clone(),
                dimension_scores,
                weakest_dimension,
                rewrite_prompt,
            },
        )
    }

    /// MDS §3 tool 4: Query the specification graph by search term with configurable depth.
    #[tool(
        description = "Query the specification graph by search term with configurable traversal depth per MDS §3"
    )]
    async fn spec_graph_query(
        &self,
        Parameters(GraphQueryRequest {
            query,
            depth,
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

        let max_depth = depth.unwrap_or(3);
        let all_specs = match self.load_all_specs_val() {
            Ok(specs) => specs,
            Err(v) => return span.internal_error(v),
        };

        let query_lower = query.to_lowercase();

        // Match specs where name, goals, or category contain the query
        let nodes: Vec<GraphNode> = all_specs
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&query_lower)
                    || s.goals
                        .iter()
                        .any(|g| g.text.to_lowercase().contains(&query_lower))
                    || s.category.as_str().contains(&query_lower)
            })
            .map(|s| GraphNode {
                id: s.id.to_string(),
                label: s.name.clone(),
                category: s.category.as_str().to_string(),
            })
            .collect();

        // Build edges between specs in the same category (composition adjacency)
        let mut edges = Vec::new();
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                if nodes[i].category == nodes[j].category {
                    edges.push(GraphEdge {
                        from: nodes[i].id.clone(),
                        to: nodes[j].id.clone(),
                        relation: "same-category".to_string(),
                    });
                }
            }
        }

        // Build simple paths (direct category-linked chains up to max_depth)
        let mut paths = Vec::new();
        for node in &nodes {
            let linked: Vec<String> = edges
                .iter()
                .filter(|e| e.from == node.id || e.to == node.id)
                .flat_map(|e| vec![e.from.clone(), e.to.clone()])
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .take(max_depth as usize)
                .collect();
            if !linked.is_empty() {
                paths.push(GraphPath {
                    nodes: linked,
                    length: 1,
                });
            }
        }

        self.record_experience(
            "spec_graph_query",
            &query,
            "success",
            serde_json::json!({"nodes": nodes.len(), "edges": edges.len(), "paths": paths.len()}),
        );

        respond(
            span,
            &GraphQueryResponse {
                nodes,
                edges,
                paths,
            },
        )
    }

    /// MDS §3 tool 5: Validate specification collection coherence.
    #[tool(
        description = "Validate specification collection for internal consistency and coherence per MDS §3"
    )]
    async fn spec_graph_coherence(
        &self,
        Parameters(GraphCoherenceRequest {
            collection_id: _,
            capability_token,
        }): Parameters<GraphCoherenceRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_graph_coherence", &self.webid);
        check_cap!(
            self,
            span,
            capability_token,
            "coherence",
            DelegationAction::Read
        );

        let all_specs = match self.load_all_specs_val() {
            Ok(specs) => specs,
            Err(v) => return span.internal_error(v),
        };

        let coherence = Spec::collection_coherence(&all_specs);
        let threshold = 0.7;
        let mut violations = Vec::new();
        let mut suggestions = Vec::new();

        if coherence < threshold {
            violations.push(format!(
                "Collection coherence {:.2} below threshold {:.2}",
                coherence, threshold
            ));
        }

        let categories_covered: std::collections::HashSet<String> = all_specs
            .iter()
            .map(|s| s.category.as_str().to_string())
            .collect();
        for cat in SpecCategory::all() {
            if !categories_covered.contains(cat.as_str()) {
                suggestions.push(format!("Missing category: {}", cat.as_str()));
            }
        }

        for spec in &all_specs {
            if !spec.is_complete() {
                suggestions.push(format!("Incomplete spec: {} ({})", spec.id, spec.name));
            }
        }

        respond(
            span,
            &GraphCoherenceResponse {
                coherence_score: coherence,
                violations,
                suggestions,
            },
        )
    }

    /// Rewrite a passage or document using the Gentle Lovelace replica persona.
    /// Retrieves exemplar passages from the target dimension's centroid and
    /// generates improved prose optimized for that dimension of excellence.
    #[tool(
        description = "Rewrite a passage or document using the Gentle Lovelace replica. Optimizes prose for a target quality dimension (Gentle/Schriver/Hopper/Lovelace) using exemplar retrieval and centroid-guided generation."
    )]
    async fn spec_replica_rewrite(
        &self,
        Parameters(ReplicaRewriteRequest {
            passage,
            dimension,
            document_type: _,
            db_path,
            db_passphrase,
            capability_token,
        }): Parameters<ReplicaRewriteRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("spec_replica_rewrite", &self.webid);
        let started = Instant::now();

        check_cap!(
            self,
            span,
            capability_token,
            "rewrite",
            DelegationAction::Read
        );

        // Build dimension-specific rewrite prompt
        let dimension_guidance = match dimension.to_lowercase().as_str() {
            "gentle" => {
                "Rewrite this text to maximize agent-correctness. Docs ARE code — ensure every statement is actionable and unambiguous. Remove any stale references or outdated information."
            }
            "schriver" => {
                "Rewrite this text for maximum findability. Use scannable headings, descriptive hyperlinks, and front-load key concepts. A reader must find their answer within 30 seconds."
            }
            "hopper" => {
                "Rewrite this text for maximum accessibility. Make it comprehensible on first reading with zero prior context. Use plain language, active voice, and short sentences."
            }
            "lovelace" => {
                "Rewrite this text for maximum precision. Make every specification independently verifiable — a reader must be able to write a test from this text alone."
            }
            _ => {
                "Rewrite this text for all four dimensions of documentation excellence: agent-correctness (Gentle), findability (Schriver), accessibility (Hopper), and precision (Lovelace)."
            }
        };

        let prompt = format!("{dimension_guidance}\n\n=== TEXT TO REWRITE ===\n\n{passage}");

        // Target the appropriate centroid
        let centroid_ref = if dimension.to_lowercase() == "composite" {
            "style:gentle-lovelace:centroid".to_string()
        } else {
            format!(
                "style:gentle-lovelace:{}-centroid",
                dimension.to_lowercase()
            )
        };

        let run = async {
            let settings = HkaskSettings::load();
            let gen_model = settings.generation_model();
            let emb_model = settings.embedding_model();

            let inf_cfg = InferenceConfig::from_env();
            let inference_ctx = InferenceContext::from_parts(None, &gen_model, inf_cfg);

            let config = CognitionConfig {
                author: "gentle-lovelace".to_string(),
                jinja2_template: None,
                embedding: EmbeddingSection {
                    model: emb_model.clone(),
                    dim: 1024,
                    centroid_entity_ref: centroid_ref,
                    retrieval: RetrievalSection::default(),
                },
                validation: ValidationSection {
                    centroid_distance_max: 0.40,
                },
            };

            let request = ComposeRequest {
                prompt,
                db_path: std::path::PathBuf::from(&db_path),
                db_passphrase: db_passphrase.clone(),
                cognition: config,
                inference_ctx,
                no_validate: false,
            };

            ComposeService::compose(request)
                .await
                .map_err(|e| e.to_string())
        };

        match run.await {
            Ok(result) => {
                let output = serde_json::to_value(&ReplicaRewriteResponse {
                    rewritten: result.generated_prose,
                    dimension: dimension.clone(),
                    exemplar_count: result.exemplar_count,
                    centroid_distance: result.validation.as_ref().map(|v| v.distance),
                    elapsed_ms: started.elapsed().as_millis() as u64,
                })
                .unwrap_or(serde_json::json!({"error": "serialization failed"}));

                self.record_experience(
                    "spec_replica_rewrite",
                    &dimension,
                    "success",
                    output.clone(),
                );
                span.ok_json(output)
            }
            Err(e) => span.internal_error(serde_json::json!({"error": e})),
        }
    }

    /// Discover uncontracted public functions in a crate.
    /// Returns per-crate coverage percentages and lists of uncontracted functions
    /// for replicant-driven contract proposals.
    #[tool(
        description = "Discover uncontracted public functions in a crate. Returns coverage percentages and lists of functions lacking REQ contracts for replicant-driven proposals."
    )]
    async fn contract_audit(
        &self,
        Parameters(ContractAuditRequest {
            crate_name,
            workspace_root,
        }): Parameters<ContractAuditRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("contract_audit", &self.webid);

        let root = workspace_root.unwrap_or_else(|| {
            std::env::var("HKASK_WORKSPACE_ROOT").unwrap_or_else(|_| {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string())
            })
        });

        let crates: Vec<String> = if let Some(c) = crate_name {
            vec![c]
        } else {
            let crates_dir = std::path::Path::new(&root).join("crates");
            match std::fs::read_dir(&crates_dir) {
                Ok(entries) => entries
                    .flatten()
                    .filter(|e| e.path().is_dir())
                    .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                    .filter(|s| s.starts_with("hkask-"))
                    .collect(),
                Err(_) => return span.internal_error(
                    serde_json::json!({"error": format!("crates directory not found: {}", crates_dir.display())})
                ),
            }
        };

        let mut crate_results = Vec::new();
        let mut total_fns = 0usize;
        let mut total_contracted = 0usize;
        let mut total_uncontracted = 0usize;

        for name in &crates {
            match hkask_test_harness::test_runner::discover_uncontracted_functions(name, &root) {
                Some(audit) => {
                    total_fns += audit.total_pub_fns;
                    total_contracted += audit.contracted;
                    total_uncontracted += audit.uncontracted.len();
                    crate_results.push(CrateCoverage {
                        crate_name: name.clone(),
                        total_pub_fns: audit.total_pub_fns,
                        contracted: audit.contracted,
                        coverage_pct: audit.coverage_pct,
                        uncontracted: audit
                            .uncontracted
                            .iter()
                            .map(|f| UncontractedFn {
                                function_name: f.function_name.clone(),
                                file: f.file.clone(),
                                line: f.line,
                            })
                            .collect(),
                    });
                }
                None => {
                    crate_results.push(CrateCoverage {
                        crate_name: name.clone(),
                        total_pub_fns: 0,
                        contracted: 0,
                        coverage_pct: 0.0,
                        uncontracted: vec![],
                    });
                }
            }
        }

        let overall_pct = if total_fns > 0 {
            (total_contracted as f64 / total_fns as f64) * 100.0
        } else {
            100.0
        };

        respond(
            span,
            &ContractAuditResponse {
                crates: crate_results,
                totals: AuditTotals {
                    total_pub_fns: total_fns,
                    contracted: total_contracted,
                    coverage_pct: overall_pct,
                    uncontracted_total: total_uncontracted,
                },
            },
        )
    }

    /// Submit a contract proposal for human review.
    #[tool(
        description = "Propose a behavioral contract for a public function. Submits for human consent review."
    )]
    async fn contract_propose(
        &self,
        Parameters(ContractProposeRequest {
            crate_name,
            function,
            contract_id,
            pre,
            post,
            replicant,
        }): Parameters<ContractProposeRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("contract_propose", &self.webid);
        let rep = replicant.unwrap_or_else(|| self.webid.to_string());

        // Emit CNS span to real event sink (flows to Curator's review queue)
        emit_contract_proposed(
            &*self.event_sink,
            &rep,
            &crate_name,
            &function,
            &contract_id,
        );

        // Persist proposal as triple for Curator review
        let value = serde_json::json!({
            "replicant": rep,
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
            &contract_id,
            value,
            hkask_types::WebID::from_persona(rep.as_bytes()),
        );
        let _ = self.triple_store.insert(&triple);

        respond(
            span,
            &ContractProposeResponse {
                contract_id,
                crate_name,
                function,
                status: "proposed".to_string(),
            },
        )
    }

    /// Accept a proposed contract (human consent gate).
    #[tool(description = "Accept a proposed behavioral contract. Human consent gate per P2.")]
    async fn contract_accept(
        &self,
        Parameters(ContractAcceptRequest {
            contract_id,
            reviewer,
        }): Parameters<ContractAcceptRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("contract_accept", &self.webid);
        let rev = reviewer.unwrap_or_else(|| self.webid.to_string());

        emit_contract_accepted(&*self.event_sink, &rev, "", "", "", &contract_id);

        // Update proposal triple status
        if let Ok(mut existing) = self
            .triple_store
            .query_by_entity_attribute("cns:contract_proposal", &contract_id)
        {
            if let Some(mut triple) = existing.pop() {
                let mut value = triple.value.clone();
                value["status"] = serde_json::json!("accepted");
                value["reviewer"] = serde_json::json!(&rev);
                value["accepted_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
                triple.value = value.clone();
                let _ =
                    self.triple_store
                        .update(&triple.id, value, hkask_types::Confidence::full());
            }
        }

        respond(
            span,
            &ContractAcceptResponse {
                contract_id,
                status: "accepted".to_string(),
            },
        )
    }

    /// Reject a proposed contract with rationale.
    #[tool(description = "Reject a proposed behavioral contract with rationale.")]
    async fn contract_reject(
        &self,
        Parameters(ContractRejectRequest {
            contract_id,
            reason,
            reviewer,
        }): Parameters<ContractRejectRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("contract_reject", &self.webid);
        let rev = reviewer.unwrap_or_else(|| self.webid.to_string());

        emit_contract_rejected(&*self.event_sink, &rev, "", "", "", &contract_id, &reason);

        // Update proposal triple status
        if let Ok(mut existing) = self
            .triple_store
            .query_by_entity_attribute("cns:contract_proposal", &contract_id)
        {
            if let Some(mut triple) = existing.pop() {
                let mut value = triple.value.clone();
                value["status"] = serde_json::json!("rejected");
                value["reviewer"] = serde_json::json!(&rev);
                value["reason"] = serde_json::json!(&reason);
                value["rejected_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
                triple.value = value.clone();
                let _ =
                    self.triple_store
                        .update(&triple.id, value, hkask_types::Confidence::full());
            }
        }

        respond(
            span,
            &ContractRejectResponse {
                contract_id,
                status: "rejected".to_string(),
            },
        )
    }

    /// List proposed contracts awaiting review.
    #[tool(description = "List proposed behavioral contracts and their review status.")]
    async fn contract_list(&self) -> String {
        let span = ToolSpanGuard::new("contract_list", &self.webid);
        let proposals = match self.triple_store.query_by_entity("cns:contract_proposal") {
            Ok(p) => p,
            Err(_) => vec![],
        };

        let entries: Vec<ProposalEntry> = proposals
            .iter()
            .map(|t| ProposalEntry {
                contract_id: t.value["contract_id"].as_str().unwrap_or("?").to_string(),
                status: t.value["status"].as_str().unwrap_or("unknown").to_string(),
                function: t.value["function"].as_str().unwrap_or("?").to_string(),
                crate_name: t.value["crate"].as_str().unwrap_or("?").to_string(),
                pre: t.value["pre"].as_str().unwrap_or("").to_string(),
                post: t.value["post"].as_str().unwrap_or("").to_string(),
                replicant: t.value["replicant"].as_str().map(|s| s.to_string()),
            })
            .collect();

        respond(span, &ContractListResponse { proposals: entries })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let replicant = std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());

    let daemon_ok = match try_daemon_flow(&replicant).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(target: "hkask.mcp.spec", replicant = %replicant, error = %e, "Daemon unavailable — falling back to direct mode");
            false
        }
    };

    let daemon_client = if daemon_ok {
        Some(hkask_mcp::DaemonClient::new())
    } else {
        None
    };

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
            let store = std::sync::Arc::new(hkask_storage::SqliteSpecStore::new(Arc::clone(&conn)));
            store.init_schema().map_err(|e| anyhow::anyhow!("{}", e))?;

            // Event sink + triple store for contract proposal persistence
            let event_sink: Arc<dyn hkask_types::event::NuEventSink> =
                std::sync::Arc::new(NuEventStore::new(Arc::clone(&conn)));
            let triple_store = std::sync::Arc::new(TripleStore::new(Arc::clone(&conn)));

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
            Ok(SpecServer::new(store, ctx.webid, checker, replicant.clone(), daemon_client.clone(), event_sink, triple_store))
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

async fn try_daemon_flow(replicant: &str) -> anyhow::Result<()> {
    let client = hkask_mcp::DaemonClient::new();
    let result = hkask_mcp::verify_startup_gates(&client, replicant, "spec", &[]).await?;
    tracing::info!(target: "hkask.mcp.spec", replicant = %replicant,
        "P4 gates verified{}",
        if result.denied_tools.is_empty() { String::new() }
        else { format!(" — {} tool(s) denied: {:?}", result.denied_tools.len(), result.denied_tools) }
    );
    Ok(())
}
