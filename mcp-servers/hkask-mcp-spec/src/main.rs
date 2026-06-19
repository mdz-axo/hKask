//! hKask MCP Spec — Thin MCP wrapper around SpecService + spec_ops.
//!
//! All business logic lives in:
//! - `hkask_storage::spec_ops` — pure algorithms (no I/O)
//! - `hkask_services::SpecService` — orchestrated operations (store, CNS, contracts)
//!
//! This server handles only: OCAP gating, CNS spanning, experience recording,
//! embedding-based comparison (Gentle Lovelace), and replica rewriting.
//!
//! 12 tools: capture, decompose, writing-quality, graph-query, graph-coherence,
//! replica-rewrite, contract-audit, contract-propose, contract-accept,
//! contract-reject, contract-list, test-run.

pub mod types;

use hkask_mcp::server::{McpToolError, ServerContext, ToolSpanGuard};
use hkask_mcp::validate_field;

use hkask_inference::{EmbeddingRouter, InferenceConfig};
use hkask_services::{
    ComposeRequest, ComposeService, EmbeddingSection, HkaskSettings, InferenceContext,
    SpecCaptureRequest, SpecService, cosine_distance,
};
use hkask_storage::spec_ops::{
    assess_writing_quality_heuristic, build_centroid_ref, build_rewrite_prompt,
    build_spec_document_text, collect_goal_and_criteria_texts, compute_embedding_quality,
    extract_ocap_boundaries,
};
use hkask_storage::spec_types::{Spec, SpecId};
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

/// Spec MCP server — thin wrapper around SpecService.
///
/// Provides mechanism (capture, decompose, query, assess), not governance.
/// Curation decisions (Accept/Revise/Reject) are external — made by the
/// Curator agent or human, never by a tool call.
pub struct SpecServer {
    store: Arc<dyn SpecStore + Send + Sync>,
    capability_checker: Arc<CapabilityChecker>,
    webid: WebID,
    replicant: String,
    daemon: Option<hkask_mcp::DaemonClient>,
    event_sink: Arc<dyn hkask_types::event::NuEventSink>,
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

    // ── Thin store wrappers ────────────────────────────────────────

    fn load_spec(&self, spec_id: &str) -> Result<Spec, (McpErrorKind, String)> {
        let parsed = SpecId::from_string(spec_id).unwrap_or_default();
        self.store.load(parsed).map_err(|_| {
            (
                McpErrorKind::NotFound,
                McpToolError::not_found(format!("Spec not found: {}", spec_id)).to_json_string(),
            )
        })
    }

    fn load_all_specs_val(&self) -> Result<Vec<Spec>, serde_json::Value> {
        self.store
            .list_all()
            .map_err(|e| serde_json::json!({"error": format!("Failed to load specs: {}", e)}))
    }

    fn save_spec(&self, spec: &Spec) -> Result<(), serde_json::Value> {
        self.store
            .save(spec)
            .map_err(|e| serde_json::json!({"error": format!("Failed to persist spec: {}", e)}))
    }

    // ── Embedding-based comparison (Gentle Lovelace) ──────────────

    /// Compare spec text against Gentle Lovelace persona centroids.
    /// Uses `spec_ops` for text building, then does embedding I/O locally.
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

        let doc_text = build_spec_document_text(spec);

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

        let prefix = format!("style:{}:", persona);
        let all_refs = store.query_by_prefix(&prefix).map_err(|e| e.to_string())?;

        let mut dimension_scores: Vec<DimensionScore> = Vec::new();

        for entity_ref in &all_refs {
            let last_segment = entity_ref.rsplit(':').next().unwrap_or(entity_ref);
            if !last_segment.ends_with("-centroid") && last_segment != "centroid" {
                continue;
            }

            let emb = store.get(entity_ref).map_err(|e| e.to_string())?;
            let dist = cosine_distance(doc_vec, &emb.vector);

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

    // ── Experience recording ───────────────────────────────────────

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
                        tracing::debug!(target: "cns.mcp.spec.memory", tool = %tool_name, "Experience stored via daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "cns.mcp.spec.memory", tool = %tool_name, response = ?other, "Unexpected daemon response")
                    }
                    Err(e) => {
                        tracing::warn!(target: "cns.mcp.spec.memory", tool = %tool_name, error = %e, "Failed to store experience")
                    }
                }
            });
        }
    }
}

/// Serialize response and convert to ok_json.
fn respond<T: serde::Serialize>(span: ToolSpanGuard, resp: &T) -> String {
    match serde_json::to_value(resp) {
        Ok(val) => span.ok_json(val),
        Err(e) => {
            span.internal_error(serde_json::json!({"error": format!("serialization failed: {e}")}))
        }
    }
}

// ── Tool Handlers ────────────────────────────────────────────

#[tool_router(server_handler)]
impl SpecServer {
    /// MDS §3 tool 1: Capture a goal as a binding specification requirement.
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

        let ocap_boundaries = extract_ocap_boundaries(context.as_deref());

        // Delegate to SpecService for the core logic
        let req = SpecCaptureRequest {
            name_or_description: description.clone(),
            category: None,
            domain: None,
            criteria: None,
            context: context.clone(),
        };

        match SpecService::capture_to_store(&*self.store, req) {
            Ok(resp) => {
                self.record_experience(
                    "spec_goal_capture",
                    &description,
                    "captured",
                    serde_json::json!({"goal_id": resp.spec_id, "category": resp.category}),
                );

                // Rebuild criteria from the stored spec for the response
                let spec = self.load_spec(&resp.spec_id);
                let requirements: Vec<String> = match spec {
                    Ok(s) => s
                        .goals
                        .iter()
                        .flat_map(|g| g.criteria.iter().map(|c| c.description.clone()))
                        .collect(),
                    Err(_) => vec![],
                };

                respond(
                    span,
                    &GoalCaptureResponse {
                        goal_id: resp.spec_id,
                        requirements,
                        ocap_boundaries,
                    },
                )
            }
            Err(e) => span.internal_error(serde_json::json!({"error": e.to_string()})),
        }
    }

    /// MDS §3 tool 2: Decompose a specification goal into ordered sub-goals.
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

        match SpecService::decompose(&*self.store, &goal_id) {
            Ok((sub_goals, dependencies)) => {
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
                        dependencies: dependencies
                            .into_iter()
                            .map(|d| DependencyEdgeDto {
                                from: d.from,
                                to: d.to,
                            })
                            .collect(),
                    },
                )
            }
            Err(e) => span.internal_error(serde_json::json!({"error": e.to_string()})),
        }
    }

    /// MDS §3 tool 3: Assess a specification's writing quality.
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

        // Heuristic check (always computed, from spec_ops)
        let heuristic = assess_writing_quality_heuristic(&spec);

        // Embedding-based comparison (when replica + DB available)
        let dimension_scores = match (
            replica_persona.as_deref(),
            db_path.as_deref(),
            db_passphrase.as_deref(),
        ) {
            (Some(persona), Some(path), Some(passphrase)) => {
                match self
                    .compare_against_replica(&spec, persona, path, passphrase)
                    .await
                {
                    Ok(scores) => Some(scores),
                    Err(e) => {
                        tracing::warn!(target: "cns.mcp.spec", persona = %persona, error = %e, "Replica comparison failed");
                        None
                    }
                }
            }
            _ => None,
        };

        // Compute pass/fail from embedding scores, or fall back to heuristic
        let (dimensions_passing, meets_standard, weakest_dimension, rewrite_prompt) =
            match &dimension_scores {
                Some(scores) if !scores.is_empty() => {
                    let pairs: Vec<(String, f64)> = scores
                        .iter()
                        .map(|s| (s.dimension.clone(), s.cosine_distance))
                        .collect();
                    let (goals, criteria) = collect_goal_and_criteria_texts(&spec);
                    let result = compute_embedding_quality(&pairs, &spec.name, &goals, &criteria);
                    (
                        result.dimensions_passing,
                        result.meets_standard,
                        result.weakest_dimension,
                        result.rewrite_prompt,
                    )
                }
                _ => (
                    heuristic.passes(),
                    heuristic.meets_publication_standard(),
                    None,
                    None,
                ),
            };

        respond(
            span,
            &WritingQualityResponse {
                dimensions_passing,
                meets_publication_standard: meets_standard,
                replica_persona,
                dimension_scores,
                weakest_dimension,
                rewrite_prompt,
            },
        )
    }

    /// MDS §3 tool 4: Query the specification graph.
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

        match SpecService::graph_query(&*self.store, &query, max_depth) {
            Ok(result) => {
                self.record_experience(
                    "spec_graph_query",
                    &query,
                    "success",
                    serde_json::json!({"nodes": result.nodes.len(), "edges": result.edges.len(), "paths": result.paths.len()}),
                );

                respond(
                    span,
                    &GraphQueryResponse {
                        nodes: result
                            .nodes
                            .into_iter()
                            .map(|n| GraphNodeDto {
                                id: n.id,
                                label: n.label,
                                category: n.category,
                            })
                            .collect(),
                        edges: result
                            .edges
                            .into_iter()
                            .map(|e| GraphEdgeDto {
                                from: e.from,
                                to: e.to,
                                relation: e.relation,
                            })
                            .collect(),
                        paths: result
                            .paths
                            .into_iter()
                            .map(|p| GraphPathDto {
                                nodes: p.nodes,
                                length: p.length,
                            })
                            .collect(),
                    },
                )
            }
            Err(e) => span.internal_error(serde_json::json!({"error": e.to_string()})),
        }
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

        match SpecService::graph_coherence(&*self.store, 0.7) {
            Ok(result) => respond(
                span,
                &GraphCoherenceResponse {
                    coherence_score: result.coherence_score,
                    violations: result.violations,
                    suggestions: result.suggestions,
                },
            ),
            Err(e) => span.internal_error(serde_json::json!({"error": e.to_string()})),
        }
    }

    /// Tool 6: Rewrite a passage using the Gentle Lovelace replica persona.
    #[tool(
        description = "Rewrite a passage or document using the Gentle Lovelace replica. Optimizes prose for a target quality dimension using exemplar retrieval and centroid-guided generation."
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

        // Build prompt and centroid ref using spec_ops
        let prompt = build_rewrite_prompt(&dimension, &passage);
        let centroid_ref = build_centroid_ref(&dimension);

        let run = async {
            let settings = HkaskSettings::load();
            let gen_model = settings.generation_model();
            let emb_model = settings.embedding_model();

            let inf_cfg = InferenceConfig::from_env();
            let inference_ctx = InferenceContext::from_parts(None, &gen_model, inf_cfg);

            let config = hkask_services::CognitionConfig {
                author: "gentle-lovelace".to_string(),
                jinja2_template: None,
                embedding: EmbeddingSection {
                    model: emb_model.clone(),
                    dim: 1024,
                    centroid_entity_ref: centroid_ref,
                    retrieval: hkask_services::RetrievalSection::default(),
                },
                validation: hkask_services::ValidationSection {
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

    /// Tool 7: Discover uncontracted public functions in a crate.
    #[tool(
        description = "Discover uncontracted public functions in a crate. Returns coverage percentages and lists of functions lacking REQ contracts."
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

        match SpecService::contract_audit(crate_name.as_deref(), &root) {
            Ok(audits) => {
                let mut total_fns = 0usize;
                let mut total_contracted = 0usize;
                let mut total_uncontracted = 0usize;
                let crate_results: Vec<CrateCoverage> = audits
                    .into_iter()
                    .map(|a| {
                        total_fns += a.total_pub_fns;
                        total_contracted += a.contracted;
                        total_uncontracted += a.uncontracted.len();
                        CrateCoverage {
                            crate_name: a.crate_name,
                            total_pub_fns: a.total_pub_fns,
                            contracted: a.contracted,
                            coverage_pct: a.coverage_pct,
                            uncontracted: a
                                .uncontracted
                                .iter()
                                .map(|f| UncontractedFn {
                                    function_name: f.function_name.clone(),
                                    file: f.file.clone(),
                                    line: f.line,
                                })
                                .collect(),
                        }
                    })
                    .collect();

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
            Err(e) => span.internal_error(serde_json::json!({"error": e.to_string()})),
        }
    }

    /// Tool 8: Submit a contract proposal for human review.
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

        match SpecService::contract_propose(
            &*self.event_sink,
            &self.triple_store,
            &rep,
            &crate_name,
            &function,
            &contract_id,
            &pre,
            &post,
        ) {
            Ok(()) => respond(
                span,
                &ContractProposeResponse {
                    contract_id,
                    crate_name,
                    function,
                    status: "proposed".to_string(),
                },
            ),
            Err(e) => span.internal_error(serde_json::json!({"error": e.to_string()})),
        }
    }

    /// Tool 9: Accept a proposed contract (human consent gate).
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

        match SpecService::contract_accept(
            &*self.event_sink,
            &self.triple_store,
            &rev,
            &contract_id,
        ) {
            Ok(()) => respond(
                span,
                &ContractAcceptResponse {
                    contract_id,
                    status: "accepted".to_string(),
                },
            ),
            Err(e) => span.internal_error(serde_json::json!({"error": e.to_string()})),
        }
    }

    /// Tool 10: Reject a proposed contract with rationale.
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

        match SpecService::contract_reject(
            &*self.event_sink,
            &self.triple_store,
            &rev,
            &contract_id,
            &reason,
        ) {
            Ok(()) => respond(
                span,
                &ContractRejectResponse {
                    contract_id,
                    status: "rejected".to_string(),
                },
            ),
            Err(e) => span.internal_error(serde_json::json!({"error": e.to_string()})),
        }
    }

    /// Tool 11: List proposed contracts awaiting review.
    #[tool(description = "List proposed behavioral contracts and their review status.")]
    async fn contract_list(&self) -> String {
        let span = ToolSpanGuard::new("contract_list", &self.webid);

        match SpecService::contract_list(&self.triple_store) {
            Ok(entries) => {
                let proposals: Vec<ProposalEntry> = entries
                    .into_iter()
                    .map(
                        |(contract_id, status, function, crate_name, pre, post, replicant)| {
                            ProposalEntry {
                                contract_id,
                                status,
                                function,
                                crate_name,
                                pre,
                                post,
                                replicant,
                            }
                        },
                    )
                    .collect();
                respond(span, &ContractListResponse { proposals })
            }
            Err(e) => span.internal_error(serde_json::json!({"error": e.to_string()})),
        }
    }

    /// Tool 12: Run contract tests on a crate and report REQ-tagged violations.
    #[tool(description = "Run cargo test on a crate and report REQ-tagged contract violations.")]
    async fn test_run(
        &self,
        Parameters(TestRunRequest {
            crate_name,
            workspace_root,
        }): Parameters<TestRunRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("test_run", &self.webid);

        let root = workspace_root.unwrap_or_else(|| {
            std::env::var("HKASK_WORKSPACE_ROOT").unwrap_or_else(|_| {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string())
            })
        });

        match SpecService::test_run(&crate_name, &root) {
            Ok(result) => {
                SpecService::emit_test_violations(&*self.event_sink, &result.violations);

                respond(
                    span,
                    &TestRunResponse {
                        crate_name: result.crate_name,
                        total_tests: result.total_tests,
                        passed: result.passed,
                        failed: result.failed,
                        violations: result
                            .violations
                            .iter()
                            .map(|v| TestViolation {
                                test_name: v.test_name.clone(),
                                contract_id: v.contract_id.clone(),
                                failure_reason: v.failure_reason.clone(),
                            })
                            .collect(),
                        pass: result.failed == 0,
                    },
                )
            }
            Err(e) => span.internal_error(serde_json::json!({"error": e.to_string()})),
        }
    }
}

// ── Main ──────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), hkask_mcp::McpError> {
    dotenvy::dotenv().ok();
    let replicant = std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());

    let daemon_ok = match try_daemon_flow(&replicant).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(target: "cns.mcp.spec", replicant = %replicant, error = %e, "Daemon unavailable — falling back to direct mode");
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
            Ok((|| -> anyhow::Result<SpecServer> {
                let conn = match ctx.credentials.get("HKASK_SPEC_DB_PATH") {
                    Some(path) => {
                        let passphrase = ctx
                            .credentials
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
                            target: "cns.mcp.spec",
                            "No persistent DB — spec store in-memory (set HKASK_SPEC_DB_PATH + HKASK_DB_PASSPHRASE for persistence)"
                        );
                        let conn = rusqlite::Connection::open_in_memory()?;
                        std::sync::Arc::new(std::sync::Mutex::new(conn))
                    }
                };
                let store =
                    std::sync::Arc::new(hkask_storage::SqliteSpecStore::new(Arc::clone(&conn)));
                store
                    .init_schema()
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                let event_sink: Arc<dyn hkask_types::event::NuEventSink> =
                    std::sync::Arc::new(NuEventStore::new(Arc::clone(&conn)));
                let triple_store = std::sync::Arc::new(TripleStore::new(Arc::clone(&conn)));

                let secret_hex = ctx
                    .credentials
                    .get("HKASK_OCAP_SECRET")
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "HKASK_OCAP_SECRET is required for spec capability verification"
                        )
                    })?;
                let secret = hex::decode(secret_hex)
                    .map_err(|e| anyhow::anyhow!("HKASK_OCAP_SECRET must be hex-encoded: {e}"))?;
                let checker = CapabilityChecker::new(&secret);
                Ok(SpecServer::new(
                    store,
                    ctx.webid,
                    checker,
                    replicant.clone(),
                    daemon_client.clone(),
                    event_sink,
                    triple_store,
                ))
            })()?)
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
    tracing::info!(target: "cns.mcp.spec", replicant = %replicant,
        "P4 gates verified{}",
        if result.denied_tools.is_empty() {
            String::new()
        } else {
            format!(
                " — {} tool(s) denied: {:?}",
                result.denied_tools.len(),
                result.denied_tools
            )
        }
    );
    Ok(())
}
