//! hKask MCP Training — Model training data ingestion server (stub)
//!
//! Starts an MCP server over stdio exposing 1 tool:
//! - `training_ingest_qa` — Ingest QA pairs for future model fine-tuning
//!
//! This is a stub — the actual training pipeline (dataset assembly, formatting,
//! fine-tuning job submission) will be built in a future release. For now, it
//! accepts and stores QA pairs as the destination for `docproc_generate_qa`.
//!
//! # Environment Variables
//!
//! - `HKASK_MEMORY_DB` — Path to per-agent memory database for QA storage
//! - `HKASK_DB_PASSPHRASE` — Passphrase for the database

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

// ── Request structs ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QaItem {
    pub question: String,
    pub answer: String,
    #[serde(default)]
    pub bloom_level: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IngestQaRequest {
    /// QA pairs to ingest for training.
    pub qa_items: Vec<QaItem>,
    /// Source document or dataset identifier.
    pub source: String,
    /// Optional training dataset name (default: "default").
    #[serde(default)]
    pub dataset: Option<String>,
}

// ── Server ───────────────────────────────────────────────────────────────

pub struct TrainingServer {
    webid: WebID,
    replicant: String,
    daemon: Option<hkask_mcp::DaemonClient>,
    semantic: Option<SemanticMemory>,
}

impl TrainingServer {
    pub fn new(
        webid: WebID,
        replicant: String,
        daemon: Option<hkask_mcp::DaemonClient>,
        semantic: Option<SemanticMemory>,
    ) -> Self {
        Self {
            webid,
            replicant,
            daemon,
            semantic,
        }
    }

    fn record_experience(
        &self,
        tool: &str,
        input_summary: &str,
        outcome: &str,
        detail: serde_json::Value,
    ) {
        if let Some(ref daemon) = self.daemon {
            let value = serde_json::json!({
                "tool": tool, "input": input_summary, "outcome": outcome,
                "detail": detail, "timestamp": chrono::Utc::now().to_rfc3339(),
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
                        tracing::debug!(target: "hkask.mcp.training.memory", tool = %tool_name, "Experience stored via daemon");
                    }
                    Ok(other) => {
                        tracing::warn!(target: "hkask.mcp.training.memory", tool = %tool_name, response = ?other, "Unexpected daemon response")
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.mcp.training.memory", tool = %tool_name, error = %e, "Failed to store experience")
                    }
                }
            });
        }
    }
}

// ── Tools ────────────────────────────────────────────────────────────────

#[tool_router(server_handler)]
impl TrainingServer {
    #[tool(
        description = "Ingest QA pairs for model training. Stores question-answer pairs with provenance in semantic memory for future fine-tuning dataset assembly."
    )]
    async fn training_ingest_qa(
        &self,
        Parameters(IngestQaRequest {
            qa_items,
            source,
            dataset,
        }): Parameters<IngestQaRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("training_ingest_qa", &self.webid);
        let source_clone = source.clone();

        let Some(semantic) = &self.semantic else {
            return span.error(
                McpErrorKind::PermissionDenied,
                McpToolError::permission_denied(
                    "Semantic memory not available — set HKASK_MEMORY_DB and HKASK_DB_PASSPHRASE",
                )
                .to_json_string(),
            );
        };

        if qa_items.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("qa_items must not be empty").to_json_string(),
            );
        }

        validate_field!(span, "source", &source, 256);

        let ds = dataset.as_deref().unwrap_or("default");

        let mut stored = 0;
        let mut errors = Vec::new();

        for (i, qa) in qa_items.iter().enumerate() {
            let entity = format!("training:qa:{ds}:{source}:{i}");
            let level = qa.bloom_level.as_deref().unwrap_or("factual");
            let value = json!({
                "question": qa.question,
                "answer": qa.answer,
                "bloom_level": level,
                "source": source,
                "dataset": ds,
            });

            let triple = Triple::new(&entity, "training_qa_pair", value, self.webid)
                .with_visibility(Visibility::Public)
                .with_confidence(1.0);

            match semantic.store(triple) {
                Ok(()) => stored += 1,
                Err(e) => errors.push(format!("Item {i}: {e}")),
            }
        }

        if errors.is_empty() {
            let result = json!({ "stored": stored, "source": source, "dataset": ds });
            self.record_experience(
                "training_ingest_qa",
                &source_clone,
                "success",
                result.clone(),
            );
            span.ok_json(result)
        } else {
            let result =
                json!({ "stored": stored, "errors": errors, "source": source, "dataset": ds });
            self.record_experience(
                "training_ingest_qa",
                &source_clone,
                "partial",
                result.clone(),
            );
            span.internal_error(result)
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let replicant = std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());

    let daemon_ok = match try_daemon_flow(&replicant).await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(target: "hkask.mcp.training", replicant = %replicant, error = %e, "Daemon unavailable — falling back to direct mode");
            false
        }
    };

    let daemon_client = if daemon_ok {
        Some(hkask_mcp::DaemonClient::new())
    } else {
        None
    };

    hkask_mcp::run_server(
        "hkask-mcp-training",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            let semantic = match ctx.credentials.get("HKASK_MEMORY_DB") {
                Some(path) => {
                    let passphrase =
                        ctx.credentials.get("HKASK_DB_PASSPHRASE").ok_or_else(|| {
                            anyhow::anyhow!("HKASK_MEMORY_DB set but HKASK_DB_PASSPHRASE missing")
                        })?;
                    let db = hkask_storage::Database::open(path, passphrase)
                        .map_err(|e| anyhow::anyhow!("Failed to open memory database: {}", e))?;
                    let conn = db.conn_arc();
                    let triple_store = hkask_storage::TripleStore::new(Arc::clone(&conn));
                    let embedding_store = hkask_storage::EmbeddingStore::new(conn);
                    Some(hkask_memory::SemanticMemory::new(
                        triple_store,
                        embedding_store,
                    ))
                }
                None => None,
            };

            Ok(TrainingServer::new(
                ctx.webid,
                replicant.clone(),
                daemon_client.clone(),
                semantic,
            ))
        },
        vec![
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_MEMORY_DB",
                "Path to per-agent memory database for QA storage (in-memory if absent)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_DB_PASSPHRASE",
                "Passphrase for the database (required if HKASK_MEMORY_DB is set)",
            ),
        ],
    )
    .await
}

async fn try_daemon_flow(replicant: &str) -> anyhow::Result<()> {
    let client = hkask_mcp::DaemonClient::new();

    let auth = client.auth_query(replicant).await?;
    match auth {
        hkask_mcp::DaemonResponse::AuthResponse {
            authenticated: true,
            webid: Some(ref webid),
            ..
        } => {
            tracing::info!(target: "hkask.mcp.training", replicant = %replicant, webid = %webid, "Replicant authenticated via daemon");
        }
        hkask_mcp::DaemonResponse::AuthResponse {
            authenticated: false,
            action: Some(ref action),
            ..
        } if action == "prompt_user" => {
            anyhow::bail!(
                "Replicant '{}' is not authenticated. Enter the replicant's passphrase in the hKask terminal.",
                replicant
            );
        }
        other => anyhow::bail!("Unexpected auth response: {:?}", other),
    }

    let assignment = client.assignment_query(replicant, "training").await?;
    match assignment {
        hkask_mcp::DaemonResponse::AssignmentResponse { assigned: true } => {
            tracing::info!(target: "hkask.mcp.training", replicant = %replicant, "Replicant assigned to training role");
        }
        hkask_mcp::DaemonResponse::AssignmentResponse { assigned: false } => {
            anyhow::bail!(
                "Replicant '{}' is not assigned to the training MCP role. Use 'kask pod assign {} training' to grant this role.",
                replicant,
                replicant
            );
        }
        other => anyhow::bail!("Unexpected assignment response: {:?}", other),
    }

    tracing::info!(target: "hkask.mcp.training", replicant = %replicant, "P4 dual-gate verification complete");
    Ok(())
}
