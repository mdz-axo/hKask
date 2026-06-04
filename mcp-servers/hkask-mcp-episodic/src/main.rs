//! hKask MCP Episodic — Episodic memory store and recall
//!
//! 4 tools:
//! - `episodic_ping` — Liveness and storage info
//! - `episodic_store` — Store an episodic triple (private, perspective-bound)
//! - `episodic_recall` — Recall triples by entity (filtered by caller's WebID)
//! - `episodic_budget` — Storage usage and budget info
//!
//! **Sovereignty:** All operations use the calling agent's `WebID` as the
//! `perspective`. An agent cannot read another agent's episodic memory.
//!
//! **Gap:** `episodic_retract` is not exposed in v1. `retract_triple()` is
//! `pub(crate)` in `hkask-memory` — membrane-sealed. Retraction requires
//! routing through `EpisodicLoop::act()`, which is not available in the MCP
//! server context.

use hkask_mcp::server::{McpToolError, McpToolOutput, ToolSpanGuard, validate_identifier};
use hkask_memory::EpisodicMemory;
use hkask_storage::Triple;
use hkask_types::{McpErrorKind, Visibility, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

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
pub struct BudgetRequest {}

pub struct EpisodicServer {
    memory: EpisodicMemory,
    webid: WebID,
}

impl EpisodicServer {
    pub fn new(memory: EpisodicMemory, webid: WebID) -> Self {
        Self { memory, webid }
    }
}

#[tool_router(server_handler)]
impl EpisodicServer {
    #[tool(description = "Liveness and storage info for episodic memory")]
    async fn episodic_ping(&self) -> String {
        let span = ToolSpanGuard::new("episodic_ping", &self.webid);
        span.ok(
            McpToolOutput::new(json!({
                "status": "ok",
                "server": "hkask-mcp-episodic",
                "perspective": self.webid.to_string(),
            }))
            .to_json_string(),
        )
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

        if let Err(e) = validate_identifier("entity", &entity, 256) {
            return span.error(McpErrorKind::InvalidArgument, e.to_json_string());
        }
        if let Err(e) = validate_identifier("attribute", &attribute, 256) {
            return span.error(McpErrorKind::InvalidArgument, e.to_json_string());
        }

        let triple = Triple::new(&entity, &attribute, value, self.webid)
            .with_perspective(self.webid)
            .with_confidence(confidence.unwrap_or(1.0))
            .with_visibility(Visibility::Private);

        match self.memory.store(triple) {
            Ok(()) => span.ok(
                McpToolOutput::new(json!({
                    "stored": true,
                    "entity": entity,
                    "attribute": attribute,
                }))
                .to_json_string(),
            ),
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("Failed to store episodic triple: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(description = "Recall episodic triples by entity (filtered by caller's WebID)")]
    async fn episodic_recall(
        &self,
        Parameters(RecallRequest { entity }): Parameters<RecallRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("episodic_recall", &self.webid);

        if let Err(e) = validate_identifier("entity", &entity, 256) {
            return span.error(McpErrorKind::InvalidArgument, e.to_json_string());
        }

        match self.memory.query_for_deduped(&entity, self.webid) {
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
                span.ok(
                    McpToolOutput::new(json!({
                        "count": serialized.len(),
                        "triples": serialized,
                    }))
                    .to_json_string(),
                )
            }
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("Failed to recall episodic triples: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(description = "Storage usage and budget for episodic memory")]
    async fn episodic_budget(&self, Parameters(_budget): Parameters<BudgetRequest>) -> String {
        let span = ToolSpanGuard::new("episodic_budget", &self.webid);

        let usage = match self.memory.storage_usage(&self.webid) {
            Ok(u) => u,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Failed to query storage usage: {}", e))
                        .to_json_string(),
                );
            }
        };
        let budget = self.memory.storage_budget();
        let remaining = budget.saturating_sub(usage);

        span.ok(
            McpToolOutput::new(json!({
                "used": usage,
                "budget": budget,
                "remaining": remaining,
            }))
            .to_json_string(),
        )
    }
}

hkask_mcp::mcp_server_main!(
    "hkask-mcp-episodic",
    factory: |ctx: hkask_mcp::ServerContext| {
        let db_path = ctx.credentials.get("HKASK_EPISODIC_DB")
            .ok_or_else(|| anyhow::anyhow!("Missing HKASK_EPISODIC_DB"))?
            .clone();
        let passphrase = ctx.credentials.get("HKASK_DB_PASSPHRASE")
            .ok_or_else(|| anyhow::anyhow!("Missing HKASK_DB_PASSPHRASE"))?
            .clone();
        let db = hkask_storage::Database::open(&db_path, &passphrase)
            .map_err(|e| anyhow::anyhow!("Failed to open episodic database: {}", e))?;
        let conn = db.conn_arc();
        let triple_store = hkask_storage::TripleStore::new(conn);
        let memory = hkask_memory::EpisodicMemory::new(triple_store);
        Ok(EpisodicServer::new(memory, ctx.webid))
    },
    credentials: vec![
        hkask_mcp::CredentialRequirement::required("HKASK_EPISODIC_DB", "Path to episodic database file"),
        hkask_mcp::CredentialRequirement::required("HKASK_DB_PASSPHRASE", "SQLCipher encryption passphrase"),
    ]
);
