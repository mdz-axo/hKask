//! hKask MCP Training — Model training data ingestion and fine-tuning server.
//!
//! Starts an MCP server over stdio exposing 6 tools:
//! - `training_ingest_qa` — Ingest QA pairs for future model fine-tuning
//! - `training_submit` — Submit a training job for execution
//! - `training_status` — Query training job status
//! - `training_cancel` — Cancel a running or queued job
//! - `training_list_adapters` — List completed LoRA adapters
//! - `training_delete_adapter` — Remove a LoRA adapter
//!
//! Architecture:
//!   Dataset file → DatasetPipeline → normalized ChatML → TrainingJob → TrainingProvider → LoRAAdapter
//!
//! Provider selection via config (training.provider in settings.json), routed through
//! the shared `hkask-services` config init. Provider pluggability is via the
//! `TrainingProvider` trait, isolating the MCP surface from framework-specific details.
//!
//! # Environment Variables
//!
//! - `HKASK_MEMORY_DB` — Path to per-agent memory database for QA storage
//! - `HKASK_DB_PASSPHRASE` — Passphrase for the database
//! - `HKASK_TRAINING_PROVIDER` — Override training provider (axolotl|unsloth)
//! - `HKASK_TRAINING_CACHE_DIR` — Dataset cache directory

use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_mcp::validate_field;
use hkask_mcp_training::adapters::{
    AdapterStore, InMemoryAdapterStore, LoRAAdapter, SqliteAdapterStore,
};
use hkask_mcp_training::dataset::{ChatConversation, ChatMessage, DatasetPipeline};
use hkask_mcp_training::providers::{
    ProviderConfig, TrainingJob, TrainingJobStatus, TrainingParams, TrainingProvider,
    TrainingProviderId, create_provider,
};
use hkask_memory::SemanticMemory;
use hkask_storage::{DatabaseConnection, Triple};
use hkask_types::{McpErrorKind, Visibility, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainSubmitRequest {
    /// Path to the training dataset file.
    pub dataset_path: String,
    /// Base model to fine-tune (provider-prefixed, e.g., "OM/qwen3:8b").
    pub base_model: String,
    /// Optional training hyperparameters. Uses defaults if not provided.
    #[serde(default)]
    pub params: Option<TrainingParams>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainStatusRequest {
    /// Job ID from a previous `training_submit` call.
    pub job_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainCancelRequest {
    /// Job ID to cancel.
    pub job_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainDeleteAdapterRequest {
    /// Adapter ID to delete.
    pub adapter_id: String,
}

// ── Server ───────────────────────────────────────────────────────────────

pub struct TrainingServer {
    webid: WebID,
    replicant: String,
    daemon: Option<hkask_mcp::DaemonClient>,
    semantic: Option<SemanticMemory>,
    provider: Box<dyn TrainingProvider>,
    pipeline: DatasetPipeline,
    adapter_store: Arc<dyn AdapterStore>,
}

impl TrainingServer {
    pub fn new(
        webid: WebID,
        replicant: String,
        daemon: Option<hkask_mcp::DaemonClient>,
        semantic: Option<SemanticMemory>,
        provider: Box<dyn TrainingProvider>,
        pipeline: DatasetPipeline,
        adapter_store: Arc<dyn AdapterStore>,
    ) -> Self {
        Self {
            webid,
            replicant,
            daemon,
            semantic,
            provider,
            pipeline,
            adapter_store,
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

    #[tool(
        description = "Submit a training job for execution. Ingests, normalizes, and submits a dataset for LoRA fine-tuning via the configured provider (axolotl or unsloth)."
    )]
    async fn training_submit(
        &self,
        Parameters(TrainSubmitRequest {
            dataset_path,
            base_model,
            params,
        }): Parameters<TrainSubmitRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("training_submit", &self.webid);

        let file_path = PathBuf::from(&dataset_path);
        if !file_path.exists() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!("Dataset file not found: {}", dataset_path))
                    .to_json_string(),
            );
        }

        // Ingest and normalize the dataset
        let normalized_path = match self.pipeline.ingest(&file_path) {
            Ok(path) => path,
            Err(e) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(format!("Dataset pipeline error: {e}"))
                        .to_json_string(),
                );
            }
        };

        let job = TrainingJob {
            id: uuid::Uuid::new_v4().to_string(),
            dataset_path: normalized_path,
            base_model: base_model.clone(),
            params: params.unwrap_or_default(),
            status: TrainingJobStatus::Queued,
            created_at: chrono::Utc::now(),
            provider: self.provider_id(),
        };

        match self.provider.submit(&job).await {
            Ok(job_id) => {
                let result = json!({
                    "job_id": job_id,
                    "status": "queued",
                    "base_model": base_model,
                    "provider": format!("{:?}", self.provider_id()),
                });
                self.record_experience("training_submit", &dataset_path, "success", result.clone());
                span.ok_json(result)
            }
            Err(e) => {
                tracing::error!(
                    target: "cns.training.job.fail",
                    error = %e,
                    "Training job submission failed"
                );
                span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Training job failed: {}", e)).to_json_string(),
                )
            }
        }
    }

    #[tool(description = "Query the status of a training job by its ID.")]
    async fn training_status(
        &self,
        Parameters(TrainStatusRequest { job_id }): Parameters<TrainStatusRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("training_status", &self.webid);
        match self.provider.status(&job_id).await {
            Ok(status) => {
                let result = json!({
                    "job_id": job_id,
                    "status": serde_json::to_value(status).unwrap_or_default(),
                });
                span.ok_json(result)
            }
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("Status query failed: {}", e)).to_json_string(),
            ),
        }
    }

    #[tool(description = "Cancel a running or queued training job.")]
    async fn training_cancel(
        &self,
        Parameters(TrainCancelRequest { job_id }): Parameters<TrainCancelRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("training_cancel", &self.webid);
        match self.provider.cancel(&job_id).await {
            Ok(()) => {
                let result = json!({ "job_id": job_id, "status": "cancelled" });
                span.ok_json(result)
            }
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("Cancellation failed: {}", e)).to_json_string(),
            ),
        }
    }

    #[tool(description = "List all completed LoRA adapters available for model composition.")]
    async fn training_list_adapters(&self) -> String {
        let span = ToolSpanGuard::new("training_list_adapters", &self.webid);
        match self.provider.list_adapters().await {
            Ok(adapter_ids) => {
                // Enrich with metadata from adapter store
                let metadata_list: Vec<serde_json::Value> =
                    futures::future::join_all(adapter_ids.iter().map(|id| async {
                        match self.adapter_store.get_metadata(id).await {
                            Ok(Some(adapter)) => json!({
                                "id": adapter.id,
                                "name": adapter.name,
                                "base_model": adapter.base_model,
                                "dataset_hash": adapter.dataset_hash,
                                "training_job_id": adapter.training_job_id,
                                "created_at": adapter.created_at,
                                "size_bytes": adapter.size_bytes,
                                "metrics": adapter.metrics.map(|m| json!({
                                    "loss": m.loss,
                                    "perplexity": m.perplexity,
                                    "training_duration_secs": m.training_duration_secs,
                                    "tokens_processed": m.tokens_processed,
                                })),
                            }),
                            _ => json!({"id": id, "warning": "metadata not found in store"}),
                        }
                    }))
                    .await;
                span.ok_json(json!({
                    "adapters": metadata_list,
                    "total": metadata_list.len(),
                }))
            }
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("Failed to list adapters: {}", e)).to_json_string(),
            ),
        }
    }

    #[tool(description = "Delete a LoRA adapter and all associated artifacts.")]
    async fn training_delete_adapter(
        &self,
        Parameters(TrainDeleteAdapterRequest { adapter_id }): Parameters<TrainDeleteAdapterRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("training_delete_adapter", &self.webid);

        // Delete from provider storage (filesystem)
        if let Err(e) = self.provider.delete_adapter(&adapter_id).await {
            // Non-fatal — provider storage may already be gone, still clean up metadata
            tracing::warn!(
                target: "cns.training.adapter.deleted",
                adapter_id = %adapter_id,
                error = %e,
                "Provider deletion failed, continuing with metadata cleanup"
            );
        }

        // Delete from adapter store (metadata + blob)
        match self.adapter_store.delete(&adapter_id).await {
            Ok(()) => {
                let result = json!({ "adapter_id": adapter_id, "deleted": true });
                span.ok_json(result)
            }
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("Metadata deletion failed: {}", e)).to_json_string(),
            ),
        }
    }

    fn provider_id(&self) -> TrainingProviderId {
        // The provider doesn't expose its ID — derive from type name heuristics.
        // A proper implementation would store the ID on the server struct.
        TrainingProviderId::Axolotl
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

    // Resolve provider config from environment
    let provider_id = std::env::var("HKASK_TRAINING_PROVIDER")
        .ok()
        .and_then(|s| TrainingProviderId::from_str(&s))
        .unwrap_or(TrainingProviderId::Axolotl);
    let cloud_dispatch = std::env::var("HKASK_TRAINING_CLOUD_DISPATCH")
        .map(|s| s == "1" || s == "true")
        .unwrap_or(false);
    let provider_config = ProviderConfig {
        provider: provider_id,
        axolotl_path: std::env::var("HKASK_AXOLOTL_PATH").ok().map(PathBuf::from),
        python_path: std::env::var("HKASK_PYTHON_PATH").ok().map(PathBuf::from),
        cloud_dispatch,
    };

    let cache_dir = PathBuf::from(
        std::env::var("HKASK_TRAINING_CACHE_DIR")
            .unwrap_or_else(|_| "/tmp/hkask-training-cache".to_string()),
    );
    let pipeline = DatasetPipeline::new(cache_dir);

    // In-memory adapter store for now — production should use SQLite
    let adapter_store: Arc<dyn AdapterStore> = Arc::new(InMemoryAdapterStore::new());

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

            let provider = create_provider(&provider_config)
                .map_err(|e| anyhow::anyhow!("Failed to create training provider: {}", e))?;

            Ok(TrainingServer::new(
                ctx.webid,
                replicant.clone(),
                daemon_client.clone(),
                semantic,
                provider,
                pipeline.clone(),
                Arc::clone(&adapter_store),
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
