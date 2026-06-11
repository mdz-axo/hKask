//! hKask MCP Memory — Unified episodic + semantic memory MCP server
//!
//! 16 tools:
//! - `episodic_ping` — Liveness and storage info for episodic memory
//! - `episodic_store` — Store an episodic triple (private, perspective-bound)
//! - `episodic_recall` — Recall triples by entity (filtered by caller's WebID)
//! - `episodic_budget` — Storage usage and budget info
//! - `episodic_consolidate_status` — Check consolidation candidates and budget status
//! - `semantic_ping` — Liveness and storage info for semantic memory
//! - `semantic_store` — Store a shared semantic triple (no perspective)
//! - `semantic_recall` — Recall triples by entity (public, any agent can read)
//! - `semantic_embed` — Store an embedding vector for similarity search
//! - `semantic_search` — KNN similarity search over embeddings
//! - `semantic_centroid` — Compute mean embedding vector for a prefix-filtered set
//! - `semantic_purge` — Delete embeddings matching an entity_ref prefix
//! - `semantic_chunk` — Chunk text into passages for embedding
//! - `semantic_count` — Triple and embedding counts
//! - `memory_backup` — Export the memory database to a local backup file
//! - `memory_restore` — Restore the memory database from a local backup file
//!
//! **Sovereignty:** Episodic operations use the calling agent's `WebID` as
//! the `perspective`. An agent cannot read another agent's episodic memory.
//! Semantic operations use public `Visibility` — any agent can read shared triples.
//!
//! This server replaces `hkask-mcp-episodic` and `hkask-mcp-semantic`,
//! merging both into a single server that connects to one per-agent memory DB
//! (`HKASK_MEMORY_DB` / `hkask-memory-{agent}.db`).

use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_mcp::validate_field;
use hkask_memory::{EpisodicMemory, SemanticMemory};
use hkask_storage::Triple;
use hkask_types::{McpErrorKind, Visibility, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

// ── Shared request types ───────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StoreRequest {
    pub entity: String,
    pub attribute: String,
    pub value: serde_json::Value,
    pub confidence: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RecallRequest {
    pub entity: String,
}

// ── Episodic-specific request types ─────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BudgetRequest {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConsolidateStatusRequest {}

// ── Semantic-specific request types ─────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EmbedRequest {
    pub entity_ref: String,
    pub vector: Vec<f32>,
    pub model: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchRequest {
    pub query_vector: Vec<f32>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CentroidRequest {
    pub prefix: String,
    pub exclude_prefix: String,
    pub exclude_ref: String,
    pub dim: usize,
    pub store_as: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PurgeRequest {
    pub prefix: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChunkTextRequest {
    pub text: String,
    pub entity_ref_prefix: String,
    pub min_words: Option<usize>,
    pub max_words: Option<usize>,
    pub sentence_boundary: Option<String>,
    pub strip_gutenberg: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CountRequest {}

// ── Backup/restore request types ──────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BackupRequest {
    /// File path for the backup. Defaults to "hkask-memory-backup.db"
    /// if not provided.
    pub target_path: Option<String>,
    /// Optional passphrase for the backup file. If not provided,
    /// the backup is unencrypted (plain SQLite).
    pub passphrase: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RestoreRequest {
    /// Path to the backup file to restore from.
    pub source_path: String,
    /// Passphrase for the backup file. Required if the backup was encrypted.
    pub passphrase: Option<String>,
}

// ── Server ──────────────────────────────────────────────────────────

pub struct MemoryServer {
    episodic: EpisodicMemory,
    semantic: Arc<SemanticMemory>,
    db: Option<Arc<std::sync::Mutex<rusqlite::Connection>>>,
    webid: WebID,
}

impl MemoryServer {
    pub fn new(
        episodic: EpisodicMemory,
        semantic: Arc<SemanticMemory>,
        db: Option<Arc<std::sync::Mutex<rusqlite::Connection>>>,
        webid: WebID,
    ) -> Self {
        Self {
            episodic,
            semantic,
            db,
            webid,
        }
    }

    fn internal_error(
        &self,
        span: ToolSpanGuard,
        context: &str,
        e: impl std::fmt::Display,
    ) -> String {
        span.internal_error(json!({"error": format!("Failed to {}: {}", context, e)}))
    }
}

#[tool_router(server_handler)]
impl MemoryServer {
    // ── Episodic tools ──────────────────────────────────────────

    #[tool(description = "Liveness and storage info for episodic memory")]
    async fn episodic_ping(&self) -> String {
        let span = ToolSpanGuard::new("episodic_ping", &self.webid);
        span.ok_json(json!({
            "status": "ok",
            "server": "hkask-mcp-memory",
            "perspective": self.webid.to_string(),
        }))
    }

    #[tool(description = "Store an episodic triple (private, perspective-bound)")]
    async fn episodic_store(
        &self,
        Parameters(StoreRequest {
            entity,
            attribute,
            value,
            confidence,
        }): Parameters<StoreRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("episodic_store", &self.webid);
        validate_field!(span, "entity", &entity, 256);
        validate_field!(span, "attribute", &attribute, 256);
        let triple = Triple::new(&entity, &attribute, value, self.webid)
            .with_perspective(self.webid)
            .with_confidence(confidence.unwrap_or(1.0))
            .with_visibility(Visibility::Private);
        match self.episodic.store(triple) {
            Ok(()) => span.ok_json(json!({
                "stored": true, "entity": entity, "attribute": attribute,
            })),
            Err(e) => span.internal_error(
                json!({"error": format!("Failed to store episodic triple: {}", e)}),
            ),
        }
    }

    #[tool(description = "Recall episodic triples by entity (filtered by caller's WebID)")]
    async fn episodic_recall(
        &self,
        Parameters(RecallRequest { entity }): Parameters<RecallRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("episodic_recall", &self.webid);
        validate_field!(span, "entity", &entity, 256);
        match self.episodic.query_for_deduped(&entity, self.webid) {
            Ok(triples) => {
                let serialized: Vec<serde_json::Value> = triples
                    .iter()
                    .map(|t| {
                        json!({
                            "entity": t.entity,
                            "attribute": t.attribute,
                            "value": t.value,
                            "confidence": t.confidence,
                            "valid_from": t.temporal.valid_from.to_rfc3339(),
                        })
                    })
                    .collect();
                span.ok_json(json!({"count": serialized.len(), "triples": serialized}))
            }
            Err(e) => span.internal_error(
                json!({"error": format!("Failed to recall episodic triples: {}", e)}),
            ),
        }
    }

    #[tool(description = "Storage usage and budget for episodic memory")]
    async fn episodic_budget(&self, Parameters(_budget): Parameters<BudgetRequest>) -> String {
        let span = ToolSpanGuard::new("episodic_budget", &self.webid);
        let usage = match self.episodic.storage_usage(&self.webid) {
            Ok(u) => u,
            Err(e) => {
                return span.internal_error(
                    json!({"error": format!("Failed to query storage usage: {}", e)}),
                );
            }
        };
        let budget = self.episodic.storage_budget();
        let remaining = budget.saturating_sub(usage);
        span.ok_json(json!({"used": usage, "budget": budget, "remaining": remaining}))
    }

    #[tool(
        description = "Check consolidation candidates and budget status for episodic→semantic promotion"
    )]
    async fn episodic_consolidate_status(
        &self,
        Parameters(_req): Parameters<ConsolidateStatusRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("episodic_consolidate_status", &self.webid);
        let candidate_count = self.episodic.consolidation_candidate_count(&self.webid);
        let usage = match self.episodic.storage_usage(&self.webid) {
            Ok(u) => u,
            Err(e) => {
                return span.internal_error(
                    json!({"error": format!("Failed to query storage usage: {}", e)}),
                );
            }
        };
        let budget = self.episodic.storage_budget();
        let over_budget = usage > budget;
        span.ok_json(json!({
            "consolidation_candidates": candidate_count,
            "episodic_usage": usage,
            "episodic_budget": budget,
            "over_budget": over_budget,
        }))
    }

    // ── Semantic tools ──────────────────────────────────────────

    #[tool(description = "Liveness and storage info for semantic memory")]
    async fn semantic_ping(&self) -> String {
        let span = ToolSpanGuard::new("semantic_ping", &self.webid);
        span.ok_json(json!({"status": "ok", "server": "hkask-mcp-memory"}))
    }

    #[tool(description = "Store a shared semantic triple (no perspective)")]
    async fn semantic_store(
        &self,
        Parameters(StoreRequest {
            entity,
            attribute,
            value,
            confidence,
        }): Parameters<StoreRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("semantic_store", &self.webid);
        validate_field!(span, "entity", &entity, 256);
        validate_field!(span, "attribute", &attribute, 256);
        let triple = Triple::new(&entity, &attribute, value, self.webid)
            .with_visibility(Visibility::Public)
            .with_confidence(confidence.unwrap_or(1.0));
        match self.semantic.store(triple) {
            Ok(()) => {
                span.ok_json(json!({"stored": true, "entity": entity, "attribute": attribute}))
            }
            Err(e) => self.internal_error(span, "store semantic triple", e),
        }
    }

    #[tool(description = "Recall shared semantic triples by entity")]
    async fn semantic_recall(
        &self,
        Parameters(RecallRequest { entity }): Parameters<RecallRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("semantic_recall", &self.webid);
        validate_field!(span, "entity", &entity, 256);
        match self.semantic.query_deduped(&entity) {
            Ok(triples) => {
                let serialized: Vec<_> = triples
                    .iter()
                    .map(|t| {
                        json!({
                            "entity": t.entity,
                            "attribute": t.attribute,
                            "value": t.value,
                            "confidence": t.confidence,
                            "valid_from": t.temporal.valid_from.to_rfc3339(),
                        })
                    })
                    .collect();
                span.ok_json(json!({"count": serialized.len(), "triples": serialized}))
            }
            Err(e) => self.internal_error(span, "recall semantic triples", e),
        }
    }

    #[tool(description = "Store an embedding vector for similarity search")]
    async fn semantic_embed(
        &self,
        Parameters(EmbedRequest {
            entity_ref,
            vector,
            model,
        }): Parameters<EmbedRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("semantic_embed", &self.webid);
        validate_field!(span, "entity_ref", &entity_ref, 256);
        if vector.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("vector must not be empty").to_json_string(),
            );
        }
        match self.semantic.store_embedding(&entity_ref, &vector, &model) {
            Ok(_id) => span.ok_json(json!({
                "stored": true,
                "entity_ref": entity_ref,
                "model": model,
                "dimensions": vector.len(),
            })),
            Err(e) => self.internal_error(span, "store embedding", e),
        }
    }

    #[tool(description = "KNN similarity search over embeddings")]
    async fn semantic_search(
        &self,
        Parameters(SearchRequest {
            query_vector,
            limit,
        }): Parameters<SearchRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("semantic_search", &self.webid);
        if query_vector.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("query_vector must not be empty").to_json_string(),
            );
        }
        match self
            .semantic
            .search_similar(&query_vector, limit.unwrap_or(10))
        {
            Ok(results) => {
                let serialized: Vec<_> = results
                    .iter()
                    .map(|r| {
                        json!({
                            "entity_ref": r.embedding.entity_ref,
                            "model": r.embedding.model,
                            "distance": r.distance,
                        })
                    })
                    .collect();
                span.ok_json(json!({"count": serialized.len(), "results": serialized}))
            }
            Err(e) => self.internal_error(span, "search embeddings", e),
        }
    }

    #[tool(
        description = "Compute mean embedding vector (centroid) for embeddings matching a prefix"
    )]
    async fn semantic_centroid(
        &self,
        Parameters(CentroidRequest {
            prefix,
            exclude_prefix,
            exclude_ref,
            dim,
            store_as,
            model,
        }): Parameters<CentroidRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("semantic_centroid", &self.webid);
        validate_field!(span, "prefix", &prefix, 256);
        validate_field!(span, "exclude_prefix", &exclude_prefix, 256);
        validate_field!(span, "exclude_ref", &exclude_ref, 256);
        if dim == 0 {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("dim must be positive").to_json_string(),
            );
        }
        match self.semantic.compute_centroid(
            &prefix,
            &exclude_prefix,
            &exclude_ref,
            dim,
            store_as.as_deref(),
            model.as_deref(),
        ) {
            Ok(result) => span.ok_json(json!({
                "centroid": result.centroid,
                "dimensions": result.centroid.len(),
                "prefix": prefix,
                "passage_count": result.passage_count,
                "stored": result.stored,
            })),
            Err(e) => self.internal_error(span, "compute centroid", e),
        }
    }

    #[tool(description = "Delete all embeddings whose entity_ref starts with a prefix")]
    async fn semantic_purge(
        &self,
        Parameters(PurgeRequest { prefix }): Parameters<PurgeRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("semantic_purge", &self.webid);
        validate_field!(span, "prefix", &prefix, 256);
        match self.semantic.purge_by_prefix(&prefix) {
            Ok(count) => span.ok_json(json!({"purged": count, "prefix": prefix})),
            Err(e) => self.internal_error(span, "purge embeddings", e),
        }
    }

    #[tool(
        description = "Chunk text into passages for embedding, with optional Gutenberg header stripping"
    )]
    async fn semantic_chunk(
        &self,
        Parameters(ChunkTextRequest {
            text,
            entity_ref_prefix,
            min_words,
            max_words,
            sentence_boundary,
            strip_gutenberg,
        }): Parameters<ChunkTextRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("semantic_chunk", &self.webid);
        if text.is_empty() || entity_ref_prefix.is_empty() {
            let field = if text.is_empty() {
                "text"
            } else {
                "entity_ref_prefix"
            };
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!("{field} must not be empty"))
                    .to_json_string(),
            );
        }
        validate_field!(span, "entity_ref_prefix", &entity_ref_prefix, 256);
        let min_w = min_words.unwrap_or(50);
        let max_w = max_words.unwrap_or(200);
        let boundary = sentence_boundary.unwrap_or_else(|| ".!? ".to_string());
        let processed = if strip_gutenberg.unwrap_or(false) {
            SemanticMemory::strip_gutenberg_headers(&text)
        } else {
            text.clone()
        };
        let passages =
            SemanticMemory::chunk_text(&processed, &entity_ref_prefix, min_w, max_w, &boundary);
        let serialized: Vec<_> = passages
            .into_iter()
            .map(|(entity_ref, passage_text)| {
                json!({"entity_ref": entity_ref, "text": passage_text})
            })
            .collect();
        span.ok_json(json!({
            "total_passages": serialized.len(),
            "passages": serialized,
            "min_words": min_w,
            "max_words": max_w,
            "sentence_boundary": boundary,
            "stripped_gutenberg": strip_gutenberg.unwrap_or(false),
        }))
    }

    #[tool(description = "Triple and embedding counts for semantic memory")]
    async fn semantic_count(&self, Parameters(_req): Parameters<CountRequest>) -> String {
        let span = ToolSpanGuard::new("semantic_count", &self.webid);
        let triple_count = match self.semantic.triple_count() {
            Ok(c) => c,
            Err(e) => return self.internal_error(span, "count triples", e),
        };
        let embedding_count = match self.semantic.embedding_count() {
            Ok(c) => c,
            Err(e) => return self.internal_error(span, "count embeddings", e),
        };
        span.ok_json(json!({"triple_count": triple_count, "embedding_count": embedding_count}))
    }

    // ── Backup/restore tools ───────────────────────────────────

    #[tool(description = "Export the memory database to a local backup file")]
    async fn memory_backup(
        &self,
        Parameters(BackupRequest {
            target_path,
            passphrase: _passphrase,
        }): Parameters<BackupRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("memory_backup", &self.webid);
        let target = target_path.unwrap_or_else(|| "hkask-memory-backup.sqlite".to_string());
        // passphrase is reserved for future encrypted backup support.
        // Currently all backups are unencrypted plain SQLite.

        let Some(ref db_conn) = self.db else {
            return self.internal_error(span, "backup", "in-memory database");
        };

        // Open the destination as a plain unencrypted SQLite file
        let mut dst_conn = match rusqlite::Connection::open(&target) {
            Ok(conn) => conn,
            Err(e) => {
                return self.internal_error(span, "open backup destination", e);
            }
        };

        // Copy source → destination using SQLite's backup API
        let result = {
            let src_conn = match db_conn.lock() {
                Ok(guard) => guard,
                Err(_) => return self.internal_error(span, "backup", "lock poisoned"),
            };
            rusqlite::backup::Backup::new(&src_conn, &mut dst_conn)
                .map_err(|e| format!("Backup setup failed: {}", e))
                .and_then(|backup| {
                    backup
                        .run_to_completion(100, Duration::from_millis(250), None)
                        .map_err(|e| format!("Backup failed: {}", e))
                })
        };

        match result {
            Ok(()) => span.ok_json(json!({
                "backed_up": true,
                "target_path": target,
            })),
            Err(e) => self.internal_error(span, "backup", e),
        }
    }

    #[tool(description = "Restore the memory database from a local backup file")]
    async fn memory_restore(
        &self,
        Parameters(RestoreRequest {
            source_path,
            passphrase: _passphrase,
        }): Parameters<RestoreRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("memory_restore", &self.webid);
        // passphrase is reserved for future encrypted restore support.

        let Some(ref db_conn) = self.db else {
            return self.internal_error(span, "restore", "in-memory database");
        };

        // Validate source file exists and is a SQLite database before destroying current data
        let src_conn = match rusqlite::Connection::open(&source_path) {
            Ok(conn) => {
                // Quick validation: try reading sqlite_master
                if let Err(e) = conn.query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table'",
                    [],
                    |row| row.get::<_, i64>(0),
                ) {
                    return self.internal_error(
                        span,
                        "validate backup source",
                        format!("Not a valid SQLite database: {}", e),
                    );
                }
                conn
            }
            Err(e) => {
                return self.internal_error(span, "open backup source", e);
            }
        };

        // Clear current database, then copy backup → current
        let result = {
            let mut dst_conn = match db_conn.lock() {
                Ok(guard) => guard,
                Err(_) => return self.internal_error(span, "restore", "lock poisoned"),
            };
            if let Err(e) = dst_conn.execute_batch(
                "PRAGMA writable_schema = 1; \
                 DELETE FROM sqlite_master WHERE type IN ('table', 'index', 'trigger'); \
                 PRAGMA writable_schema = 0;",
            ) {
                Err(format!("Failed to clear existing data: {}", e))
            } else {
                rusqlite::backup::Backup::new(&src_conn, &mut dst_conn)
                    .map_err(|e| format!("Restore setup failed: {}", e))
                    .and_then(|backup| {
                        backup
                            .run_to_completion(100, Duration::from_millis(250), None)
                            .map_err(|e| format!("Restore failed: {}", e))
                    })
            }
        };

        match result {
            Ok(()) => span.ok_json(json!({
                "restored": true,
                "source_path": source_path,
                "warning": "Memory restored. Restart the MCP server for full consistency across all connections.",
            })),
            Err(e) => self.internal_error(span, "restore", e),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hkask_mcp::run_server(
        "hkask-mcp-memory",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            let db = ctx.open_database("HKASK_MEMORY_DB")?;
            let conn = db.conn_arc();
            let triple_store = hkask_storage::TripleStore::new(Arc::clone(&conn));
            let episodic = hkask_memory::EpisodicMemory::new(triple_store);
            let conn2 = db.conn_arc();
            let triple_store2 = hkask_storage::TripleStore::new(Arc::clone(&conn2));
            let embedding_store = hkask_storage::EmbeddingStore::new(conn2);
            let semantic = Arc::new(hkask_memory::SemanticMemory::new(
                triple_store2,
                embedding_store,
            ));
            Ok(MemoryServer::new(
                episodic,
                semantic,
                Some(db.conn_arc()),
                ctx.webid,
            ))
        },
        vec![
            hkask_mcp::CredentialRequirement::required(
                "HKASK_MEMORY_DB",
                "Path to per-agent memory database file (episodic + semantic)",
            ),
            hkask_mcp::CredentialRequirement::required(
                "HKASK_DB_PASSPHRASE",
                "SQLCipher encryption passphrase",
            ),
        ],
    )
    .await
}
