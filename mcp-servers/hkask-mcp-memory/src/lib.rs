//! hKask MCP Memory — Unified episodic + semantic memory MCP server (library).
//!
//! Exports MemoryServer struct and tool methods for fuzz testability (P5 Testing
//! Discipline, P4 Clear Boundaries). The binary entrypoint in main.rs delegates
//! to `run()`.
//!
//! 18 tools:
//! - `episodic_ping` — Liveness and storage info for episodic memory
//! - `episodic_store` — Store an episodic triple (private, perspective-bound)
//! - `episodic_recall` — Recall triples by entity (filtered by caller's WebID)
//! - `episodic_recall_context` — Recall episodes ranked by salience to context (mirrors ChatService::recall_episodic)
//! - `episodic_budget` — Storage usage and budget info
//! - `episodic_consolidate_status` — Check consolidation candidates and budget status
//! - `semantic_ping` — Liveness and storage info for semantic memory
//! - `semantic_store` — Store a shared semantic triple (no perspective)
//! - `semantic_recall` — Recall triples by entity (public, any agent can read)
//! - `memory_recall` — Paired semantic + episodic recall, mirrored dual-recall circuit
//! - `semantic_embed` — Store an embedding vector for similarity search
//! - `semantic_search` — KNN similarity search over embeddings
//! - `semantic_centroid` — Compute mean embedding vector for a prefix-filtered set
//! - `semantic_purge` — Delete embeddings matching an entity_ref prefix
//! - `semantic_chunk` — Chunk text into passages for embedding
//! - `semantic_count` — Triple and embedding counts
//! - `memory_backup` — Export the memory database to a local backup file
//! - `memory_restore` — Restore the memory database from a local backup file

pub mod cogat;
pub mod types;

// Bridge crates: shared ontological vocabulary (P5.4 dual-axis framework)

use hkask_mcp::server::{McpToolError, execute_tool};
use hkask_mcp::validate_identifier;
use hkask_memory::{EpisodicMemory, SemanticMemory};
use hkask_storage::Triple;
use hkask_types::{Visibility, WebID};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use types::RecallContextRequest;
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
}

impl hkask_mcp::server::ToolContext for MemoryServer {
    fn webid(&self) -> &WebID {
        &self.webid
    }

    fn record_tool_outcome(&self, tool: &str, outcome: &str) {
        hkask_mcp::record_via_daemon(&self.daemon, &self.replicant, tool, outcome);
    }
}

#[tool_router(server_handler)]
impl MemoryServer {
    // ── Episodic tools ──────────────────────────────────────────

    #[tool(description = "Liveness and storage info for episodic memory")]
    pub async fn episodic_ping(&self) -> String {
        execute_tool(self, "episodic_ping", async {
            Ok(json!({
                "status": "ok",
                "server": "hkask-mcp-memory",
                "perspective": self.webid.to_string(),
            }))
        })
        .await
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
        execute_tool(self, "episodic_store", async {
            validate_identifier("entity", &entity, 256)?;
            validate_identifier("attribute", &attribute, 256)?;
            let triple = Triple::new(&entity, &attribute, value, self.webid)
                .with_perspective(self.webid)
                .with_confidence(confidence.unwrap_or(1.0))
                .with_visibility(Visibility::Private);
            self.episodic
                .store(triple)
                .map_err(|e| McpToolError::internal(format!("store episodic triple: {}", e)))?;
            Ok(json!({
                "stored": true, "entity": entity, "attribute": attribute,
            }))
        })
        .await
    }

    #[tool(description = "Recall episodic triples by entity (filtered by caller's WebID)")]
    pub async fn episodic_recall(
        &self,
        Parameters(RecallRequest { entity }): Parameters<RecallRequest>,
    ) -> String {
        execute_tool(self, "episodic_recall", async {
            validate_identifier("entity", &entity, 256)?;
            let triples = self
                .episodic
                .query_for_deduped(&entity, self.webid)
                .map_err(|e| McpToolError::internal(format!("recall episodic triples: {}", e)))?;
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
            Ok(json!({"count": serialized.len(), "triples": serialized}))
        })
        .await
    }

    #[tool(
        description = "Recall episodic memories ranked by salience to context. \
        Returns formatted episodes (User:/Agent: pairs for chat history) sorted by keyword relevance. \
        Mirrors ChatService::recall_episodic — use this when you need relevant past interactions, \
        not just entity-matched triples."
    )]
    pub async fn episodic_recall_context(
        &self,
        Parameters(RecallContextRequest {
            entity,
            context,
            limit,
        }): Parameters<RecallContextRequest>,
    ) -> String {
        execute_tool(self, "episodic_recall_context", async {
            validate_identifier("entity", &entity, 256)?;
            let limit = limit.unwrap_or(10);

            let triples = self
                .episodic
                .query_for_deduped(&entity, self.webid)
                .map_err(|e| McpToolError::internal(format!("recall episodic triples: {}", e)))?;

            if triples.is_empty() {
                return Ok(json!({"count": 0, "episodes": []}));
            }

            if let Some(ref ctx) = context {
                // Salience-scored: build keywords from context, score each episode
                let ctx_lower = ctx.to_lowercase();
                let keywords: Vec<&str> = ctx_lower
                    .split_whitespace()
                    .filter(|w| w.len() > 2)
                    .collect();

                let mut scored: Vec<(usize, serde_json::Value)> = triples
                    .iter()
                    .filter_map(|t| {
                        let v = t.value.as_object()?;
                        let ui = v.get("user_input")?.as_str()?;
                        let ar = v.get("agent_response")?.as_str()?;
                        let combined = format!("{} {}", ui.to_lowercase(), ar.to_lowercase());
                        let score = keywords.iter().filter(|kw| combined.contains(*kw)).count();
                        Some((
                            score,
                            json!({
                                "user_input": ui,
                                "agent_response": ar,
                                "salience": score,
                                "confidence": t.confidence,
                                "valid_from": t.temporal.valid_from.to_rfc3339(),
                            }),
                        ))
                    })
                    .collect();

                scored.sort_by(|a, b| b.0.cmp(&a.0));
                let episodes: Vec<serde_json::Value> =
                    scored.into_iter().take(limit).map(|(_, v)| v).collect();

                Ok(json!({
                    "count": episodes.len(),
                    "context": ctx,
                    "episodes": episodes,
                }))
            } else {
                // No context: return most recent episodes, sorted by recency (reverse order)
                let episodes: Vec<serde_json::Value> = triples
                    .iter()
                    .rev()
                    .take(limit)
                    .filter_map(|t| {
                        let v = t.value.as_object()?;
                        let ui = v.get("user_input")?.as_str()?;
                        let ar = v.get("agent_response")?.as_str()?;
                        Some(json!({
                            "user_input": ui,
                            "agent_response": ar,
                            "confidence": t.confidence,
                            "valid_from": t.temporal.valid_from.to_rfc3339(),
                        }))
                    })
                    .collect();

                Ok(json!({
                    "count": episodes.len(),
                    "episodes": episodes,
                }))
            }
        })
        .await
    }

    #[tool(description = "Storage usage and budget for episodic memory")]
    pub async fn episodic_budget(&self, Parameters(_budget): Parameters<BudgetRequest>) -> String {
        execute_tool(self, "episodic_budget", async {
            let usage = self
                .episodic
                .storage_usage(&self.webid)
                .map_err(|e| McpToolError::internal(format!("storage usage: {}", e)))?;
            let budget = self.episodic.storage_budget();
            let remaining = budget.saturating_sub(usage);
            Ok(json!({"used": usage, "budget": budget, "remaining": remaining}))
        })
        .await
    }

    #[tool(
        description = "Check consolidation candidates and budget status for episodic→semantic promotion"
    )]
    pub async fn episodic_consolidate_status(
        &self,
        Parameters(_req): Parameters<ConsolidateStatusRequest>,
    ) -> String {
        execute_tool(self, "episodic_consolidate_status", async {
            let candidate_count = self.episodic.consolidation_candidate_count(&self.webid);
            let usage = self
                .episodic
                .storage_usage(&self.webid)
                .map_err(|e| McpToolError::internal(format!("storage usage: {}", e)))?;
            let budget = self.episodic.storage_budget();
            let over_budget = usage > budget;
            Ok(json!({
                "consolidation_candidates": candidate_count,
                "episodic_usage": usage,
                "episodic_budget": budget,
                "over_budget": over_budget,
            }))
        })
        .await
    }

    // ── Semantic tools ──────────────────────────────────────────

    #[tool(description = "Liveness and storage info for semantic memory")]
    pub async fn semantic_ping(&self) -> String {
        execute_tool(self, "semantic_ping", async {
            Ok(json!({"status": "ok", "server": "hkask-mcp-memory"}))
        })
        .await
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
        execute_tool(self, "semantic_store", async {
            validate_identifier("entity", &entity, 256)?;
            validate_identifier("attribute", &attribute, 256)?;
            let triple = Triple::new(&entity, &attribute, value, self.webid)
                .with_visibility(Visibility::Public)
                .with_confidence(confidence.unwrap_or(1.0));
            self.semantic
                .store(triple)
                .map_err(|e| McpToolError::internal(format!("store semantic triple: {}", e)))?;
            Ok(json!({"stored": true, "entity": entity, "attribute": attribute}))
        })
        .await
    }

    #[tool(description = "Recall shared semantic triples by entity")]
    pub async fn semantic_recall(
        &self,
        Parameters(RecallRequest { entity }): Parameters<RecallRequest>,
    ) -> String {
        execute_tool(self, "semantic_recall", async {
            validate_identifier("entity", &entity, 256)?;
            let triples = self
                .semantic
                .query_deduped(&entity)
                .map_err(|e| McpToolError::internal(format!("recall semantic triples: {}", e)))?;
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
            Ok(json!({"count": serialized.len(), "triples": serialized}))
        })
        .await
    }

    // ── FlowDef dispatch tools — route by memory_type ───────────────────

    #[tool(
        description = "Store a memory triple — routes to episodic_store or semantic_store based on memory_type"
    )]
    pub async fn remember(
        &self,
        Parameters(MemoryDispatchRequest {
            entity,
            attribute,
            value,
            confidence,
            memory_type,
        }): Parameters<MemoryDispatchRequest>,
    ) -> String {
        execute_tool(self, "remember", async {
            match memory_type.as_str() {
                "semantic" => {
                    validate_identifier("entity", &entity, 256)?;
                    validate_identifier("attribute", &attribute, 256)?;
                    let triple = Triple::new(&entity, &attribute, value, self.webid)
                        .with_visibility(Visibility::Public)
                        .with_confidence(confidence.unwrap_or(1.0));
                    self.semantic.store(triple).map_err(|e| {
                        McpToolError::internal(format!("store semantic triple: {}", e))
                    })?;
                    Ok(json!({"stored": true, "entity": entity, "attribute": attribute, "memory_type": "semantic"}))
                }
                _ => {
                    // Default: episodic
                    validate_identifier("entity", &entity, 256)?;
                    validate_identifier("attribute", &attribute, 256)?;
                    let triple = Triple::new(&entity, &attribute, value, self.webid)
                        .with_perspective(self.webid)
                        .with_confidence(confidence.unwrap_or(1.0))
                        .with_visibility(Visibility::Private);
                    self.episodic.store(triple).map_err(|e| {
                        McpToolError::internal(format!("store episodic triple: {}", e))
                    })?;
                    Ok(json!({"stored": true, "entity": entity, "attribute": attribute, "memory_type": "episodic"}))
                }
            }
        })
        .await
    }

    #[tool(description = "Recall memory triples by entity — routes based on memory_type")]
    pub async fn recall(
        &self,
        Parameters(RecallDispatchRequest {
            entity,
            memory_type,
        }): Parameters<RecallDispatchRequest>,
    ) -> String {
        execute_tool(self, "recall", async {
            match memory_type.as_str() {
                "semantic" => {
                    validate_identifier("entity", &entity, 256)?;
                    let triples = self.semantic.query_deduped(&entity).map_err(|e| {
                        McpToolError::internal(format!("recall semantic triples: {}", e))
                    })?;
                    let serialized: Vec<serde_json::Value> = triples.iter().map(|t| json!({
                        "entity": t.entity, "attribute": t.attribute, "value": t.value,
                        "confidence": t.confidence, "valid_from": t.temporal.valid_from.to_rfc3339(),
                    })).collect();
                    Ok(json!({"count": serialized.len(), "triples": serialized, "memory_type": "semantic"}))
                }
                _ => {
                    validate_identifier("entity", &entity, 256)?;
                    let triples = self.episodic.query_for_deduped(&entity, self.webid).map_err(|e| {
                        McpToolError::internal(format!("recall episodic triples: {}", e))
                    })?;
                    let serialized: Vec<serde_json::Value> = triples.iter().map(|t| json!({
                        "entity": t.entity, "attribute": t.attribute, "value": t.value,
                        "confidence": t.confidence, "valid_from": t.temporal.valid_from.to_rfc3339(),
                    })).collect();
                    Ok(json!({"count": serialized.len(), "triples": serialized, "memory_type": "episodic"}))
                }
            }
        })
        .await
    }

    #[tool(
        description = "Paired memory recall — returns both semantic (third-person) and \
        episodic (first-person) memories for an entity in a single call. Episodic results \
        are ranked by salience when context is provided. Use this as the primary memory \
        recall tool — it mirrors the dual-recall circuit in ChatService::prepare_chat."
    )]
    pub async fn memory_recall(
        &self,
        Parameters(PairedRecallRequest {
            entity,
            context,
            limit,
        }): Parameters<PairedRecallRequest>,
    ) -> String {
        execute_tool(self, "memory_recall", async {
            validate_identifier("entity", &entity, 256)?;
            let limit = limit.unwrap_or(10);

            // ── Semantic recall (third-person facts, no personal filter) ──
            let semantic_triples = self
                .semantic
                .query_deduped(&entity)
                .map_err(|e| McpToolError::internal(format!("recall semantic memory: {}", e)))?;
            let semantic: Vec<_> = semantic_triples
                .iter()
                .take(limit)
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

            // ── Episodic recall (first-person, filtered by caller's WebID) ──
            let episodic_triples = self
                .episodic
                .query_for_deduped(&entity, self.webid)
                .map_err(|e| McpToolError::internal(format!("recall episodic memory: {}", e)))?;

            if episodic_triples.is_empty() {
                return Ok(json!({
                    "entity": entity,
                    "semantic": { "count": semantic.len(), "triples": semantic },
                    "episodic": { "count": 0, "episodes": [] },
                }));
            }

            let episodic = if let Some(ref ctx) = context {
                // Salience-scored episodic recall (mirrors ChatService::recall_episodic)
                let ctx_lower = ctx.to_lowercase();
                let keywords: Vec<&str> = ctx_lower
                    .split_whitespace()
                    .filter(|w| w.len() > 2)
                    .collect();

                let mut scored: Vec<(usize, serde_json::Value)> = episodic_triples
                    .iter()
                    .filter_map(|t| {
                        let v = t.value.as_object()?;
                        let ui = v.get("user_input")?.as_str()?;
                        let ar = v.get("agent_response")?.as_str()?;
                        let combined = format!("{} {}", ui.to_lowercase(), ar.to_lowercase());
                        let score = keywords.iter().filter(|kw| combined.contains(*kw)).count();
                        Some((
                            score,
                            json!({
                                "user_input": ui,
                                "agent_response": ar,
                                "salience": score,
                                "confidence": t.confidence,
                                "valid_from": t.temporal.valid_from.to_rfc3339(),
                            }),
                        ))
                    })
                    .collect();
                scored.sort_by(|a, b| b.0.cmp(&a.0));
                scored
                    .into_iter()
                    .take(limit)
                    .map(|(_, v)| v)
                    .collect::<Vec<_>>()
            } else {
                // No context: most recent by recency
                episodic_triples
                    .iter()
                    .rev()
                    .take(limit)
                    .filter_map(|t| {
                        let v = t.value.as_object()?;
                        let ui = v.get("user_input")?.as_str()?;
                        let ar = v.get("agent_response")?.as_str()?;
                        Some(json!({
                            "user_input": ui,
                            "agent_response": ar,
                            "confidence": t.confidence,
                            "valid_from": t.temporal.valid_from.to_rfc3339(),
                        }))
                    })
                    .collect::<Vec<_>>()
            };

            Ok(json!({
                "entity": entity,
                "semantic": { "count": semantic.len(), "triples": semantic },
                "episodic": { "count": episodic.len(), "episodes": episodic },
            }))
        })
        .await
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
        execute_tool(self, "semantic_embed", async {
            validate_identifier("entity_ref", &entity_ref, 256)?;
            if vector.is_empty() {
                return Err(McpToolError::invalid_argument("vector must not be empty"));
            }
            self.semantic
                .store_embedding(&entity_ref, &vector, &model)
                .map_err(|e| McpToolError::internal(format!("store embedding: {}", e)))?;
            Ok(json!({
                "stored": true,
                "entity_ref": entity_ref,
                "model": model,
                "dimensions": vector.len(),
            }))
        })
        .await
    }

    #[tool(description = "KNN similarity search over embeddings")]
    pub async fn semantic_search(
        &self,
        Parameters(SearchRequest {
            query_vector,
            limit,
        }): Parameters<SearchRequest>,
    ) -> String {
        execute_tool(self, "semantic_search", async {
            if query_vector.is_empty() {
                return Err(McpToolError::invalid_argument(
                    "query_vector must not be empty",
                ));
            }
            let results = self
                .semantic
                .search_similar(&query_vector, limit.unwrap_or(10))
                .map_err(|e| McpToolError::internal(format!("search embeddings: {}", e)))?;
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
            Ok(json!({"count": serialized.len(), "results": serialized}))
        })
        .await
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
        execute_tool(self, "semantic_centroid", async {
            validate_identifier("prefix", &prefix, 256)?;
            validate_identifier("exclude_prefix", &exclude_prefix, 256)?;
            validate_identifier("exclude_ref", &exclude_ref, 256)?;
            if dim == 0 {
                return Err(McpToolError::invalid_argument("dim must be positive"));
            }
            let result = self
                .semantic
                .compute_centroid(
                    &prefix,
                    &exclude_prefix,
                    &exclude_ref,
                    dim,
                    store_as.as_deref(),
                    model.as_deref(),
                )
                .map_err(|e| McpToolError::internal(format!("compute centroid: {}", e)))?;
            Ok(json!({
                "centroid": result.centroid,
                "dimensions": result.centroid.len(),
                "prefix": prefix,
                "passage_count": result.passage_count,
                "stored": result.stored,
            }))
        })
        .await
    }

    #[tool(description = "Delete all embeddings whose entity_ref starts with a prefix")]
    pub async fn semantic_purge(
        &self,
        Parameters(PurgeRequest { prefix }): Parameters<PurgeRequest>,
    ) -> String {
        execute_tool(self, "semantic_purge", async {
            validate_identifier("prefix", &prefix, 256)?;
            let count = self
                .semantic
                .purge_by_prefix(&prefix)
                .map_err(|e| McpToolError::internal(format!("purge embeddings: {}", e)))?;
            Ok(json!({"purged": count, "prefix": prefix}))
        })
        .await
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
        execute_tool(self, "semantic_chunk", async {
            if text.is_empty() || entity_ref_prefix.is_empty() {
                let field = if text.is_empty() {
                    "text"
                } else {
                    "entity_ref_prefix"
                };
                return Err(McpToolError::invalid_argument(format!(
                    "{field} must not be empty"
                )));
            }
            validate_identifier("entity_ref_prefix", &entity_ref_prefix, 256)?;
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
            Ok(json!({
                "total_passages": serialized.len(),
                "passages": serialized,
                "min_words": min_w,
                "max_words": max_w,
                "sentence_boundary": boundary,
                "stripped_gutenberg": strip_gutenberg.unwrap_or(false),
            }))
        })
        .await
    }

    #[tool(description = "Triple and embedding counts for semantic memory")]
    pub async fn semantic_count(&self, Parameters(_req): Parameters<CountRequest>) -> String {
        execute_tool(self, "semantic_count", async {
            let triple_count = self
                .semantic
                .triple_count()
                .map_err(|e| McpToolError::internal(format!("count triples: {}", e)))?;
            let embedding_count = self
                .semantic
                .embedding_count()
                .map_err(|e| McpToolError::internal(format!("count embeddings: {}", e)))?;
            Ok(json!({"triple_count": triple_count, "embedding_count": embedding_count}))
        })
        .await
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
        execute_tool(self, "memory_backup", async {
            let target =
                target_path.unwrap_or_else(|| "hkask-memory-backup.sqlite".to_string());

            // [NORMATIVE] Refuse to write sovereign memory to an unencrypted file —
            // a plaintext backup defeats the SQLCipher at-rest encryption boundary
            // (P1 — User Sovereignty). A passphrase is mandatory.
            let Some(passphrase) = passphrase.filter(|p| !p.is_empty()) else {
                return Err(McpToolError::internal(
                    "backup: a non-empty passphrase is required: refusing to write an unencrypted backup of sovereign memory",
                ));
            };

            let Some(ref db_conn) = self.db else {
                return Err(McpToolError::internal("backup: in-memory database"));
            };

            // Open the destination and key it as a SQLCipher-encrypted database
            // BEFORE copying pages, so the backup is written encrypted.
            let mut dst_conn = rusqlite::Connection::open(&target)
                .map_err(|e| McpToolError::internal(format!("open backup destination: {}", e)))?;
            dst_conn
                .pragma_update(None, "key", passphrase.as_str())
                .map_err(|e| McpToolError::internal(format!("encrypt backup destination: {}", e)))?;

            // Copy source → destination using SQLite's backup API (re-encrypts pages
            // under the destination key)
            let src_conn = db_conn
                .lock()
                .map_err(|_| McpToolError::internal("backup: lock poisoned"))?;
            let result = rusqlite::backup::Backup::new(&src_conn, &mut dst_conn)
                .map_err(|e| format!("Backup setup failed: {}", e))
                .and_then(|backup| {
                    backup
                        .run_to_completion(100, Duration::from_millis(250), None)
                        .map_err(|e| format!("Backup failed: {}", e))
                });

            match result {
                Ok(()) => Ok(json!({
                    "backed_up": true,
                    "target_path": target,
                })),
                Err(e) => Err(McpToolError::internal(format!("backup: {}", e))),
            }
        })
        .await
    }

    #[tool(description = "Restore the memory database from a local backup file")]
    pub async fn memory_restore(
        &self,
        Parameters(RestoreRequest {
            source_path,
            passphrase,
        }): Parameters<RestoreRequest>,
    ) -> String {
        execute_tool(self, "memory_restore", async {
            let Some(ref db_conn) = self.db else {
                return Err(McpToolError::internal("restore: in-memory database"));
            };

            // Validate source file exists and is a SQLite database before destroying current data
            let src_conn = match rusqlite::Connection::open(&source_path) {
                Ok(conn) => {
                    // Backups are written encrypted (see `memory_backup`); key the
                    // source before reading so an encrypted backup can be restored.
                    if let Some(p) = passphrase.as_deref().filter(|p| !p.is_empty())
                        && let Err(e) = conn.pragma_update(None, "key", p)
                    {
                        return Err(McpToolError::internal(format!(
                            "decrypt backup source: {}",
                            e
                        )));
                    }
                    // Quick validation: try reading sqlite_master
                    if let Err(e) = conn.query_row(
                        "SELECT count(*) FROM sqlite_master WHERE type='table'",
                        [],
                        |row| row.get::<_, i64>(0),
                    ) {
                        return Err(McpToolError::internal(format!(
                            "validate backup source: Not a valid SQLite database: {}",
                            e
                        )));
                    }
                    conn
                }
                Err(e) => {
                    return Err(McpToolError::internal(format!("open backup source: {}", e)));
                }
            };

            // Clear current database, then copy backup → current
            let mut dst_conn = db_conn
                .lock()
                .map_err(|_| McpToolError::internal("restore: lock poisoned"))?;
            dst_conn
                .execute_batch(
                    "PRAGMA writable_schema = 1; \
                     DELETE FROM sqlite_master WHERE type IN ('table', 'index', 'trigger'); \
                     PRAGMA writable_schema = 0;",
                )
                .map_err(|e| McpToolError::internal(format!("Failed to clear existing data: {}", e)))?;

            let result = rusqlite::backup::Backup::new(&src_conn, &mut dst_conn)
                .map_err(|e| format!("Restore setup failed: {}", e))
                .and_then(|backup| {
                    backup
                        .run_to_completion(100, Duration::from_millis(250), None)
                        .map_err(|e| format!("Restore failed: {}", e))
                });

            match result {
                Ok(()) => Ok(json!({
                    "restored": true,
                    "source_path": source_path,
                    "warning": "Memory restored. Restart the MCP server for full consistency across all connections.",
                })),
                Err(e) => Err(McpToolError::internal(format!("restore: {}", e))),
            }
        })
        .await
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
                    hkask_storage::open_or_repair(&memory_db_path, passphrase)
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
