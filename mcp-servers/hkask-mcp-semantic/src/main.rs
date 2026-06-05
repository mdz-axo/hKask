//! hKask MCP Semantic — Semantic memory store, recall, and similarity search
//!
//! 6 tools:
//! - `semantic_ping` — Liveness and storage info
//! - `semantic_store` — Store a shared semantic triple (no perspective)
//! - `semantic_recall` — Recall triples by entity (public, any agent can read)
//! - `semantic_embed` — Store an embedding vector for similarity search
//! - `semantic_search` — KNN similarity search over embeddings
//! - `semantic_count` — Triple and embedding counts
//!
//! **Consolidation NOT exposed:** The Episodic → Semantic consolidation bridge
//! requires a `ConsolidationToken` issued by the Curation Loop. MCP servers
//! cannot mint this token.

use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_mcp::validate_field;
use hkask_memory::SemanticMemory;
use hkask_storage::Triple;
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
pub struct CountRequest {}

pub struct SemanticServer {
    memory: SemanticMemory,
    webid: WebID,
}

impl SemanticServer {
    pub fn new(memory: SemanticMemory, webid: WebID) -> Self {
        Self { memory, webid }
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
                return span
                    .internal_error(json!({"error": format!("Failed to count embeddings: {}", e)}));
            }
        };
        span.ok_json(json!({"triple_count": triple_count, "embedding_count": embedding_count}))
    }
}

hkask_mcp::mcp_server_main!(
    "hkask-mcp-semantic",
    factory: |ctx: hkask_mcp::ServerContext| {
        let db_path = ctx.credentials.get("HKASK_SEMANTIC_DB")
            .ok_or_else(|| anyhow::anyhow!("Missing HKASK_SEMANTIC_DB"))?
            .clone();
        let passphrase = ctx.credentials.get("HKASK_DB_PASSPHRASE")
            .ok_or_else(|| anyhow::anyhow!("Missing HKASK_DB_PASSPHRASE"))?
            .clone();
        let db = hkask_storage::Database::open(&db_path, &passphrase)
            .map_err(|e| anyhow::anyhow!("Failed to open semantic database: {}", e))?;
        let conn = db.conn_arc();
        let triple_store = hkask_storage::TripleStore::new(Arc::clone(&conn));
        let embedding_store = hkask_storage::EmbeddingStore::new(conn);
        let memory = hkask_memory::SemanticMemory::new(triple_store, embedding_store);
        Ok(SemanticServer::new(memory, ctx.webid))
    },
    credentials: vec![
        hkask_mcp::CredentialRequirement::required("HKASK_SEMANTIC_DB", "Path to semantic database file"),
        hkask_mcp::CredentialRequirement::required("HKASK_DB_PASSPHRASE", "SQLCipher encryption passphrase"),
    ]
);
