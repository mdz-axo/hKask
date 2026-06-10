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
use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_services::{EmbedProgress, EmbedService, InferenceContext};
use hkask_storage::{Database, EmbeddingStore};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
struct ReplicaServer;

// ── Tool 1: Build ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct BuildRequest {
    config_path: String,
    db_path: String,
    passphrase: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct BuildResponse {
    author: String,
    purged: usize,
    total_passages: usize,
    centroid_ref: String,
    centroid_stored: bool,
    passage_count: usize,
}

#[tool(
    description = "Embed a style corpus and create an authorial replica. Downloads public domain texts, chunks them, generates embeddings via Okapi, and computes a style centroid."
)]
async fn replica_build(
    _span: ToolSpanGuard,
    Parameters(params): Parameters<BuildRequest>,
) -> Result<BuildResponse, McpToolError> {
    let config_path = PathBuf::from(&params.config_path);

    if !config_path.exists() {
        return Err(McpToolError::invalid_argument(&format!(
            "Config file not found: {}",
            params.config_path
        )));
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
        None,
        None,
        Some(progress),
    )
    .await
    .map_err(|e| McpToolError::internal(e.to_string()))?;

    Ok(BuildResponse {
        author: result.author,
        purged: result.purged,
        total_passages: result.total_passages,
        centroid_ref: result.centroid_ref,
        centroid_stored: result.centroid_stored,
        passage_count: result.passage_count,
    })
}

// ── Tool 2: Compose ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct ComposeToolRequest {
    prompt: String,
    author: String,
    db_path: String,
    passphrase: String,
    #[serde(default = "default_no_validate")]
    no_validate: bool,
}

fn default_no_validate() -> bool {
    false
}

#[derive(Debug, Serialize, JsonSchema)]
struct ComposeToolResponse {
    prose: String,
    exemplar_count: usize,
    centroid_distance: Option<f64>,
    style_passed: Option<bool>,
}

#[tool(
    description = "Generate prose in an author's style. Retrieves exemplar passages from the embedded corpus, assembles a style-specific system prompt, generates prose via inference, and validates against the author's style centroid."
)]
async fn replica_compose(
    _span: ToolSpanGuard,
    Parameters(params): Parameters<ComposeToolRequest>,
) -> Result<ComposeToolResponse, McpToolError> {
    let config = hkask_services::CognitionConfig {
        author: params.author.clone(),
        embedding: hkask_services::EmbeddingSection {
            model: "qwen3-embedding:0.6b".to_string(),
            dim: 1024,
            centroid_entity_ref: format!("style:{}:centroid", params.author),
            retrieval: Default::default(),
        },
        validation: hkask_services::ValidationSection {
            centroid_distance_max: 0.25,
        },
    };

    let inference_ctx =
        InferenceContext::from_parts(None, "qwen3-embedding:0.6b", "http://127.0.0.1:11434");

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
        .map_err(|e| McpToolError::internal(e.to_string()))?;

    Ok(ComposeToolResponse {
        prose: result.generated_prose,
        exemplar_count: result.exemplar_count,
        centroid_distance: result.validation.as_ref().map(|v| v.distance),
        style_passed: result.validation.map(|v| v.passed),
    })
}

// ── Tool 3: Compare ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct CompareToolRequest {
    db_path: String,
    passphrase: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AuthorInfo {
    name: String,
    centroid_ref: String,
    passage_count: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
struct CompareToolResponse {
    authors: Vec<AuthorInfo>,
    distances: Vec<AuthorDistance>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AuthorDistance {
    author_a: String,
    author_b: String,
    cosine_distance: f64,
    compatible: bool,
}

#[tool(
    description = "Compare all built author replicas in a database. Returns each author's stats and a pairwise distance matrix showing stylistic similarity. Compatible pairs (distance < 0.30) can be blended via replica_mashup."
)]
async fn replica_compare(
    _span: ToolSpanGuard,
    Parameters(params): Parameters<CompareToolRequest>,
) -> Result<CompareToolResponse, McpToolError> {
    let db = Database::open(&params.db_path, &params.passphrase)
        .map_err(|e| McpToolError::internal(e.to_string()))?;
    let conn = db.conn_arc();
    let store = EmbeddingStore::new(conn);

    let centroids = store
        .query_by_prefix("style:")
        .map_err(|e| McpToolError::internal(e.to_string()))?;

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
                let refs = store
                    .query_by_prefix(&prefix)
                    .map_err(|e| McpToolError::internal(e.to_string()))?;
                let passage_count = refs.iter().filter(|r| !r.ends_with(":centroid")).count();
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
            let centroid_a_ref = format!("style:{}:centroid", author_names[i]);
            let centroid_b_ref = format!("style:{}:centroid", author_names[j]);
            let emb_a = store.get(&centroid_a_ref);
            let emb_b = store.get(&centroid_b_ref);
            if let (Ok(a), Ok(b)) = (emb_a, emb_b) {
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

    Ok(CompareToolResponse {
        authors: author_info,
        distances,
    })
}

// ── Tool 4: Mashup ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
struct MashupToolRequest {
    prompt: String,
    author_a: String,
    author_b: String,
    #[serde(default = "default_blend")]
    blend: f64,
    db_path: String,
    passphrase: String,
}

fn default_blend() -> f64 {
    0.5
}

#[derive(Debug, Serialize, JsonSchema)]
struct MashupToolResponse {
    prose: String,
    exemplar_count: usize,
    blend_ratio: f64,
    blended_centroid_ref: String,
    centroid_distance: Option<f64>,
    distance_a: f64,
    distance_b: f64,
}

#[tool(
    description = "Generate prose blending two authors' styles. Interpolates between the two centroids to create a blended style vector, retrieves exemplar passages from both corpora, and generates prose. blend=0.0 is pure author_a, blend=1.0 is pure author_b, 0.5 is equal blend."
)]
async fn replica_mashup(
    _span: ToolSpanGuard,
    Parameters(params): Parameters<MashupToolRequest>,
) -> Result<MashupToolResponse, McpToolError> {
    let blend = params.blend.clamp(0.0, 1.0);
    let centroid_a_ref = format!("style:{}:centroid", params.author_a);
    let centroid_b_ref = format!("style:{}:centroid", params.author_b);
    let blended_ref = format!(
        "style:mashup:{}:{}:centroid",
        params.author_a, params.author_b
    );

    let db = Database::open(&params.db_path, &params.passphrase)
        .map_err(|e| McpToolError::internal(e.to_string()))?;
    let conn = db.conn_arc();
    let store = EmbeddingStore::new(Arc::clone(&conn));

    let emb_a = store.get(&centroid_a_ref).map_err(|_| {
        McpToolError::invalid_argument(&format!(
            "Author '{}' not found. Run replica_build first.",
            params.author_a
        ))
    })?;
    let emb_b = store.get(&centroid_b_ref).map_err(|_| {
        McpToolError::invalid_argument(&format!(
            "Author '{}' not found. Run replica_build first.",
            params.author_b
        ))
    })?;

    let blended: Vec<f32> = emb_a
        .vector
        .iter()
        .zip(emb_b.vector.iter())
        .map(|(a, b)| a * (1.0 - blend as f32) + b * blend as f32)
        .collect();

    let dist_a = hkask_services::cosine_distance(&blended, &emb_a.vector);
    let dist_b = hkask_services::cosine_distance(&blended, &emb_b.vector);

    store
        .store(&blended_ref, &blended, "qwen3-embedding:0.6b")
        .map_err(|e| McpToolError::internal(e.to_string()))?;

    let config = hkask_services::CognitionConfig {
        author: format!("mashup:{}:{}", params.author_a, params.author_b),
        embedding: hkask_services::EmbeddingSection {
            model: "qwen3-embedding:0.6b".to_string(),
            dim: 1024,
            centroid_entity_ref: blended_ref.clone(),
            retrieval: Default::default(),
        },
        validation: hkask_services::ValidationSection {
            centroid_distance_max: 0.25,
        },
    };

    let inference_ctx =
        InferenceContext::from_parts(None, "qwen3-embedding:0.6b", "http://127.0.0.1:11434");

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
        .map_err(|e| McpToolError::internal(e.to_string()))?;

    Ok(MashupToolResponse {
        prose: result.generated_prose,
        exemplar_count: result.exemplar_count,
        blend_ratio: blend,
        blended_centroid_ref: blended_ref,
        centroid_distance: result.validation.as_ref().map(|v| v.distance),
        distance_a: dist_a,
        distance_b: dist_b,
    })
}

// ── Tool 5: Registry ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "lowercase")]
enum RegistryAction {
    List,
    Remove { author: String },
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RegistryToolRequest {
    #[serde(flatten)]
    action: RegistryAction,
    db_path: String,
    passphrase: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct RegistryEntry {
    name: String,
    centroid_ref: String,
    passage_count: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
struct RegistryToolResponse {
    entries: Vec<RegistryEntry>,
    message: String,
}

#[tool(
    description = "Manage the registry of built author replicas. Use action=list to see all authors with stats. Use action=remove with author=name to delete an author's embeddings and centroid."
)]
async fn replica_registry(
    _span: ToolSpanGuard,
    Parameters(params): Parameters<RegistryToolRequest>,
) -> Result<RegistryToolResponse, McpToolError> {
    let db = Database::open(&params.db_path, &params.passphrase)
        .map_err(|e| McpToolError::internal(e.to_string()))?;
    let conn = db.conn_arc();
    let store = EmbeddingStore::new(conn);

    match params.action {
        RegistryAction::List => {
            let centroids = store
                .query_by_prefix("style:")
                .map_err(|e| McpToolError::internal(e.to_string()))?;
            let mut entries: Vec<RegistryEntry> = Vec::new();
            for entity_ref in &centroids {
                if entity_ref.ends_with(":centroid") {
                    let parts: Vec<&str> = entity_ref.split(':').collect();
                    if parts.len() >= 3 {
                        let name = parts[1].to_string();
                        let prefix = format!("style:{}:", name);
                        let refs = store
                            .query_by_prefix(&prefix)
                            .map_err(|e| McpToolError::internal(e.to_string()))?;
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
            Ok(RegistryToolResponse {
                message: format!("{} author replicas registered", entries.len()),
                entries,
            })
        }
        RegistryAction::Remove { author } => {
            let prefix = format!("style:{}:", author);
            let refs = store
                .query_by_prefix(&prefix)
                .map_err(|e| McpToolError::internal(e.to_string()))?;
            let count = refs.len();
            for entity_ref in refs {
                let _ = store.delete(&entity_ref);
            }
            Ok(RegistryToolResponse {
                message: format!("Removed {} embeddings for author '{}'", count, author),
                entries: vec![],
            })
        }
    }
}

// ── Tool 6: Explain ────────────────────────────────────────────────────────

#[derive(Debug, Serialize, JsonSchema)]
struct ExplainToolResponse {
    what_is_a_centroid: String,
    how_blending_works: String,
    style_space_topology: String,
    distance_thresholds: ExplainThresholds,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ExplainThresholds {
    identical: String,
    very_similar: String,
    compatible: String,
    distinct: String,
    opposite: String,
}

#[tool(
    description = "Explain what style centroids are, how style blending works, and how to interpret cosine distance values. Useful for agents that need to reason about the style-space topology."
)]
async fn replica_explain(_span: ToolSpanGuard) -> Result<ExplainToolResponse, McpToolError> {
    Ok(ExplainToolResponse {
        what_is_a_centroid: "\
A style centroid is the average of all embedded passage vectors for an author. \
Each passage (50-200 words) is converted to a 1024-dimensional vector. \
The centroid is the 'average passage' for that author. Prose that is stylistically \
similar to the author will have a low cosine distance to this centroid."
            .to_string(),
        how_blending_works: "\
Style blending interpolates between two author centroids: \
blended[i] = centroid_a[i] * (1 - blend) + centroid_b[i] * blend. \
A blend of 0.0 produces pure author A, 1.0 produces pure author B, and 0.5 is an equal mix. \
The blended vector retrieves exemplar passages from both corpora."
            .to_string(),
        style_space_topology: "\
Authors cluster in different regions of the 1024-dimensional embedding space. \
Similar styles have centroids that are close. Opposite styles are far apart. \
The distance matrix from replica_compare reveals which authors can be blended naturally. \
Hemingway (paratactic, short sentences) and Woolf (hypotactic, long sentences) are maximally \
distant — blending them produces incoherent prose. Similar authors (e.g., Hemingway and \
Crane, or Woolf and Proust) would blend well."
            .to_string(),
        distance_thresholds: ExplainThresholds {
            identical: "0.000 — same text (impossible for different authors)".to_string(),
            very_similar: "0.000-0.030 — nearly identical style".to_string(),
            compatible: "0.030-0.300 — stylistically related, blendable".to_string(),
            distinct: "0.300-1.000 — clearly different styles".to_string(),
            opposite: "1.000-2.000 — maximally different, blending produces noise".to_string(),
        },
    })
}

// ── Router ────────────────────────────────────────────────────────────────

#[tool_router]
async fn router(
    server: &ReplicaServer,
    method: String,
    params: Parameters,
    span: ToolSpanGuard,
) -> Result<String, McpToolError> {
    match method.as_str() {
        "replica_build" => {
            let r = replica_build(span, params).await?;
            serde_json::to_string(&r).map_err(|e| McpToolError::internal(e.to_string()))
        }
        "replica_compose" => {
            let r = replica_compose(span, params).await?;
            serde_json::to_string(&r).map_err(|e| McpToolError::internal(e.to_string()))
        }
        "replica_compare" => {
            let r = replica_compare(span, params).await?;
            serde_json::to_string(&r).map_err(|e| McpToolError::internal(e.to_string()))
        }
        "replica_mashup" => {
            let r = replica_mashup(span, params).await?;
            serde_json::to_string(&r).map_err(|e| McpToolError::internal(e.to_string()))
        }
        "replica_registry" => {
            let r = replica_registry(span, params).await?;
            serde_json::to_string(&r).map_err(|e| McpToolError::internal(e.to_string()))
        }
        "replica_explain" => {
            let r = replica_explain(span).await?;
            serde_json::to_string(&r).map_err(|e| McpToolError::internal(e.to_string()))
        }
        _ => Err(McpToolError::not_found(format!("Unknown tool: {method}"))),
    }
}

// ── Entry Point ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_server(
        "hkask-mcp-replica",
        env!("CARGO_PKG_VERSION"),
        |ctx| {
            let _webid = ctx.webid;
            Ok(ReplicaServer)
        },
        vec![],
    )
    .await
}
