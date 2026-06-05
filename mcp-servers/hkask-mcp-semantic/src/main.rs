//! hKask MCP Semantic — Semantic memory store, recall, similarity search, text chunking, and consolidation
//!
//! 10 tools:
//! - `semantic_ping` — Liveness and storage info
//! - `semantic_store` — Store a shared semantic triple (no perspective)
//! - `semantic_recall` — Recall triples by entity (public, any agent can read)
//! - `semantic_embed` — Store an embedding vector for similarity search
//! - `semantic_search` — KNN similarity search over embeddings
//! - `semantic_centroid` — Compute mean embedding vector for a prefix-filtered set
//! - `semantic_purge` — Delete embeddings matching an entity_ref prefix
//! - `semantic_chunk` — Chunk text into passages for embedding
//! - `semantic_count` — Triple and embedding counts
//! - `semantic_consolidate` — Full consolidation: episodic→semantic promotion + semantic cleanup

use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_mcp::validate_field;
use hkask_memory::{ConsolidationBridge, ConsolidationService, EpisodicMemory, SemanticMemory};
use hkask_storage::Triple;
use hkask_types::loops::CuratorHandle;
use hkask_types::ports::{ConsolidationPort, ConsolidationRequest};
use hkask_types::{McpErrorKind, Visibility, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConsolidateRequest {
    pub limit: Option<usize>,
    pub confidence_floor: Option<f64>,
    pub max_semantic_triples: Option<usize>,
}

pub struct SemanticServer {
    memory: Arc<SemanticMemory>,
    /// Kept for ownership lifecycle — the bridge holds its own Arc<EpisodicMemory>,
    /// but this field ensures the original Arc stays alive for the server's lifetime.
    #[allow(dead_code)] // ownership guard for ConsolidationBridge's episodic Arc
    episodic: Arc<EpisodicMemory>,
    bridge: Arc<ConsolidationBridge>,
    webid: WebID,
}

impl SemanticServer {
    pub fn new(
        memory: Arc<SemanticMemory>,
        episodic: Arc<EpisodicMemory>,
        bridge: Arc<ConsolidationBridge>,
        webid: WebID,
    ) -> Self {
        Self {
            memory,
            episodic,
            bridge,
            webid,
        }
    }
}

#[tool_router(server_handler)]
impl SemanticServer {
    #[tool(description = "Liveness and storage info for semantic memory")]
    async fn semantic_ping(&self) -> String {
        let span = ToolSpanGuard::new("semantic_ping", &self.webid);
        span.ok_json(json!({
            "status": "ok",
            "server": "hkask-mcp-semantic",
        }))
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
            .with_visibility(Visibility::Shared)
            .with_confidence(confidence.unwrap_or(1.0));

        match self.memory.store(triple) {
            Ok(()) => span.ok_json(json!({
                "stored": true,
                "entity": entity,
                "attribute": attribute,
            })),
            Err(e) => span.internal_error(
                json!({"error": format!("Failed to store semantic triple: {}", e)}),
            ),
        }
    }

    #[tool(description = "Recall shared semantic triples by entity")]
    async fn semantic_recall(
        &self,
        Parameters(RecallRequest { entity }): Parameters<RecallRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("semantic_recall", &self.webid);

        validate_field!(span, "entity", &entity, 256);

        match self.memory.query_deduped(&entity) {
            Ok(triples) => {
                let serialized: Vec<serde_json::Value> = triples
                    .iter()
                    .map(|t| {
                        json!({
                            "entity": t.entity,
                            "attribute": t.attribute,
                            "value": t.value,
                            "confidence": t.confidence,
                            "valid_from": t.valid_from.to_rfc3339(),
                        })
                    })
                    .collect();
                span.ok_json(json!({
                    "count": serialized.len(),
                    "triples": serialized,
                }))
            }
            Err(e) => span.internal_error(
                json!({"error": format!("Failed to recall semantic triples: {}", e)}),
            ),
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

        match self.memory.store_embedding(&entity_ref, &vector, &model) {
            Ok(_id) => span.ok_json(json!({
                "stored": true,
                "entity_ref": entity_ref,
                "model": model,
                "dimensions": vector.len(),
            })),
            Err(e) => {
                span.internal_error(json!({"error": format!("Failed to store embedding: {}", e)}))
            }
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

        let limit = limit.unwrap_or(10);

        match self.memory.search_similar(&query_vector, limit) {
            Ok(results) => {
                let serialized: Vec<serde_json::Value> = results
                    .iter()
                    .map(|r| {
                        json!({
                            "entity_ref": r.embedding.entity_ref,
                            "model": r.embedding.model,
                            "distance": r.distance,
                        })
                    })
                    .collect();
                span.ok_json(json!({
                    "count": serialized.len(),
                    "results": serialized,
                }))
            }
            Err(e) => {
                span.internal_error(json!({"error": format!("Failed to search embeddings: {}", e)}))
            }
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

        match self.memory.compute_centroid(
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
            Err(e) => {
                span.internal_error(json!({"error": format!("Failed to compute centroid: {}", e)}))
            }
        }
    }

    #[tool(description = "Delete all embeddings whose entity_ref starts with a prefix")]
    async fn semantic_purge(
        &self,
        Parameters(PurgeRequest { prefix }): Parameters<PurgeRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("semantic_purge", &self.webid);

        validate_field!(span, "prefix", &prefix, 256);

        match self.memory.purge_by_prefix(&prefix) {
            Ok(count) => span.ok_json(json!({
                "purged": count,
                "prefix": prefix,
            })),
            Err(e) => {
                span.internal_error(json!({"error": format!("Failed to purge embeddings: {}", e)}))
            }
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

        if text.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("text must not be empty").to_json_string(),
            );
        }

        if entity_ref_prefix.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("entity_ref_prefix must not be empty")
                    .to_json_string(),
            );
        }

        validate_field!(span, "entity_ref_prefix", &entity_ref_prefix, 256);

        let min_w = min_words.unwrap_or(50);
        let max_w = max_words.unwrap_or(200);
        let boundary = sentence_boundary.unwrap_or_else(|| ".!? ".to_string());

        let processed = if strip_gutenberg.unwrap_or(false) {
            hkask_memory::SemanticMemory::strip_gutenberg_headers(&text)
        } else {
            text.clone()
        };

        let passages = hkask_memory::SemanticMemory::chunk_text(
            &processed,
            &entity_ref_prefix,
            min_w,
            max_w,
            &boundary,
        );

        let total_passages = passages.len();
        let serialized: Vec<serde_json::Value> = passages
            .into_iter()
            .map(|(entity_ref, passage_text)| {
                json!({
                    "entity_ref": entity_ref,
                    "text": passage_text,
                })
            })
            .collect();

        span.ok_json(json!({
            "total_passages": total_passages,
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
        let triple_count = match self.memory.triple_count() {
            Ok(c) => c,
            Err(e) => {
                return span
                    .internal_error(json!({"error": format!("Failed to count triples: {}", e)}));
            }
        };
        let embedding_count = match self.memory.embedding_count() {
            Ok(c) => c,
            Err(e) => {
                return span.internal_error(
                    json!({"error": format!("Failed to count embeddings: {}", e)}),
                );
            }
        };
        span.ok_json(json!({"triple_count": triple_count, "embedding_count": embedding_count}))
    }

    #[tool(
        description = "Full consolidation: episodic→semantic promotion + semantic cleanup (low-confidence deletion, max-triple enforcement)"
    )]
    async fn semantic_consolidate(
        &self,
        Parameters(ConsolidateRequest {
            limit,
            confidence_floor,
            max_semantic_triples,
        }): Parameters<ConsolidateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("semantic_consolidate", &self.webid);

        // NOTE: This MCP tool does NOT verify a master passphrase.
        // The MCP semantic server is always OCAP-gated — invocations require a
        // valid capability token issued through the GovernedTool membrane
        // (see hkask-mcp/src/dispatch.rs). Adding a passphrase check here
        // would create a redundant authorization layer: OCAP already ensures
        // that only token-bearing callers can invoke this tool. The CLI and
        // API endpoints are directly user-facing (no OCAP membrane), so they
        // need passphrase verification. This tool does not.

        // Issue a ConsolidationToken via the system CuratorHandle.
        // The MCP server is OCAP-gated, so the caller already has authority
        // to invoke consolidation. The token proves this to ConsolidationPort.
        let handle = CuratorHandle::system();
        let token = handle.issue_consolidation_token();

        // Build ConsolidationService wrapping the bridge + semantic memory + token
        let service = ConsolidationService::new(
            Arc::clone(&self.bridge) as Arc<dyn ConsolidationPort>,
            Arc::clone(&self.memory),
            token,
        );

        let perspective = handle.curator_id().clone();
        let limit = limit.unwrap_or(100);

        let request = ConsolidationRequest {
            limit,
            confidence_floor,
            max_semantic_triples,
        };

        // ConsolidationPort::consolidate is sync — call it from this async context
        let outcome = match service.consolidate(&perspective, request) {
            Ok(outcome) => outcome,
            Err(e) => {
                return span.internal_error(json!({
                    "error": format!("Consolidation failed: {}", e)
                }));
            }
        };

        let final_count = self.memory.triple_count().unwrap_or(0);

        span.ok_json(json!({
            "consolidated_count": outcome.consolidated_count,
            "deleted_count": outcome.deleted_count,
            "failed_count": outcome.failed_count,
            "semantic_triple_count": final_count,
        }))
    }
}

hkask_mcp::mcp_server_main!(
    "hkask-mcp-semantic",
    factory: |ctx: hkask_mcp::ServerContext| {
        let db_path = ctx.credentials.get("HKASK_MEMORY_DB")
            .ok_or_else(|| anyhow::anyhow!("Missing HKASK_MEMORY_DB"))?
            .clone();
        let passphrase = ctx.credentials.get("HKASK_DB_PASSPHRASE")
            .ok_or_else(|| anyhow::anyhow!("Missing HKASK_DB_PASSPHRASE"))?
            .clone();
        let db = hkask_storage::Database::open(&db_path, &passphrase)
            .map_err(|e| anyhow::anyhow!("Failed to open memory database: {}", e))?;
        let conn = db.conn_arc();
        // Episodic memory from the same per-agent DB
        let ts_episodic = hkask_storage::TripleStore::new(Arc::clone(&conn));
        let episodic = Arc::new(hkask_memory::EpisodicMemory::new(ts_episodic));
        // Semantic memory from the same per-agent DB
        let ts_semantic = hkask_storage::TripleStore::new(Arc::clone(&conn));
        let embedding_store = hkask_storage::EmbeddingStore::new(conn);
        let memory = Arc::new(hkask_memory::SemanticMemory::new(ts_semantic, embedding_store));
        // Consolidation bridge from shared episodic + semantic
        let bridge = Arc::new(hkask_memory::ConsolidationBridge::new(
            Arc::clone(&episodic),
            Arc::clone(&memory),
        ));
        Ok(SemanticServer::new(memory, episodic, bridge, ctx.webid))
    },
    credentials: vec![
        hkask_mcp::CredentialRequirement::required("HKASK_MEMORY_DB", "Path to per-agent memory database file (episodic + semantic)"),
        hkask_mcp::CredentialRequirement::required("HKASK_DB_PASSPHRASE", "SQLCipher encryption passphrase"),
    ]
);
