//! hKask MCP Replica — Build and compose authorial style replicas
//!
//! Tools:
//! - replica_build    — embed a corpus, create a style replica
//! - replica_compose  — generate prose in an author's style
//! - replica_mashup   — blend two authors' styles via centroid interpolation
//! - replica_compare  — measure stylistic distance between two authors
//! - replica_registry — list, inspect, and manage built replicas
//! - replica_explain  — explain centroids and style-space topology

use hkask_inference::EmbeddingRouter;
use hkask_mcp::run_server;
use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_services::{
    EmbedProgress, EmbedService, HkaskSettings, InferenceContext, cosine_distance,
};
use hkask_storage::{Database, EmbeddingStore};
use hkask_types::time::now_rfc3339;
use hkask_types::{McpErrorKind, WebID};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Default embedding model (DeepInfra Qwen3-Embedding-0.6B).
/// Override via settings.json or HKASK_EMBEDDING_MODEL env var.
fn embedding_model() -> String {
    HkaskSettings::load().embedding_model()
}

/// Default generation model for prose composition.
/// Override via settings.json or HKASK_REPLICA_MODEL env var.
fn generation_model() -> String {
    HkaskSettings::load().generation_model()
}

fn inference_config() -> hkask_inference::InferenceConfig {
    hkask_inference::InferenceConfig::from_env()
}

struct ReplicaServer {
    webid: WebID,
    /// Replicant identity serving this MCP server (for narrative memory)
    replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    daemon: Option<hkask_mcp::DaemonClient>,
}

fn internal_error(span: ToolSpanGuard, context: &str, e: impl std::fmt::Display) -> String {
    hkask_mcp::tool_internal_error(span, context, e)
}

impl ReplicaServer {
    fn new(webid: WebID, replicant: String, daemon: Option<hkask_mcp::DaemonClient>) -> Self {
        Self {
            webid,
            replicant,
            daemon,
        }
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
                "tool": tool, "input": input_summary, "outcome": outcome,
                "detail": detail, "timestamp": now_rfc3339(),
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
                        tracing::debug!(target: "cns.mcp.replica.memory", tool = %tool_name, "Experience stored via daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "cns.mcp.replica.memory", tool = %tool_name, response = ?other, "Unexpected daemon response")
                    }
                    Err(e) => {
                        tracing::warn!(target: "cns.mcp.replica.memory", tool = %tool_name, error = %e, "Failed to store experience")
                    }
                }
            });
        }
    }
}

// ── Request/Response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct BuildRequest {
    config_path: String,
    db_path: String,
    passphrase: String,
}

#[derive(Debug, Serialize)]
struct BuildResult {
    author: String,
    purged: usize,
    total_passages: usize,
    centroid_ref: String,
    centroid_stored: bool,
    passage_count: usize,
    budget: usize,
    tagged_passages: usize,
    triples_stored: usize,
    embedding_only: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ComposeRequest {
    prompt: String,
    author: String,
    db_path: String,
    passphrase: String,
    #[serde(default = "default_false")]
    no_validate: bool,
}

fn default_false() -> bool {
    false
}

#[derive(Debug, Serialize)]
struct ComposeResult {
    prose: String,
    exemplar_count: usize,
    centroid_distance: Option<f64>,
    style_passed: Option<bool>,
}

fn default_compare_mode() -> String {
    "per-dimension".to_string()
}

fn qualitative_label(distance: f64) -> String {
    if distance < 0.20 {
        "Excellent".to_string()
    } else if distance < 0.40 {
        "Good".to_string()
    } else if distance < 0.60 {
        "Fair".to_string()
    } else {
        "Needs Work".to_string()
    }
}

fn is_centroid_entity(entity_ref: &str) -> bool {
    // Match composite centroid (style:persona:centroid) and
    // per-dimension centroids (style:persona:dimension-centroid)
    if let Some(last) = entity_ref.rsplit(':').next() {
        last == "centroid" || last.ends_with("-centroid")
    } else {
        false
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CompareRequest {
    db_path: String,
    passphrase: String,
    /// Scope comparison to a specific persona's centroids (e.g., "gentle-lovelace").
    /// When set, only centroids under style:{persona}: are considered.
    #[serde(default)]
    persona: Option<String>,
    /// Document content to embed and compare against centroids.
    /// When set, compares document embedding to persona centroids instead
    /// of doing pairwise author comparison.
    #[serde(default)]
    document_content: Option<String>,
    /// Comparison mode: "per-dimension" returns scores for each dimension
    /// centroid + composite; "composite" returns only the weighted composite.
    #[serde(default = "default_compare_mode")]
    compare_mode: String,
}

#[derive(Debug, Serialize)]
struct DimensionScore {
    /// Dimension name: "Gentle", "Schriver", "Hopper", "Lovelace".
    dimension_name: String,
    /// Entity ref in the embedding store (e.g., "style:gentle-lovelace:gentle-centroid").
    centroid_ref: String,
    /// Human-readable description of what this dimension measures.
    description: String,
    /// Cosine distance from document to dimension centroid (lower = stronger alignment).
    cosine_distance: f64,
    /// Number of passages used to compute this centroid.
    passage_count: usize,
    /// Qualitative rating: "Excellent", "Good", "Fair", or "Needs Work".
    qualitative: String,
}

#[derive(Debug, Serialize)]
struct PersonaCompareResult {
    /// Persona name (e.g., "gentle-lovelace").
    persona: String,
    /// Comparison mode used: "per-dimension" or "composite".
    compare_mode: String,
    /// Embedding model used for document embedding.
    embedding_model: String,
    /// Composite weighted centroid score (always present).
    composite_score: Option<DimensionScore>,
    /// Per-dimension scores (only in "per-dimension" mode).
    dimension_scores: Vec<DimensionScore>,
    /// Elapsed time for the comparison.
    elapsed_ms: u64,
}

#[derive(Debug, Serialize)]
struct AuthorInfo {
    name: String,
    centroid_ref: String,
    passage_count: usize,
}

#[derive(Debug, Serialize)]
struct CompareResult {
    authors: Vec<AuthorInfo>,
    distances: Vec<AuthorDistance>,
}

#[derive(Debug, Serialize)]
struct AuthorDistance {
    author_a: String,
    author_b: String,
    cosine_distance: f64,
    compatible: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MashupRequest {
    prompt: String,
    author_a: String,
    author_b: String,
    #[serde(default = "default_half")]
    blend: f64,
    db_path: String,
    passphrase: String,
}

fn default_half() -> f64 {
    0.5
}

#[derive(Debug, Serialize)]
struct MashupResult {
    prose: String,
    exemplar_count: usize,
    blend_ratio: f64,
    blended_centroid_ref: String,
    centroid_distance: Option<f64>,
    distance_a: f64,
    distance_b: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "lowercase")]
enum RegistryAction {
    List,
    Remove { author: String },
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RegistryRequest {
    #[serde(flatten)]
    action: RegistryAction,
    db_path: String,
    passphrase: String,
}

#[derive(Debug, Serialize)]
struct RegistryEntry {
    name: String,
    centroid_ref: String,
    passage_count: usize,
}

#[derive(Debug, Serialize)]
struct RegistryResult {
    entries: Vec<RegistryEntry>,
    message: String,
}

// ── Replica Discovery types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct DiscoverRequest {
    /// Full name of the academic author to research (e.g., "David Dunning")
    author_name: String,
    /// Discovery mode: "agentic" (fully automated) or "curated" (human-in-the-loop)
    #[serde(default = "default_curated")]
    mode: String,
    /// Maximum number of works to include in the corpus
    #[serde(default = "default_max_works")]
    max_works: u32,
    /// Whether to search for and include YouTube transcripts
    #[serde(default = "default_true")]
    include_transcripts: bool,
    /// Whether to include institutional pages and open web content
    #[serde(default = "default_true")]
    include_web: bool,
    /// Optional path to write the generated corpus.yaml
    output_path: Option<String>,
}

fn default_curated() -> String {
    "curated".to_string()
}
fn default_max_works() -> u32 {
    20
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
struct DiscoverResult {
    /// The manifest ID to execute for discovery
    manifest_id: String,
    /// Parameters forwarded to the manifest
    parameters: serde_json::Value,
    /// Human-readable summary of what will happen
    summary: String,
    /// Estimated phases
    phases: Vec<DiscoverPhase>,
}

#[derive(Debug, Serialize)]
struct DiscoverPhase {
    ordinal: u32,
    name: String,
    description: String,
    sources: Vec<String>,
}

// ── Cache Work types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct CacheWorkRequest {
    /// Work slug (used as filename: {slug}.txt)
    slug: String,
    /// Extracted markdown/text content to cache
    content: String,
    /// Cache directory path (e.g., "./.cache")
    cache_dir: String,
}

#[derive(Debug, Serialize)]
struct CacheWorkResult {
    slug: String,
    path: String,
    bytes_written: u64,
}

// ── Server implementation ───────────────────────────────────────────────────

#[tool_router(server_handler)]
impl ReplicaServer {
    #[tool(
        description = "Embed a style corpus and create an authorial replica. Downloads public domain texts, chunks them, generates embeddings, and computes a style centroid."
    )]
    async fn replica_build(&self, Parameters(params): Parameters<BuildRequest>) -> String {
        let span = ToolSpanGuard::new("replica_build", &self.webid);
        let config_path = PathBuf::from(&params.config_path);

        let run = async {
            if !config_path.exists() {
                return Err(format!("Config file not found: {}", params.config_path));
            }

            let progress = Arc::new(|p: &EmbedProgress| {
                tracing::info!(
                    target: "cns.mcp.replica",
                    phase = ?p.phase,
                    author = %p.author,
                    work = %p.current_work,
                    done = p.completed_passages,
                    total = p.total_passages,
                    "Embedding progress"
                );
            });

            let result = EmbedService::embed_corpus(
                &config_path,
                &params.db_path,
                &params.passphrase,
                None,
                Some(progress),
            )
            .await
            .map_err(|e| e.to_string())?;

            serde_json::to_string(&BuildResult {
                author: result.author,
                purged: result.purged,
                total_passages: result.total_passages,
                centroid_ref: result.centroid_ref,
                centroid_stored: result.centroid_stored,
                passage_count: result.passage_count,
                budget: result.budget,
                tagged_passages: result.tagged_passages,
                triples_stored: result.triples_stored,
                embedding_only: result.embedding_only,
            })
            .map_err(|e| e.to_string())
        };

        match run.await {
            Ok(json) => {
                let parsed: serde_json::Value = serde_json::from_str(&json).unwrap_or(json!({}));
                self.record_experience(
                    "replica_build",
                    &config_path.to_string_lossy(),
                    "success",
                    parsed.clone(),
                );
                span.ok_json(parsed)
            }
            Err(e) => internal_error(span, "build replica", e),
        }
    }

    #[tool(description = "Generate prose in an author's style.")]
    async fn replica_compose(&self, Parameters(params): Parameters<ComposeRequest>) -> String {
        let span = ToolSpanGuard::new("replica_compose", &self.webid);
        let author = params.author.clone();

        let run = async {
            let model = embedding_model();
            let gen_model = generation_model();
            let inf_cfg = inference_config();
            let config = hkask_services::CognitionConfig {
                author: params.author.clone(),
                jinja2_template: None,
                embedding: hkask_services::EmbeddingSection {
                    model: model.clone(),
                    dim: 1024,
                    centroid_entity_ref: format!("style:{}:centroid", params.author),
                    retrieval: Default::default(),
                },
                validation: hkask_services::ValidationSection {
                    centroid_distance_max: 0.25,
                },
            };

            let inference_ctx = InferenceContext::from_parts(None, &gen_model, inf_cfg);

            let request = hkask_services::ComposeRequest {
                prompt: params.prompt,
                db_path: PathBuf::from(&params.db_path),
                db_passphrase: params.passphrase,
                cognition: config,
                inference_ctx,
                no_validate: params.no_validate,
            };

            let result = hkask_services::ComposeService::compose(request)
                .await
                .map_err(|e| e.to_string())?;

            serde_json::to_string(&ComposeResult {
                prose: result.generated_prose,
                exemplar_count: result.exemplar_count,
                centroid_distance: result.validation.as_ref().map(|v| v.distance),
                style_passed: result.validation.map(|v| v.passed),
            })
            .map_err(|e| e.to_string())
        };

        match run.await {
            Ok(json) => {
                let parsed: serde_json::Value = serde_json::from_str(&json).unwrap_or(json!({}));
                self.record_experience("replica_compose", &author, "success", parsed.clone());
                span.ok_json(parsed)
            }
            Err(e) => internal_error(span, "compose prose", e),
        }
    }

    #[tool(
        description = "Compare all built author replicas, or evaluate a document against a persona's centroids."
    )]
    async fn replica_compare(&self, Parameters(params): Parameters<CompareRequest>) -> String {
        let span = ToolSpanGuard::new("replica_compare", &self.webid);
        let persona = params.persona.clone();
        let document_content = params.document_content.clone();

        let run = async {
            let db =
                Database::open(&params.db_path, &params.passphrase).map_err(|e| e.to_string())?;
            let conn = db.conn_arc();
            let store = EmbeddingStore::new(conn);

            // ── Document comparison path ──────────────────────────────
            if let Some(ref doc_text) = document_content {
                let started = Instant::now();

                // Embed the document
                let emb_model = embedding_model();
                let inf_cfg = inference_config();
                let embedder = EmbeddingRouter::new(inf_cfg);
                let vectors = embedder
                    .embed_sentences(&emb_model, &[doc_text.as_str()])
                    .await
                    .map_err(|e| format!("Failed to embed document: {e}"))?;
                let doc_vec = vectors
                    .first()
                    .ok_or_else(|| "Embedding returned empty result".to_string())?;

                // Query centroids for this persona
                let prefix = format!("style:{}:", persona.as_deref().unwrap_or(""));
                let all_refs = store.query_by_prefix(&prefix).map_err(|e| e.to_string())?;

                // Count non-centroid passages for passage_count on each centroid
                let total_passages = all_refs.iter().filter(|r| !is_centroid_entity(r)).count();

                let mut dimension_scores: Vec<DimensionScore> = Vec::new();
                let mut composite_score: Option<DimensionScore> = None;

                for entity_ref in &all_refs {
                    if !is_centroid_entity(entity_ref) {
                        continue;
                    }

                    let emb = store.get(entity_ref).map_err(|e| e.to_string())?;
                    let dist = cosine_distance(doc_vec, &emb.vector);

                    // Derive dimension name from entity_ref.
                    // "style:{persona}:{dimension}-centroid" → dimension name
                    // "style:{persona}:centroid" → composite
                    let last_segment = entity_ref.rsplit(':').next().unwrap_or(entity_ref);

                    let (dimension_name, is_composite) = if last_segment == "centroid" {
                        ("composite".to_string(), true)
                    } else if let Some(dim) = last_segment.strip_suffix("-centroid") {
                        let mut chars = dim.chars();
                        let capitalized = match chars.next() {
                            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                            None => dim.to_string(),
                        };
                        (capitalized, false)
                    } else {
                        continue;
                    };

                    let dim_lower = dimension_name.to_lowercase();
                    let dim_passage_count = all_refs
                        .iter()
                        .filter(|r| !is_centroid_entity(r) && r.to_lowercase().contains(&dim_lower))
                        .count();

                    let score = DimensionScore {
                        centroid_ref: entity_ref.clone(),
                        cosine_distance: dist,
                        qualitative: qualitative_label(dist),
                        passage_count: if is_composite {
                            total_passages
                        } else {
                            dim_passage_count
                        },
                        dimension_name: dimension_name.clone(),
                        description: String::new(),
                    };

                    if is_composite {
                        composite_score = Some(score);
                    } else {
                        dimension_scores.push(score);
                    }
                }

                let result = PersonaCompareResult {
                    persona: persona.unwrap_or_default(),
                    compare_mode: params.compare_mode.clone(),
                    embedding_model: emb_model,
                    composite_score,
                    dimension_scores: if params.compare_mode == "composite" {
                        Vec::new()
                    } else {
                        dimension_scores
                    },
                    elapsed_ms: started.elapsed().as_millis() as u64,
                };

                return serde_json::to_string(&result).map_err(|e| e.to_string());
            }

            // ── Pairwise author comparison path (backward compat) ─────
            let centroids = store.query_by_prefix("style:").map_err(|e| e.to_string())?;

            let mut author_names: Vec<String> = Vec::new();
            let mut author_info: Vec<AuthorInfo> = Vec::new();

            for entity_ref in &centroids {
                if entity_ref.ends_with(":centroid") {
                    let parts: Vec<&str> = entity_ref.split(':').collect();
                    if parts.len() >= 3 {
                        let name = parts[1].to_string();
                        if name.contains(':') {
                            continue;
                        }
                        let prefix = format!("style:{}:", name);
                        let refs = store.query_by_prefix(&prefix).map_err(|e| e.to_string())?;
                        let passage_count =
                            refs.iter().filter(|r| !r.ends_with(":centroid")).count();
                        author_names.push(name.clone());
                        author_info.push(AuthorInfo {
                            name,
                            centroid_ref: entity_ref.clone(),
                            passage_count,
                        });
                    }
                }
            }

            let mut distances: Vec<AuthorDistance> = Vec::new();
            for i in 0..author_names.len() {
                for j in (i + 1)..author_names.len() {
                    let ca = format!("style:{}:centroid", author_names[i]);
                    let cb = format!("style:{}:centroid", author_names[j]);
                    if let (Ok(a), Ok(b)) = (store.get(&ca), store.get(&cb)) {
                        let dist = cosine_distance(&a.vector, &b.vector);
                        distances.push(AuthorDistance {
                            author_a: author_names[i].clone(),
                            author_b: author_names[j].clone(),
                            cosine_distance: dist,
                            compatible: dist < 0.30,
                        });
                    }
                }
            }

            serde_json::to_string(&CompareResult {
                authors: author_info,
                distances,
            })
            .map_err(|e| e.to_string())
        };

        match run.await {
            Ok(json) => span.ok_json(serde_json::from_str(&json).unwrap_or(json!({}))),
            Err(e) => internal_error(span, "compare authors", e),
        }
    }

    #[tool(description = "Generate prose blending two authors' styles.")]
    async fn replica_mashup(&self, Parameters(params): Parameters<MashupRequest>) -> String {
        let span = ToolSpanGuard::new("replica_mashup", &self.webid);
        let author_a = params.author_a.clone();
        let author_b = params.author_b.clone();

        let run = async {
            let blend = params.blend.clamp(0.0, 1.0);
            let centroid_a_ref = format!("style:{}:centroid", params.author_a);
            let centroid_b_ref = format!("style:{}:centroid", params.author_b);
            let blended_ref = format!(
                "style:mashup:{}:{}:centroid",
                params.author_a, params.author_b
            );

            let db =
                Database::open(&params.db_path, &params.passphrase).map_err(|e| e.to_string())?;
            let conn = db.conn_arc();
            let store = EmbeddingStore::new(Arc::clone(&conn));

            let emb_a = store.get(&centroid_a_ref).map_err(|_| {
                format!(
                    "Author '{}' not found. Run replica_build first.",
                    params.author_a
                )
            })?;
            let emb_b = store.get(&centroid_b_ref).map_err(|_| {
                format!(
                    "Author '{}' not found. Run replica_build first.",
                    params.author_b
                )
            })?;

            let blended: Vec<f32> = emb_a
                .vector
                .iter()
                .zip(emb_b.vector.iter())
                .map(|(a, b)| a * (1.0 - blend as f32) + b * blend as f32)
                .collect();

            let dist_a = hkask_services::cosine_distance(&blended, &emb_a.vector);
            let dist_b = hkask_services::cosine_distance(&blended, &emb_b.vector);

            let model = embedding_model();
            let gen_model = generation_model();
            store
                .store(&blended_ref, &blended, &model)
                .map_err(|e| e.to_string())?;

            let inf_cfg = inference_config();
            let config = hkask_services::CognitionConfig {
                author: format!("mashup:{}:{}", params.author_a, params.author_b),
                jinja2_template: None,
                embedding: hkask_services::EmbeddingSection {
                    model: model.clone(),
                    dim: 1024,
                    centroid_entity_ref: blended_ref.clone(),
                    retrieval: Default::default(),
                },
                validation: hkask_services::ValidationSection {
                    centroid_distance_max: 0.25,
                },
            };

            let inference_ctx = InferenceContext::from_parts(None, &gen_model, inf_cfg);

            let request = hkask_services::ComposeRequest {
                prompt: params.prompt,
                db_path: PathBuf::from(&params.db_path),
                db_passphrase: params.passphrase,
                cognition: config,
                inference_ctx,
                no_validate: false,
            };

            let result = hkask_services::ComposeService::compose(request)
                .await
                .map_err(|e| e.to_string())?;

            serde_json::to_string(&MashupResult {
                prose: result.generated_prose,
                exemplar_count: result.exemplar_count,
                blend_ratio: blend,
                blended_centroid_ref: blended_ref,
                centroid_distance: result.validation.as_ref().map(|v| v.distance),
                distance_a: dist_a,
                distance_b: dist_b,
            })
            .map_err(|e| e.to_string())
        };

        match run.await {
            Ok(json) => {
                let parsed: serde_json::Value = serde_json::from_str(&json).unwrap_or(json!({}));
                let summary = format!("{} x {}", author_a, author_b);
                self.record_experience("replica_mashup", &summary, "success", parsed.clone());
                span.ok_json(parsed)
            }
            Err(e) => internal_error(span, "mashup styles", e),
        }
    }

    #[tool(description = "Manage the registry of built author replicas.")]
    async fn replica_registry(&self, Parameters(params): Parameters<RegistryRequest>) -> String {
        let span = ToolSpanGuard::new("replica_registry", &self.webid);

        let run = || -> Result<String, String> {
            let db =
                Database::open(&params.db_path, &params.passphrase).map_err(|e| e.to_string())?;
            let conn = db.conn_arc();
            let store = EmbeddingStore::new(conn);

            match params.action {
                RegistryAction::List => {
                    let centroids = store.query_by_prefix("style:").map_err(|e| e.to_string())?;
                    let mut entries: Vec<RegistryEntry> = Vec::new();
                    for entity_ref in &centroids {
                        if entity_ref.ends_with(":centroid") {
                            let parts: Vec<&str> = entity_ref.split(':').collect();
                            if parts.len() >= 3 {
                                let name = parts[1].to_string();
                                let prefix = format!("style:{}:", name);
                                let refs =
                                    store.query_by_prefix(&prefix).map_err(|e| e.to_string())?;
                                let passage_count =
                                    refs.iter().filter(|r| !r.ends_with(":centroid")).count();
                                entries.push(RegistryEntry {
                                    name,
                                    centroid_ref: entity_ref.clone(),
                                    passage_count,
                                });
                            }
                        }
                    }
                    serde_json::to_string(&RegistryResult {
                        message: format!("{} author replicas registered", entries.len()),
                        entries,
                    })
                    .map_err(|e| e.to_string())
                }
                RegistryAction::Remove { author } => {
                    let prefix = format!("style:{}:", author);
                    // Remove embeddings
                    let refs = store.query_by_prefix(&prefix).map_err(|e| e.to_string())?;
                    let emb_count = refs.len();
                    for entity_ref in &refs {
                        let _ = store.delete(entity_ref);
                    }
                    // Remove triples
                    let conn = db.conn_arc();
                    let triple_store = hkask_storage::TripleStore::new(Arc::clone(&conn));
                    let mut triple_count = 0usize;
                    for entity_ref in refs {
                        if let Ok(triples) = triple_store.query_by_entity(&entity_ref) {
                            for t in &triples {
                                let _ = triple_store.close_by_id(&t.id);
                                triple_count += 1;
                            }
                        }
                    }
                    serde_json::to_string(&RegistryResult {
                        message: format!(
                            "Removed {} embeddings and {} triples for author '{}'",
                            emb_count, triple_count, author
                        ),
                        entries: vec![],
                    })
                    .map_err(|e| e.to_string())
                }
            }
        };

        match run() {
            Ok(json) => span.ok_json(serde_json::from_str(&json).unwrap_or(json!({}))),
            Err(e) => internal_error(span, "manage registry", e),
        }
    }

    #[tool(description = "Explain what style centroids are and how the metadata layer works.")]
    async fn replica_explain(&self) -> String {
        let span = ToolSpanGuard::new("replica_explain", &self.webid);
        span.ok_json(json!({
            "what_is_a_centroid": "A style centroid is the average of all embedded passage vectors for an author. Each passage (50-200 words) is converted to a 1024-dimensional vector via DeepInfra's Qwen3-Embedding-0.6B. The centroid is the 'average passage' — prose that matches the author's style will have a low cosine distance to it.",
            "metadata_layer": {
                "description": "Each embedded passage is enriched with metadata triples (entity-attribute-value) stored alongside embeddings. This enables parametric retrieval beyond pure vector similarity.",
                "structural": ["author", "work_title", "work_slug", "position", "word_count", "avg_sentence_length"],
                "entities_5w1h": {
                    "who": "mentions_character — characters appearing in the passage",
                    "where": "mentions_place — locations/settings",
                    "what": "mentions_event — events/actions",
                    "why": "mentions_concept — themes/ideas",
                    "how": "exhibits_method — stylistic techniques (iceberg_theory, parataxis, etc.)"
                },
                "method_signals": ["parataxis_ratio", "adjective_density", "adverb_density", "dialogue_ratio", "passive_voice_ratio", "sentence_length_variance", "hedge_density", "intensifier_density", "concrete_noun_ratio", "sensory_word_ratio"],
                "salience": "Graph centrality score = (one_hop + two_hop/2) / 2, where one_hop is the fraction of passages sharing ≥1 entity, and two_hop is the fraction reachable within 2 hops. Higher salience = more connected in the entity graph.",
                "budget": "Triple storage is budget-gated per corpus (default: 3,750 triples per 100 pages). Passages are sorted by salience; only the top-N earn metadata triples. Others get embeddings only."
            },
            "how_blending_works": "Style blending interpolates between two centroids: blended[i] = centroid_a[i] * (1 - blend) + centroid_b[i] * blend. blend=0.0 is pure author A, 1.0 is pure B, 0.5 is equal mix. The blended vector retrieves exemplars from both corpora.",
            "style_space_topology": "Authors cluster in different regions of embedding space. Similar styles have close centroids; opposite styles are far apart. The distance matrix from replica_compare shows which authors can be blended. Hemingway (paratactic) and Woolf (hypotactic) are maximally distant — blending produces noise. Similar authors like Hemingway/Crane or Woolf/Proust would blend well.",
            "distance_thresholds": {
                "identical": "0.000 — same text",
                "very_similar": "0.000-0.030 — nearly identical style",
                "compatible": "0.030-0.300 — blendable",
                "distinct": "0.300-1.000 — clearly different",
                "opposite": "1.000-2.000 — maximally different"
            },
            "retrieval_parameters": {
                "k_min": "Minimum exemplar passages (default: 3)",
                "k_max": "Maximum exemplar passages (default: 7)",
                "distance_threshold": "Maximum cosine distance for exemplar inclusion (default: 0.50)",
                "salience_min": "Only passages with salience >= this value are considered (default: 0.0)",
                "salience_top_k": "Limit to top K most salient matching passages"
            },
            "exemplar_types": {
                "public_domain_author": {
                    "status": "Implemented",
                    "description": "Static YAML corpus config pointing to Gutenberg URLs. Works are declared in corpus.yaml, downloaded, chunked, embedded.",
                    "examples": ["hemingway", "woolf", "austen", "wilde", "twain", "grant", "christie", "eliot"]
                },
                "mashup_persona": {
                    "status": "Implemented",
                    "description": "Two-author centroid interpolation. Exemplars drawn from both source corpora via the blended centroid vector.",
                    "examples": ["jane-wilde (Austen × Wilde)", "ulysses-s-twain (Grant × Twain)", "agatha-eliot (Christie × Eliot)"]
                },
                "academic_author": {
                    "status": "Implemented",
                    "description": "Dynamic corpus discovery via CLI command. Given a name (e.g., 'David Dunning'), searches Semantic Scholar, arXiv, web (SerpAPI), and YouTube transcripts, caches content, and generates a corpus.yaml ready for replica_build. Curated by default — web and YouTube results presented for user confirmation.",
                    "cli_command": "kask style discover \"David Dunning\" [--serpapi-key KEY] [--no-curate] [--no-transcripts] [--no-web]",
                    "pipeline": [
                        "1. Semantic Scholar — free academic paper search with abstracts and open-access PDF links",
                        "2. arXiv — free preprint search with PDF links",
                        "3. Web search (SerpAPI Google) — institutional pages, interviews, faculty profiles",
                        "4. YouTube transcript search (SerpAPI) — talks, lectures, interviews with full transcripts",
                        "5. Interactive curation — user selects which web/YouTube results to include",
                        "6. Content download + cache — PDF→text, HTML→text, stored in .cache/{slug}.txt",
                        "7. Corpus YAML generation — ready for kask style embed-corpus"
                    ],
                    "build_command": "kask style embed-corpus --config <author>/corpus.yaml --db <path>",
                    "implementation": "DiscoveryService in hkask-services (CLI → service, same pattern as EmbedService). MCP tools (replica_discover, replica_cache_work) available for server-mode use. Manifest (replica-discovery.yaml) serves as specification."
                },
                "human_exemplar_principle": "All exemplar types model a named human individual whose body of work constitutes a representational corpus. The logical validity of the replica derives from the relationship between the human and their work — the corpus IS the evidence of their voice, style, and intellectual framework."
            }
        }))
    }

    #[tool(
        description = "Discover an academic author's body of work and generate a corpus.yaml for replica_build. Delegates to the replica-discovery skill manifest which orchestrates multi-source search (Semantic Scholar, arXiv, web, YouTube transcripts), content extraction, and corpus generation. Supports agentic (fully automated) and curated (human-in-the-loop) modes."
    )]
    async fn replica_discover(&self, Parameters(params): Parameters<DiscoverRequest>) -> String {
        let span = ToolSpanGuard::new("replica_discover", &self.webid);
        let author_name = params.author_name.clone();

        // Validate mode
        let mode = match params.mode.as_str() {
            "agentic" | "curated" => params.mode.clone(),
            other => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(format!(
                        "Invalid mode '{}'. Use 'agentic' or 'curated'.",
                        other
                    ))
                    .to_json_string(),
                );
            }
        };

        let author_name_lower = author_name.to_lowercase();

        // Build parameters for the manifest
        let manifest_params = serde_json::json!({
            "author_name": author_name,
            "author_name_lower": author_name_lower,
            "mode": mode,
            "max_works": params.max_works,
            "include_transcripts": params.include_transcripts,
            "include_web": params.include_web,
            "output_path": params.output_path,
        });

        // Build phase descriptions for the response
        let phases = vec![
            DiscoverPhase {
                ordinal: 1,
                name: "Name Disambiguation".into(),
                description: "Search across multiple sources to confirm author identity".into(),
                sources: vec!["web_search (deep)".into()],
            },
            DiscoverPhase {
                ordinal: 2,
                name: "Academic Paper Search".into(),
                description: "Enumerate papers via Semantic Scholar and arXiv".into(),
                sources: vec!["semantic_scholar".into(), "arxiv".into()],
            },
            DiscoverPhase {
                ordinal: 3,
                name: "Web + Institutional Content".into(),
                description: "Find faculty pages, interviews, and open web content".into(),
                sources: vec!["web_search (web)".into()],
            },
            DiscoverPhase {
                ordinal: 4,
                name: "YouTube Transcript Discovery".into(),
                description: "Search for talks, interviews, lectures on YouTube".into(),
                sources: vec![
                    "web_search (youtube.com)".into(),
                    "serpapi_transcript".into(),
                ],
            },
            DiscoverPhase {
                ordinal: 5,
                name: "Content Extraction".into(),
                description: "Extract full text from all discovered works".into(),
                sources: vec!["web_extract".into(), "docproc (PDF/OCR)".into()],
            },
            DiscoverPhase {
                ordinal: 6,
                name: "Corpus YAML Generation".into(),
                description: "Generate corpus.yaml from discovered works".into(),
                sources: vec!["minijinja template".into()],
            },
        ];

        let summary = format!(
            "Discovering corpus for '{}' in {} mode. Will search Semantic Scholar, arXiv, web{}, and generate a corpus.yaml with up to {} works.",
            params.author_name,
            mode,
            if params.include_transcripts {
                ", YouTube transcripts"
            } else {
                ""
            },
            params.max_works,
        );

        let result = DiscoverResult {
            manifest_id: "mcp/replica-discovery".into(),
            parameters: manifest_params,
            summary,
            phases,
        };

        let output = serde_json::to_value(&result)
            .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}));

        self.record_experience(
            "replica_discover",
            &params.author_name,
            "delegated_to_manifest",
            output.clone(),
        );

        span.ok_json(output)
    }

    #[tool(
        description = "Cache an extracted work's content to disk for reuse by replica_build. Writes content to {cache_dir}/{slug}.txt so the embedding pipeline can skip re-downloading."
    )]
    async fn replica_cache_work(&self, Parameters(params): Parameters<CacheWorkRequest>) -> String {
        let span = ToolSpanGuard::new("replica_cache_work", &self.webid);

        // Validate slug: alphanumeric + hyphens only, no path traversal
        if params.slug.is_empty()
            || !params
                .slug
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!(
                    "Invalid slug '{}': must be alphanumeric with hyphens/underscores only",
                    params.slug
                ))
                .to_json_string(),
            );
        }

        let cache_dir = PathBuf::from(&params.cache_dir);
        let cache_path = cache_dir.join(format!("{}.txt", params.slug));

        // Create cache directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            return span.internal_error(serde_json::json!({
                "error": format!("Failed to create cache directory '{}': {}", cache_dir.display(), e),
            }));
        }

        let bytes = params.content.as_bytes();
        match std::fs::write(&cache_path, bytes) {
            Ok(()) => {
                let result = CacheWorkResult {
                    slug: params.slug.clone(),
                    path: cache_path.to_string_lossy().to_string(),
                    bytes_written: bytes.len() as u64,
                };
                let output = serde_json::to_value(&result)
                    .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}));
                self.record_experience(
                    "replica_cache_work",
                    &params.slug,
                    "success",
                    output.clone(),
                );
                span.ok_json(output)
            }
            Err(e) => span.internal_error(serde_json::json!({
                "error": format!("Failed to write cache file '{}': {}", cache_path.display(), e),
            })),
        }
    }
}

// ── Entry Point ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), hkask_mcp::McpError> {
    dotenvy::dotenv().ok();
    let replicant = std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());

    let daemon_ok = match try_daemon_flow(&replicant).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(target: "cns.mcp.replica", replicant = %replicant, error = %e, "Daemon unavailable — falling back to direct mode");
            false
        }
    };

    let daemon_client = if daemon_ok {
        Some(hkask_mcp::DaemonClient::new())
    } else {
        None
    };

    run_server(
        "hkask-mcp-replica",
        env!("CARGO_PKG_VERSION"),
        |ctx| {
            Ok(ReplicaServer::new(
                ctx.webid,
                replicant.clone(),
                daemon_client.clone(),
            ))
        },
        vec![
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_EMBEDDING_MODEL",
                "Embedding model for corpus vectorization (default: Qwen/Qwen3-Embedding-0.6B)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_REPLICA_MODEL",
                "Generation model for prose composition (default: deepseek-v4-flash:cloud)",
            ),
        ],
    )
    .await
}

async fn try_daemon_flow(replicant: &str) -> anyhow::Result<()> {
    let client = hkask_mcp::DaemonClient::new();
    let result = hkask_mcp::verify_startup_gates(&client, replicant, "replica", &[]).await?;
    tracing::info!(target: "cns.mcp.replica", replicant = %replicant,
        "P4 gates verified{}",
        if result.denied_tools.is_empty() { String::new() }
        else { format!(" — {} tool(s) denied: {:?}", result.denied_tools.len(), result.denied_tools) }
    );
    Ok(())
}
