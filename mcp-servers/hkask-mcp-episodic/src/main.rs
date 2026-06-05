//! hKask MCP Episodic — Episodic memory store and recall
//!
//! 5 tools:
//! - `episodic_ping` — Liveness and storage info
//! - `episodic_store` — Store an episodic triple (private, perspective-bound)
//! - `episodic_recall` — Recall triples by entity (filtered by caller's WebID)
//! - `episodic_budget` — Storage usage and budget info
//! - `episodic_consolidate_status` — Check consolidation candidates and budget status
//!
//! **Sovereignty:** All operations use the calling agent's `WebID` as the
//! `perspective`. An agent cannot read another agent's episodic memory.
//!
//! **Consolidation:** This server is read-only for consolidation status.
//! Full consolidation (episodic→semantic promotion) requires both
//! `EpisodicMemory` and `SemanticMemory`, available only through the CLI,
//! API, or Chat ConsolidationService surfaces. This server connects to
//! the per-agent memory DB (`HKASK_MEMORY_DB` / `hkask-memory-{agent}.db`)
//! alongside the semantic MCP server — both access the same database.

use hkask_mcp::server::ToolSpanGuard;
use hkask_mcp::validate_field;
use hkask_memory::EpisodicMemory;
use hkask_storage::Triple;
use hkask_types::{Visibility, WebID};
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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConsolidateStatusRequest {}

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
        span.ok_json(json!({
            "status": "ok",
            "server": "hkask-mcp-episodic",
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

        match self.memory.store(triple) {
            Ok(()) => span.ok_json(json!({
                "stored": true,
                "entity": entity,
                "attribute": attribute,
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
                span.ok_json(json!({
                    "count": serialized.len(),
                    "triples": serialized,
                }))
            }
            Err(e) => span.internal_error(
                json!({"error": format!("Failed to recall episodic triples: {}", e)}),
            ),
        }
    }

    #[tool(description = "Storage usage and budget for episodic memory")]
    async fn episodic_budget(&self, Parameters(_budget): Parameters<BudgetRequest>) -> String {
        let span = ToolSpanGuard::new("episodic_budget", &self.webid);

        let usage = match self.memory.storage_usage(&self.webid) {
            Ok(u) => u,
            Err(e) => {
                return span.internal_error(
                    json!({"error": format!("Failed to query storage usage: {}", e)}),
                );
            }
        };
        let budget = self.memory.storage_budget();
        let remaining = budget.saturating_sub(usage);

        span.ok_json(json!({
            "used": usage,
            "budget": budget,
            "remaining": remaining,
        }))
    }

    #[tool(
        description = "Check consolidation candidates and budget status for episodic→semantic promotion"
    )]
    async fn episodic_consolidate_status(
        &self,
        Parameters(_req): Parameters<ConsolidateStatusRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("episodic_consolidate_status", &self.webid);

        // The MCP episodic server only has EpisodicMemory, not SemanticMemory.
        // Full consolidation (episodic→semantic) requires both stores, available
        // via the CLI/API/Chat ConsolidationService surfaces.
        // This tool identifies consolidation candidates and reports status.

        let candidate_count = self.memory.consolidation_candidate_count(&self.webid);
        let usage = match self.memory.storage_usage(&self.webid) {
            Ok(u) => u,
            Err(e) => {
                return span.internal_error(
                    json!({"error": format!("Failed to query storage usage: {}", e)}),
                );
            }
        };
        let budget = self.memory.storage_budget();
        let over_budget = usage > budget;

        span.ok_json(json!({
            "consolidation_candidates": candidate_count,
            "episodic_usage": usage,
            "episodic_budget": budget,
            "over_budget": over_budget,
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hkask_mcp::run_server(
        "hkask-mcp-episodic",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            let db = ctx.open_database("HKASK_MEMORY_DB")?;
            let conn = db.conn_arc();
            let triple_store = hkask_storage::TripleStore::new(conn);
            let memory = hkask_memory::EpisodicMemory::new(triple_store);
            Ok(EpisodicServer::new(memory, ctx.webid))
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
