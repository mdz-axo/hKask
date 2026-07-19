//! MCP server for hkask-codegraph — code understanding tools.
#![allow(unused_crate_dependencies)]

use hkask_codegraph::graph::analysis;
use hkask_codegraph::graph::traversal;
use hkask_codegraph::indexer::pipeline::IndexPipeline;
use hkask_codegraph::types::Direction;
use hkask_codegraph::{ContextBudget, graph};
use hkask_inference::config::InferenceConfig;
use hkask_inference::embedding_router::EmbeddingRouter;
use hkask_mcp::DaemonClient;
use hkask_mcp::run_server;
use hkask_mcp::server::{CapabilityTier, McpToolError, execute_tool};
use hkask_types::WebID;
use minijinja::Environment;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

hkask_mcp::mcp_server!(
    pub struct CodeGraphServer {
        pub capability_tier: CapabilityTier,
        pipeline: Arc<Mutex<IndexPipeline>>,
        embed_router: Option<EmbeddingRouter>,
        jinja: Environment<'static>,
    }
);

impl CodeGraphServer {
    fn ensure_indexed(&self) -> Result<(), McpToolError> {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut pipeline = self
            .pipeline
            .lock()
            .map_err(|_| McpToolError::internal("pipeline lock poisoned"))?;

        // OCAP-governed file access (#5): future integration point.
        // When the daemon provides capability verification, filter paths here
        // via capability tokens before passing to index_directory.
        // For now: index entire workspace (standalone mode).
        let results = pipeline
            .index_directory(&cwd)
            .map_err(|e| McpToolError::internal(format!("index failed: {e}")))?;

        let total: usize = results.iter().map(|r| r.symbols).sum();
        tracing::info!(target: "hkask.mcp.codegraph", symbols = total, "Auto-indexed");

        // Compute PageRank and emit health CNS events (G7, G8)
        if let Err(e) = pipeline.finalize() {
            tracing::warn!(target: "hkask.mcp.codegraph", error = %e, "Finalize failed");
        }
        Ok(())
    }
}

// ── Request types for tools with structured parameters ────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryRequest {
    query: String,
    #[serde(default = "default_limit")]
    limit: u64,
    /// Optional: look up exact symbol name (replaces codegraph_node)
    #[serde(default)]
    name: Option<String>,
}
fn default_limit() -> u64 {
    10
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TraverseRequest {
    symbol: String,
    #[serde(default = "default_forward")]
    direction: String,
    #[serde(default = "default_depth")]
    max_depth: u64,
}
fn default_forward() -> String {
    "forward".into()
}
fn default_depth() -> u64 {
    5
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImpactRequest {
    symbol: String,
    #[serde(default = "default_depth")]
    max_depth: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ContextRequest {
    query: String,
    #[serde(default = "default_budget")]
    budget: String,
}
fn default_budget() -> String {
    "standard".into()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalysisRequest {
    kind: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StructureRequest {
    #[serde(default = "default_structure_limit")]
    limit: u64,
}
fn default_structure_limit() -> u64 {
    20
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StatsRequest {
    #[serde(default)]
    include_health: bool,
    /// Include language/file-type breakdown (replaces codegraph_project_meta)
    #[serde(default)]
    include_meta: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FeedbackRequest {
    context_id: String,
    #[serde(default)]
    symbols_provided: Vec<String>,
    #[serde(default)]
    symbols_used: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EmbedIndexRequest {
    /// Embedding model to use (defaults to HKASK_EMBEDDING_MODEL env var, or DI/Qwen/Qwen3-Embedding-0.6B)
    #[serde(default = "default_embed_model")]
    model: String,
    /// Batch size for embedding calls (default 32)
    #[serde(default = "default_batch")]
    batch_size: u64,
}
fn default_embed_model() -> String {
    std::env::var("HKASK_EMBEDDING_MODEL")
        .unwrap_or_else(|_| "DI/Qwen/Qwen3-Embedding-0.6B".to_string())
}
fn default_batch() -> u64 {
    32
}

// ── Tools ─────────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl CodeGraphServer {
    #[tool(
        description = "Search the codebase for symbols, or look up a specific symbol by name (set 'name' field)"
    )]
    pub async fn codegraph_query(&self, Parameters(req): Parameters<QueryRequest>) -> String {
        execute_tool(self, "codegraph_query", async {
            self.ensure_indexed()?;
            let pipeline = self
                .pipeline
                .lock()
                .map_err(|_| McpToolError::internal("pipeline lock poisoned"))?;
            let results =
                graph::search::search(pipeline.store().conn(), &req.query, req.limit as usize)
                    .map_err(|e| McpToolError::internal(e.to_string()))?;
            // If name provided, return exact symbol match (replaces codegraph_node)
            if let Some(ref name) = req.name {
                let exact = results.iter().find(|r| r.symbol.name == *name);
                return Ok(serde_json::json!(exact.map(|r| &r.symbol)));
            }
            Ok(serde_json::json!(results))
        })
        .await
    }

    #[tool(description = "Traverse the code graph: forward (dependencies) or reverse (callers)")]
    pub async fn codegraph_traverse(&self, Parameters(req): Parameters<TraverseRequest>) -> String {
        execute_tool(self, "codegraph_traverse", async {
            let dir = match req.direction.as_str() {
                "forward" => Direction::Forward,
                "reverse" => Direction::Reverse,
                _ => {
                    return Err(McpToolError::invalid_argument(
                        "direction must be 'forward' or 'reverse'",
                    ));
                }
            };
            self.ensure_indexed()?;
            let pipeline = self
                .pipeline
                .lock()
                .map_err(|_| McpToolError::internal("pipeline lock poisoned"))?;
            let id = traversal::find_symbol_id(pipeline.store().conn(), &req.symbol)
                .map_err(|e| McpToolError::internal(e.to_string()))?;
            match id {
                Some(id) => {
                    let nodes = traversal::traverse(
                        pipeline.store().conn(),
                        id,
                        dir,
                        req.max_depth as usize,
                    )
                    .map_err(|e| McpToolError::internal(e.to_string()))?;
                    Ok(serde_json::json!(nodes))
                }
                None => {
                    Ok(serde_json::json!({"error": format!("symbol not found: {}", req.symbol)}))
                }
            }
        })
        .await
    }

    #[tool(description = "Analyze blast radius for a symbol")]
    pub async fn codegraph_impact(&self, Parameters(req): Parameters<ImpactRequest>) -> String {
        execute_tool(self, "codegraph_impact", async {
            self.ensure_indexed()?;
            let pipeline = self
                .pipeline
                .lock()
                .map_err(|_| McpToolError::internal("pipeline lock poisoned"))?;
            let id = traversal::find_symbol_id(pipeline.store().conn(), &req.symbol)
                .map_err(|e| McpToolError::internal(e.to_string()))?;
            match id {
                Some(id) => {
                    let results = traversal::impact_analysis(
                        pipeline.store().conn(),
                        id,
                        req.max_depth as usize,
                    )
                    .map_err(|e| McpToolError::internal(e.to_string()))?;
                    Ok(serde_json::json!({
                        "symbol": req.symbol,
                        "total_affected": results.len(),
                        "affected": results,
                    }))
                }
                None => {
                    Ok(serde_json::json!({"error": format!("symbol not found: {}", req.symbol)}))
                }
            }
        })
        .await
    }

    #[tool(description = "Run analysis: 'dead_code' or 'complexity'")]
    pub async fn codegraph_analysis(&self, Parameters(req): Parameters<AnalysisRequest>) -> String {
        execute_tool(self, "codegraph_analysis", async {
            self.ensure_indexed()?;
            let pipeline = self
                .pipeline
                .lock()
                .map_err(|_| McpToolError::internal("pipeline lock poisoned"))?;
            match req.kind.as_str() {
                "dead_code" => {
                    let findings = analysis::find_dead_code(pipeline.store().conn())
                        .map_err(|e| McpToolError::internal(e.to_string()))?;
                    Ok(serde_json::json!(findings))
                }
                "complexity" => {
                    let findings = analysis::find_high_complexity(pipeline.store().conn(), 10, 5)
                        .map_err(|e| McpToolError::internal(e.to_string()))?;
                    Ok(serde_json::json!(findings))
                }
                _ => Err(McpToolError::invalid_argument(
                    "kind must be 'dead_code' or 'complexity'",
                )),
            }
        })
        .await
    }

    #[tool(description = "Assemble token-budgeted context for LLM prompts")]
    pub async fn codegraph_context(&self, Parameters(req): Parameters<ContextRequest>) -> String {
        execute_tool(self, "codegraph_context", async {
            let budget = match req.budget.as_str() {
                "full" => ContextBudget::Full,
                "standard" => ContextBudget::Standard,
                "focused" => ContextBudget::Focused,
                "minimal" => ContextBudget::Minimal,
                _ => {
                    return Err(McpToolError::invalid_argument(
                        "budget must be minimal/focused/standard/full",
                    ));
                }
            };
            self.ensure_indexed()?;
            let pipeline = self
                .pipeline
                .lock()
                .map_err(|_| McpToolError::internal("pipeline lock poisoned"))?;
            let assembled =
                hkask_codegraph::assemble_context(pipeline.store().conn(), &req.query, budget)
                    .map_err(|e| McpToolError::internal(e.to_string()))?;
            Ok(serde_json::json!({
                "context_id": assembled.context_id.to_string(),
                "text": assembled.text,
                "symbols": assembled.symbols,
                "estimated_tokens": assembled.estimated_tokens,
            }))
        })
        .await
    }

    #[tool(description = "Get project overview: top symbols")]
    pub async fn codegraph_structure(
        &self,
        Parameters(req): Parameters<StructureRequest>,
    ) -> String {
        execute_tool(self, "codegraph_structure", async {
            self.ensure_indexed()?;
            let pipeline = self
                .pipeline
                .lock()
                .map_err(|_| McpToolError::internal("pipeline lock poisoned"))?;
            let conn = pipeline.store().conn();
            let limit = req.limit as i64;
            let mut stmt = conn
                .prepare(
                    "SELECT name, kind, f.path, signature, visibility, pagerank
                 FROM symbols s JOIN code_files f ON s.file_id = f.id
                 ORDER BY pagerank DESC LIMIT ?1",
                )
                .map_err(|e| McpToolError::internal(e.to_string()))?;
            let rows: Vec<serde_json::Value> = stmt
                .query_map(rusqlite::params![limit], |row| {
                    Ok(serde_json::json!({
                        "name": row.get::<_, String>(0)?,
                        "kind": row.get::<_, String>(1)?,
                        "file": row.get::<_, String>(2)?,
                        "signature": row.get::<_, String>(3)?,
                        "visibility": row.get::<_, String>(4)?,
                        "pagerank": row.get::<_, f64>(5)?,
                    }))
                })
                .map_err(|e| McpToolError::internal(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();
            Ok(serde_json::json!(rows))
        })
        .await
    }

    #[tool(description = "Get index statistics")]
    pub async fn codegraph_stats(&self, Parameters(req): Parameters<StatsRequest>) -> String {
        execute_tool(self, "codegraph_stats", async {
            let pipeline = self
                .pipeline
                .lock()
                .map_err(|_| McpToolError::internal("pipeline lock poisoned"))?;
            let stats = pipeline
                .stats()
                .map_err(|e| McpToolError::internal(e.to_string()))?;
            let mut output = serde_json::json!({
                "files": stats.files, "symbols": stats.symbols, "edges": stats.edges,
            });
            if req.include_health && stats.symbols > 0 {
                let ratio = stats.edges as f64 / stats.symbols as f64;
                output["connectivity_ratio"] = serde_json::json!(ratio);
                output["health"] = serde_json::json!(if ratio < 0.1 {
                    "poor"
                } else if ratio < 0.5 {
                    "fair"
                } else {
                    "good"
                });
            }
            // Include language/file-type breakdown if requested (X4: merged from codegraph_project_meta)
            if req.include_meta {
                let conn = pipeline.store().conn();
                let mut stmt = conn
                    .prepare(
                        "SELECT COUNT(*),
                        SUM(CASE WHEN path LIKE '%.rs' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN path LIKE '%.toml' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN path LIKE '%.md' THEN 1 ELSE 0 END)
                 FROM code_files",
                    )
                    .map_err(|e| McpToolError::internal(e.to_string()))?;
                if let Ok(meta) = stmt.query_row([], |row| {
                    Ok(serde_json::json!({
                        "total": row.get::<_, i64>(0)?,
                        "rust": row.get::<_, i64>(1)?,
                        "toml": row.get::<_, i64>(2)?,
                        "md": row.get::<_, i64>(3)?,
                        "primary_language": "Rust",
                    }))
                }) {
                    output["meta"] = meta;
                }
            }
            Ok(output)
        })
        .await
    }

    #[tool(description = "Force full re-index of the workspace")]
    pub async fn codegraph_reindex(&self) -> String {
        execute_tool(self, "codegraph_reindex", async {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let pipeline = self.pipeline.lock()
                .map_err(|_| McpToolError::internal("pipeline lock poisoned"))?;
            let results = pipeline.index_directory(&cwd)
                .map_err(|e| McpToolError::internal(e.to_string()))?;
            let total_sym: usize = results.iter().map(|r| r.symbols).sum();
            let total_edg: usize = results.iter().map(|r| r.edges).sum();
            let indexed: usize = results.iter().filter(|r| !r.skipped).count();
            let stats = pipeline.stats().map_err(|e| McpToolError::internal(e.to_string()))?;
            Ok(serde_json::json!({
                "files_indexed": indexed, "symbols_added": total_sym, "edges_added": total_edg,
                "total_files": stats.files, "total_symbols": stats.symbols, "total_edges": stats.edges,
            }))
        }).await
    }

    #[tool(
        description = "Record which symbols from a context_id were actually used (G12 feedback loop)"
    )]
    pub async fn codegraph_feedback(&self, Parameters(req): Parameters<FeedbackRequest>) -> String {
        execute_tool(self, "codegraph_feedback", async {
            let ratio = if req.symbols_provided.is_empty() {
                0.0
            } else {
                req.symbols_used.len() as f64 / req.symbols_provided.len() as f64
            };
            tracing::info!(
                target: "hkask.codegraph.context_efficiency",
                context_id = %req.context_id,
                symbols_provided = req.symbols_provided.len(),
                symbols_used = req.symbols_used.len(),
                ratio = ratio,
            );
            Ok(serde_json::json!({"recorded": true, "context_id": req.context_id, "ratio": ratio}))
        })
        .await
    }

    #[tool(
        description = "Generate embeddings for all symbols using the inference router, enabling semantic vector search (G13)"
    )]
    pub async fn codegraph_index_embeddings(
        &self,
        Parameters(req): Parameters<EmbedIndexRequest>,
    ) -> String {
        execute_tool(self, "codegraph_index_embeddings", async {
            let embed_router = self.embed_router.as_ref()
                .ok_or_else(|| McpToolError::invalid_argument(
                    "No embedding provider configured. Set DEEPINFRA_API_KEY or OPENROUTER_API_KEY."
                ))?;

            // Phase 1: extract all symbol data from DB (before any .await)
            let rows: Vec<(i64, serde_json::Value)> = {
                let pipeline = self.pipeline.lock()
                    .map_err(|_| McpToolError::internal("pipeline lock poisoned"))?;
                let conn = pipeline.store().conn();
                let mut stmt = conn.prepare(
                    "SELECT s.id, s.name, s.kind, s.signature, s.doc_comment,
                            s.visibility, f.path
                     FROM symbols s
                     JOIN code_files f ON s.file_id = f.id
                     WHERE NOT EXISTS (
                         SELECT 1 FROM symbols_vec WHERE rowid = s.id
                     )
                     ORDER BY s.name"
                ).map_err(|e| McpToolError::internal(e.to_string()))?;
                stmt.query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, serde_json::json!({
                        "name": row.get::<_, String>(1)?,
                        "kind": row.get::<_, String>(2)?,
                        "signature": row.get::<_, String>(3)?,
                        "doc": row.get::<_, Option<String>>(4)?,
                        "visibility": row.get::<_, String>(5)?,
                        "file": row.get::<_, String>(6)?,
                    })))
                }).map_err(|e| McpToolError::internal(e.to_string()))?
                .filter_map(|r| r.ok()).collect()
            };
            // Connection dropped here — safe to .await now

            if rows.is_empty() {
                return Ok(serde_json::json!({"indexed": 0, "message": "All symbols already have embeddings"}));
            }

            let batch_size = req.batch_size as usize;
            let mut all_embeddings: Vec<(i64, Vec<f32>)> = Vec::new();

            // Phase 2: generate embeddings (safe to .await, no SQLite references)
            for chunk in rows.chunks(batch_size) {
                let texts: Vec<String> = chunk.iter().map(|(_, ctx)| {
                    match self.jinja.get_template("symbol-embedding.j2") {
                        Ok(tmpl) => tmpl.render(ctx).unwrap_or_else(|e| {
                            tracing::warn!(target: "hkask.mcp.codegraph", error = %e, "Template render failed");
                            ctx["signature"].as_str().unwrap_or("").to_string()
                        }),
                        Err(_) => ctx["signature"].as_str().unwrap_or("").to_string(),
                    }
                }).collect();

                let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
                let embeddings = embed_router.embed_sentences(&req.model, &text_refs).await
                    .map_err(|e| McpToolError::internal(format!("Embedding failed: {e}")))?;

                for (i, (id, _ctx)) in chunk.iter().enumerate() {
                    if i < embeddings.len() {
                        all_embeddings.push((*id, embeddings[i].clone()));
                    }
                }
            }

            // Phase 3: insert into DB (re-acquire connection)
            let indexed = {
                let pipeline = self.pipeline.lock()
                    .map_err(|_| McpToolError::internal("pipeline lock poisoned"))?;
                let conn = pipeline.store().conn();
                let mut count = 0usize;
                for (id, embedding) in &all_embeddings {
                    let blob: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
                    conn.execute(
                        "INSERT OR IGNORE INTO symbols_vec(rowid, embedding) VALUES (?1, ?2)",
                        rusqlite::params![id, blob],
                    ).map_err(|e| McpToolError::internal(e.to_string()))?;
                    count += 1;
                }
                count
            };

            tracing::info!(
                target: "hkask.codegraph.embeddings",
                model = %req.model,
                indexed = indexed,
                "Embedding batch indexed"
            );

            Ok(serde_json::json!({
                "indexed": indexed,
                "model": req.model,
                "message": format!("Indexed {indexed} symbol embeddings via {}", req.model),
            }))
        }).await
    }
}

pub async fn run(
    replicant: String,
    daemon_client: Option<DaemonClient>,
) -> Result<(), hkask_mcp::McpError> {
    let db_path = std::env::var("HKASK_CODEGRAPH_DB").ok();
    run_server(
        "hkask-mcp-codegraph",
        SERVER_VERSION,
        |_ctx| {
            let webid = WebID::new();
            let store = match &db_path {
                Some(path) => hkask_codegraph::graph::store::GraphStore::open(path)
                    .map_err(|e| hkask_mcp::McpError::from(std::io::Error::other(e.to_string())))?,
                None => hkask_codegraph::graph::store::GraphStore::open_in_memory()
                    .map_err(|e| hkask_mcp::McpError::from(std::io::Error::other(e.to_string())))?,
            };
            let pipeline = IndexPipeline::new(store);
            let config = InferenceConfig::from_env();
            let embed_router =
                if config.deepinfra_api_key.is_empty() && config.openrouter_api_key.is_empty() {
                    None
                } else {
                    Some(EmbeddingRouter::new(config))
                };
            let mut jinja = Environment::new();
            jinja
                .add_template_owned(
                    "symbol-embedding.j2",
                    include_str!("../../../registry/templates/codegraph/symbol-embedding.j2")
                        .to_string(),
                )
                .map_err(|e| hkask_mcp::McpError::from(std::io::Error::other(e.to_string())))?;
            jinja
                .add_template_owned(
                    "symbol-summarize.j2",
                    include_str!("../../../registry/templates/codegraph/symbol-summarize.j2")
                        .to_string(),
                )
                .map_err(|e| hkask_mcp::McpError::from(std::io::Error::other(e.to_string())))?;
            jinja
                .add_template_owned(
                    "fix-suggestion.j2",
                    include_str!("../../../registry/templates/codegraph/fix-suggestion.j2")
                        .to_string(),
                )
                .map_err(|e| hkask_mcp::McpError::from(std::io::Error::other(e.to_string())))?;
            jinja
                .add_template_owned(
                    "analysis-dead-code.j2",
                    include_str!("../../../registry/templates/codegraph/analysis-dead-code.j2")
                        .to_string(),
                )
                .map_err(|e| hkask_mcp::McpError::from(std::io::Error::other(e.to_string())))?;
            jinja
                .add_template_owned(
                    "analysis-complexity.j2",
                    include_str!("../../../registry/templates/codegraph/analysis-complexity.j2")
                        .to_string(),
                )
                .map_err(|e| hkask_mcp::McpError::from(std::io::Error::other(e.to_string())))?;
            Ok(CodeGraphServer::new(
                webid,
                replicant.clone(),
                daemon_client.clone(),
                CapabilityTier::detect(&std::collections::HashMap::new()),
                Arc::new(Mutex::new(pipeline)),
                embed_router,
                jinja,
            ))
        },
        vec![],
    )
    .await
}
