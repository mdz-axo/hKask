//! hKask MCP Memory — Unified episodic + semantic memory MCP server (library).
//!
//! Exports MemoryServer struct and tool methods for fuzz testability (P5 Testing
//! Discipline, P4 Clear Boundaries). The binary entrypoint in main.rs delegates
//! to `run()`.
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

pub mod types;

use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_mcp::validate_field;
use hkask_memory::{EpisodicMemory, SemanticMemory};
use hkask_storage::Triple;
use hkask_types::time::now_rfc3339;
use hkask_types::{McpErrorKind, Visibility, WebID};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use types::*;

// ── Server ──────────────────────────────────────────────────────────

pub struct MemoryServer {
    pub episodic: EpisodicMemory,
    pub semantic: Arc<SemanticMemory>,
    pub db: Option<Arc<std::sync::Mutex<rusqlite::Connection>>>,
    pub webid: WebID,
    /// Replicant identity serving this MCP server (for narrative memory)
    pub replicant: String,
    /// Daemon client for dual-encoding experiences (None if daemon unavailable)
    pub daemon: Option<hkask_mcp::DaemonClient>,
}

impl MemoryServer {
    pub fn new(
        episodic: EpisodicMemory,
        semantic: Arc<SemanticMemory>,
        db: Option<Arc<std::sync::Mutex<rusqlite::Connection>>>,
        webid: WebID,
        replicant: String,
        daemon: Option<hkask_mcp::DaemonClient>,
    ) -> Self {
        Self {
            episodic,
            semantic,
            db,
            webid,
            replicant,
            daemon,
        }
    }

    fn internal_error(
        &self,
        span: ToolSpanGuard,
        context: &str,
        e: impl std::fmt::Display,
    ) -> String {
        hkask_mcp::tool_internal_error(span, context, e)
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
                        tracing::debug!(target: "hkask.mcp.memory.memory", tool = %tool_name, "Experience stored via daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.memory.memory", tool = %tool_name, response = ?other, "Unexpected daemon response")
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.memory.memory", tool = %tool_name, error = %e, "Failed to store experience")
                    }
                }
            });
        }
    }
}

#[tool_router(server_handler)]
impl MemoryServer {
    // ── Episodic tools ──────────────────────────────────────────

    #[tool(description = "Liveness and storage info for episodic memory")]
    pub async fn episodic_ping(&self) -> String {
        let span = ToolSpanGuard::new("episodic_ping", &self.webid);
        span.ok_json(json!({
            "status": "ok",
            "server": "hkask-mcp-memory",
            "perspective": self.webid.to_string(),
        }))
    }

    #[tool(description = "Store an episodic triple (private, perspective-bound)")]
    pub async fn episodic_store(
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
            Ok(()) => {
                self.record_experience(
                    "episodic_store",
                    &format!("{}:{}", entity, attribute),
                    "stored",
                    serde_json::json!({"entity": entity, "attribute": attribute}),
                );
                span.ok_json(json!({
                    "stored": true, "entity": entity, "attribute": attribute,
                }))
            }
            Err(e) => span.internal_error(
                json!({"error": format!("Failed to store episodic triple: {}", e)}),
            ),
        }
    }

    #[tool(description = "Recall episodic triples by entity (filtered by caller's WebID)")]
    pub async fn episodic_recall(
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
    pub async fn episodic_budget(&self, Parameters(_budget): Parameters<BudgetRequest>) -> String {
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
    pub async fn episodic_consolidate_status(
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
    pub async fn semantic_ping(&self) -> String {
        let span = ToolSpanGuard::new("semantic_ping", &self.webid);
        span.ok_json(json!({"status": "ok", "server": "hkask-mcp-memory"}))
    }

    #[tool(description = "Store a shared semantic triple (no perspective)")]
    pub async fn semantic_store(
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
                self.record_experience(
                    "semantic_store",
                    &format!("{}:{}", entity, attribute),
                    "stored",
                    serde_json::json!({"entity": entity, "attribute": attribute}),
                );
                span.ok_json(json!({"stored": true, "entity": entity, "attribute": attribute}))
            }
            Err(e) => self.internal_error(span, "store semantic triple", e),
        }
    }

    #[tool(description = "Recall shared semantic triples by entity")]
    pub async fn semantic_recall(
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
    pub async fn semantic_embed(
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
    pub async fn semantic_search(
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
                self.record_experience(
                    "semantic_search",
                    &format!("dim={}", query_vector.len()),
                    "success",
                    serde_json::json!({"count": serialized.len()}),
                );
                span.ok_json(json!({"count": serialized.len(), "results": serialized}))
            }
            Err(e) => self.internal_error(span, "search embeddings", e),
        }
    }

    #[tool(
        description = "Compute mean embedding vector (centroid) for embeddings matching a prefix"
    )]
    pub async fn semantic_centroid(
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
    pub async fn semantic_purge(
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
    pub async fn semantic_chunk(
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
    pub async fn semantic_count(&self, Parameters(_req): Parameters<CountRequest>) -> String {
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
    pub async fn memory_backup(
        &self,
        Parameters(BackupRequest {
            target_path,
            passphrase,
        }): Parameters<BackupRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("memory_backup", &self.webid);
        let target = target_path.unwrap_or_else(|| "hkask-memory-backup.sqlite".to_string());

        // \[NORMATIVE\] Refuse to write sovereign memory to an unencrypted file —
        // a plaintext backup defeats the SQLCipher at-rest encryption boundary
        // (P1 — User Sovereignty). A passphrase is mandatory.
        let Some(passphrase) = passphrase.filter(|p| !p.is_empty()) else {
            return self.internal_error(
                span,
                "backup",
                "a non-empty passphrase is required: refusing to write an unencrypted backup of sovereign memory",
            );
        };

        let Some(ref db_conn) = self.db else {
            return self.internal_error(span, "backup", "in-memory database");
        };

        // Open the destination and key it as a SQLCipher-encrypted database
        // BEFORE copying pages, so the backup is written encrypted.
        let mut dst_conn = match rusqlite::Connection::open(&target) {
            Ok(conn) => conn,
            Err(e) => {
                return self.internal_error(span, "open backup destination", e);
            }
        };
        if let Err(e) = dst_conn.pragma_update(None, "key", passphrase.as_str()) {
            return self.internal_error(span, "encrypt backup destination", e);
        }

        // Copy source → destination using SQLite's backup API (re-encrypts pages
        // under the destination key)
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
    pub async fn memory_restore(
        &self,
        Parameters(RestoreRequest {
            source_path,
            passphrase,
        }): Parameters<RestoreRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("memory_restore", &self.webid);

        let Some(ref db_conn) = self.db else {
            return self.internal_error(span, "restore", "in-memory database");
        };

        // Validate source file exists and is a SQLite database before destroying current data
        let src_conn = match rusqlite::Connection::open(&source_path) {
            Ok(conn) => {
                // Backups are written encrypted (see `memory_backup`); key the
                // source before reading so an encrypted backup can be restored.
                if let Some(p) = passphrase.as_deref().filter(|p| !p.is_empty())
                    && let Err(e) = conn.pragma_update(None, "key", p)
                {
                    return self.internal_error(span, "decrypt backup source", e);
                }
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

/// Run the memory MCP server (used by binary target).
pub async fn run(
    replicant: String,
    daemon_client: Option<hkask_mcp::DaemonClient>,
) -> Result<(), hkask_mcp::McpError> {
    hkask_mcp::run_server(
        "hkask-mcp-memory",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::server::ServerContext| {
            Ok((|| -> anyhow::Result<MemoryServer> {
                // Use the standard per-agent memory DB path when not explicitly set.
                // This ensures each agent's memory goes to agents/{name}/memory.db
                // alongside their pod.db, making the agent directory self-contained.
                let memory_db_path = ctx
                    .credentials
                    .get("HKASK_MEMORY_DB")
                    .cloned()
                    .unwrap_or_else(|| {
                        let default_path = hkask_types::agent_paths::agent_memory_db(&replicant);
                        if let Some(parent) = default_path.parent() {
                            std::fs::create_dir_all(parent).ok();
                        }
                        tracing::info!(
                            target: "hkask.mcp.memory",
                            path = %default_path.display(),
                            replicant = %replicant,
                            "Using default per-agent memory database"
                        );
                        default_path.to_string_lossy().to_string()
                    });
                let db = if let Some(passphrase) = ctx.credentials.get("HKASK_DB_PASSPHRASE") {
                    hkask_storage::Database::open(&memory_db_path, passphrase)
                        .map_err(|e| anyhow::anyhow!("{e}"))?
                } else {
                    hkask_storage::Database::in_memory().map_err(|e| anyhow::anyhow!("{e}"))?
                };
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
                    replicant.clone(),
                    daemon_client.clone(),
                ))
            })()?)
        },
        vec![
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_MEMORY_DB",
                "Path to per-agent memory database file (defaults to agents/{replicant}/memory.db)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_DB_PASSPHRASE",
                "SQLCipher encryption passphrase (resolved via hkask keystore chain when not set)",
            ),
        ],
    )
    .await
}

#[cfg(test)]
mod backup_encryption_tests {
    //! Verifies the at-rest encryption guarantee that `memory_backup` relies on:
    //! SQLCipher must be linked into this binary so that keying the destination
    //! connection actually encrypts the file. If SQLCipher were absent,
    //! `PRAGMA key` would be a silent no-op and backups would be plaintext.

    use std::time::Duration;

    #[test]
    fn keyed_backup_destination_is_unreadable_without_the_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let backup_path = dir.path().join("backup.sqlite");

        // Source: a keyed (encrypted) in-memory-style DB with one row.
        let src = rusqlite::Connection::open(dir.path().join("src.sqlite")).expect("open src");
        src.pragma_update(None, "key", "src-pass").expect("key src");
        src.execute_batch("CREATE TABLE t(x TEXT); INSERT INTO t VALUES ('secret');")
            .expect("seed src");

        // Destination keyed with a DIFFERENT passphrase, then page-copied — exactly
        // what `memory_backup` does.
        let mut dst = rusqlite::Connection::open(&backup_path).expect("open dst");
        dst.pragma_update(None, "key", "backup-pass")
            .expect("key dst");
        {
            let backup = rusqlite::backup::Backup::new(&src, &mut dst).expect("backup new");
            backup
                .run_to_completion(100, Duration::from_millis(250), None)
                .expect("backup run");
        }
        drop(dst);

        // Opening the backup WITHOUT the key must fail (proves it is encrypted).
        let no_key = rusqlite::Connection::open(&backup_path).expect("reopen");
        let unreadable = no_key
            .query_row("SELECT x FROM t", [], |r| r.get::<_, String>(0))
            .is_err();
        assert!(
            unreadable,
            "backup opened without key must be unreadable — SQLCipher not active? backup is PLAINTEXT"
        );

        // Opening WITH the correct key must succeed and round-trip the data.
        let keyed = rusqlite::Connection::open(&backup_path).expect("reopen keyed");
        keyed
            .pragma_update(None, "key", "backup-pass")
            .expect("key reopen");
        let value: String = keyed
            .query_row("SELECT x FROM t", [], |r| r.get(0))
            .expect("read with key");
        assert_eq!(value, "secret");
    }
}
