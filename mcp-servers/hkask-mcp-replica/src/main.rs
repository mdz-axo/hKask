//! hKask MCP Replica — Build and compose authorial style replicas
//!
//! Tools:
//! - replica_build    — embed a corpus, create a style replica
//! - replica_compose  — generate prose in an author's style
//! - replica_mashup   — blend two authors' styles via centroid interpolation
//! - replica_compare  — measure stylistic distance between two authors
//! - replica_registry — list, inspect, and manage built replicas
//! - replica_explain  — explain centroids and style-space topology

use hkask_mcp::run_server;
use hkask_mcp::server::ToolSpanGuard;
use hkask_services::{EmbedProgress, EmbedService, InferenceContext};
use hkask_storage::{Database, EmbeddingStore};
use hkask_types::WebID;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

/// Default embedding model (DeepInfra Qwen3-Embedding-0.6B).
/// Override with `HKASK_EMBEDDING_MODEL` env var.
const DEFAULT_EMBEDDING_MODEL: &str = "Qwen/Qwen3-Embedding-0.6B";

fn embedding_model() -> String {
    std::env::var("HKASK_EMBEDDING_MODEL").unwrap_or_else(|_| DEFAULT_EMBEDDING_MODEL.to_string())
}

fn okapi_base_url() -> String {
    std::env::var("OKAPI_BASE_URL")
        .unwrap_or_else(|_| hkask_services::DEFAULT_OKAPI_BASE_URL.to_string())
}

struct ReplicaServer {
    webid: WebID,
}

fn internal_error(span: ToolSpanGuard, context: &str, e: impl std::fmt::Display) -> String {
    span.internal_error(json!({"error": format!("Failed to {context}: {e}")}))
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

#[derive(Debug, Deserialize, JsonSchema)]
struct CompareRequest {
    db_path: String,
    passphrase: String,
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
                Some(&okapi_base_url()),
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
            Ok(json) => span.ok_json(serde_json::from_str(&json).unwrap_or(json!({}))),
            Err(e) => internal_error(span, "build replica", e),
        }
    }

    #[tool(description = "Generate prose in an author's style.")]
    async fn replica_compose(&self, Parameters(params): Parameters<ComposeRequest>) -> String {
        let span = ToolSpanGuard::new("replica_compose", &self.webid);

        let run = async {
            let model = embedding_model();
            let base_url = okapi_base_url();
            let config = hkask_services::CognitionConfig {
                author: params.author.clone(),
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

            let inference_ctx = InferenceContext::from_parts(None, &model, &base_url);

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
            Ok(json) => span.ok_json(serde_json::from_str(&json).unwrap_or(json!({}))),
            Err(e) => internal_error(span, "compose prose", e),
        }
    }

    #[tool(description = "Compare all built author replicas.")]
    async fn replica_compare(&self, Parameters(params): Parameters<CompareRequest>) -> String {
        let span = ToolSpanGuard::new("replica_compare", &self.webid);

        let run = || -> Result<String, String> {
            let db =
                Database::open(&params.db_path, &params.passphrase).map_err(|e| e.to_string())?;
            let conn = db.conn_arc();
            let store = EmbeddingStore::new(conn);

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
                        let dist = hkask_services::cosine_distance(&a.vector, &b.vector);
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

        match run() {
            Ok(json) => span.ok_json(serde_json::from_str(&json).unwrap_or(json!({}))),
            Err(e) => internal_error(span, "compare authors", e),
        }
    }

    #[tool(description = "Generate prose blending two authors' styles.")]
    async fn replica_mashup(&self, Parameters(params): Parameters<MashupRequest>) -> String {
        let span = ToolSpanGuard::new("replica_mashup", &self.webid);

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
            store
                .store(&blended_ref, &blended, &model)
                .map_err(|e| e.to_string())?;

            let base_url = okapi_base_url();
            let config = hkask_services::CognitionConfig {
                author: format!("mashup:{}:{}", params.author_a, params.author_b),
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

            let inference_ctx = InferenceContext::from_parts(None, &model, &base_url);

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
            Ok(json) => span.ok_json(serde_json::from_str(&json).unwrap_or(json!({}))),
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
                        match triple_store.query_by_entity(&entity_ref) {
                            Ok(triples) => {
                                for t in &triples {
                                    let _ = triple_store.close_by_id(&t.id);
                                    triple_count += 1;
                                }
                            }
                            _ => {}
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
                "distance_threshold": "Maximum cosine distance for exemplar inclusion (default: 0.30)",
                "salience_min": "Only passages with salience >= this value are considered (default: 0.0)",
                "salience_top_k": "Limit to top K most salient matching passages"
            }
        }))
    }
}

// ── Entry Point ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_server(
        "hkask-mcp-replica",
        env!("CARGO_PKG_VERSION"),
        |ctx| Ok(ReplicaServer { webid: ctx.webid }),
        vec![],
    )
    .await
}
