//! hKask MCP Training — Model training data ingestion and fine-tuning server.
//!
//! Exposes a full training surface:
//! - `training_ingest_qa` — Ingest QA pairs for model fine-tuning
//! - `training_submit` — Submit a training job via pluggable host
//! - `training_status` — Query training job status
//! - `training_cancel` — Cancel a running job
//! - `training_list_adapters` — List completed LoRA adapters
//! - `training_delete_adapter` — Remove a LoRA adapter
//! - `training_assemble_dataset` — Assemble stored QA pairs into a ChatML JSONL dataset file
//! - `training_generate_traces` — Generate decomposition traces from skill documents
//! - `training_evaluate` — Evaluate a trained adapter against a test dataset
//! - `training_register_adapter` — Register a completed adapter in persistent storage
//! - `training_recommend_model` — Recommend a base model for fine-tuning
//! - `training_record_invocation` — Record an adapter invocation for continuous training
//! - `training_curate_feedback` — Curate feedback from stored QA pairs for continuous skills training
//! - `training_retrain` — Retrain an adapter with curated feedback (closes the continuous loop)
//! - `training_ingest_dataset` — Ingest a raw dataset into the normalized cache
//!
//! Architecture:
//!   Dataset file → DatasetPipeline → normalized ChatML → TrainingJob → TrainingHost → LoRAAdapter
//!
//! Host selection via config (training.host in settings.json), routed through
//! the shared `hkask-services` config init. Host pluggability is via the
//! `TrainingHost` trait, isolating the MCP surface from framework-specific details.
//!
//! # Environment Variables
//!
//! - `HKASK_MEMORY_DB` — Path to per-agent memory database for QA storage
//! - `HKASK_DB_PASSPHRASE` — Passphrase for the database
//! - `HKASK_TRAINING_HOST` — Override host (together|runpod|baseten) — where compute runs
//! - `HKASK_TRAINING_HARNESS` — Override harness (axolotl|unsloth) — what tooling runs
//! - `HKASK_TRAINING_CACHE_DIR` — Dataset cache directory
//! - `TOGETHER_API_KEY` — Together AI API key (for Together host)
//! - `RUNPOD_API_KEY` — Runpod API key (for Runpod host)
//! - `RUNPOD_TEMPLATE_ID` — Runpod GPU pod template ID with axolotl pre-installed
//! - `RUNPOD_GPU_TYPE_ID` — GPU type ID for Runpod pods (default: "NVIDIA RTX 4090")
//! - `RUNPOD_CONTAINER_DISK_GB` — Container disk GB for Runpod pods (default: 50)
//! - `RUNPOD_MIN_MEMORY_GB` — Minimum memory GB for Runpod pods (default: 24)
//! - `HKASK_DATASET_URL` — Public URL for dataset download by Runpod pods
//! - `BASETEN_API_KEY` — Baseten API key (for Baseten host)
//! - `BASETEN_PROJECT_ID` — Baseten training project ID
//! - `BASETEN_GPU` — GPU accelerator for Baseten (default: "H100")
//! - `BASETEN_GPU_COUNT` — Number of GPUs for Baseten (default: 1)
//! - `HF_TOKEN` — HuggingFace access token (for gated model loading on Baseten)
//! - `HKASK_AXOLOTL_PATH` — Path to axolotl CLI (for Axolotl host)
//! - `HKASK_PYTHON_PATH` — Path to python3 interpreter (for Unsloth host)

use hkask_inference::{InferenceConfig, InferenceRouter};
use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_mcp::validate_field;
use hkask_mcp_training::adapters::{
    AdapterMetrics, AdapterStore, InMemoryAdapterStore, JobStore, LoRAAdapter, SqliteAdapterStore,
};
use hkask_mcp_training::dataset::DatasetPipeline;
use hkask_mcp_training::providers::{
    TrainingHarnessId, TrainingHost, TrainingHostConfig, TrainingHostId, TrainingJob,
    TrainingJobStatus, TrainingParams, create_host,
};
use hkask_memory::SemanticMemory;
use hkask_storage::Triple;
use hkask_types::ports::InferencePort;
use hkask_types::template::LLMParameters;
use hkask_types::time::now_rfc3339;
use hkask_types::{McpErrorKind, Visibility, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AssembleDatasetRequest {
    /// Training dataset name to filter by (matches QA pairs ingested with this dataset).
    #[serde(default)]
    pub dataset: Option<String>,
    /// Source identifier to filter by.
    #[serde(default)]
    pub source: Option<String>,
    /// Bloom level to filter by (e.g., "remembering", "applying").
    #[serde(default)]
    pub bloom_level: Option<String>,
    /// Path to write the assembled ChatML JSONL file.
    pub output_path: String,
    /// Fraction of examples to reserve for training (default 1.0 = all train, no test split).
    /// Set to 0.8 for an 80/20 train/test split. Test file is written to {output_path}.test.jsonl.
    #[serde(default)]
    pub train_split: Option<f64>,
    /// Maximum number of examples to include (default: all matching).
    #[serde(default)]
    pub max_examples: Option<usize>,
    /// Optional system prompt to prepend to each assembled conversation.
    /// Sets agent persona/context for fine-tuning (e.g., "You are an hKask agent trained in constraint classification.").
    #[serde(default)]
    pub system_prompt: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateTracesRequest {
    /// Path to the skill document (SKILL.md) or inline text describing the process.
    pub skill_document: String,
    /// Name of the skill for output tracking.
    pub skill_name: String,
    /// Number of decomposition traces to generate (default 50).
    #[serde(default)]
    pub num_traces: Option<usize>,
    /// Bloom taxonomy levels to target (e.g., ["applying", "analyzing"]).
    /// Default: all levels.
    #[serde(default)]
    pub bloom_levels: Option<Vec<String>>,
    /// Path to write the generated ChatML JSONL file.
    pub output_path: String,
    /// Optional system prompt to prepend to each trace (sets agent persona/context).
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Model to use for trace generation (provider-prefixed, e.g., "DI/meta-llama/Llama-3.3-70B-Instruct").
    /// Defaults to the server's configured default model.
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainEvaluateRequest {
    /// Adapter ID or fine-tuned model name to evaluate.
    pub adapter_id: String,
    /// Path to test dataset (ChatML JSONL). Each line must have a "messages" array
    /// with user/assistant turns. The last assistant message is the expected answer.
    pub test_dataset_path: String,
    /// Model identifier to run evaluation against (provider-prefixed).
    /// For Together AI adapters, use the fine-tuned model name
    /// (e.g., "mdz-axolotl/Qwen3.5-9B-ft-abc123").
    pub model: String,
    /// Evaluation method: "exact_match" (default), "contains", or "semantic".
    /// - exact_match: generated == expected after trimming
    /// - contains: expected substring is found in generated
    /// - semantic: uses a second inference call to judge correctness
    #[serde(default)]
    pub method: Option<String>,
    /// Maximum number of examples to evaluate (default: all).
    #[serde(default)]
    pub max_examples: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainRegisterAdapterRequest {
    /// Adapter ID (from training job completion).
    pub adapter_id: String,
    /// Human-readable name for the adapter (e.g., "constraint-forces-v3").
    pub name: String,
    /// Skill name this adapter serves (e.g., "constraint-forces").
    /// Enables adapter-to-skill mapping for the registry.
    pub skill_name: String,
    /// Base model the adapter was trained on (provider-prefixed).
    pub base_model: String,
    /// Content hash of the training dataset.
    #[serde(default)]
    pub dataset_hash: Option<String>,
    /// ID of the originating training job.
    #[serde(default)]
    pub training_job_id: Option<String>,
    /// Size of adapter weights in bytes.
    #[serde(default)]
    pub size_bytes: Option<u64>,
    /// Final training loss.
    #[serde(default)]
    pub loss: Option<f32>,
    /// Perplexity at end of training.
    #[serde(default)]
    pub perplexity: Option<f32>,
    /// Training duration in seconds.
    #[serde(default)]
    pub training_duration_secs: Option<u64>,
    /// Number of tokens processed.
    #[serde(default)]
    pub tokens_processed: Option<u64>,
    /// Adapter version number (default: 1). Increment on retraining.
    #[serde(default)]
    pub version: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainRecommendModelRequest {
    /// Task type: "classification", "generation", "procedural", "reasoning", or "chat".
    pub task_type: String,
    /// Budget constraint: "low" (<$1/run), "medium" (<$10/run), or "high" (unlimited).
    #[serde(default)]
    pub budget: Option<String>,
    /// Latency requirement: "realtime" (<2s), "batch" (minutes ok), or "flexible".
    #[serde(default)]
    pub latency: Option<String>,
    /// License requirement: "apache2", "mit", "open", or "any".
    #[serde(default)]
    pub license: Option<String>,
    /// Preferred provider: "together", "fireworks", "deepinfra", "ollama", or "any".
    #[serde(default)]
    pub provider: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainRecordInvocationRequest {
    /// Adapter ID that was used.
    pub adapter_id: String,
    /// Skill name that was invoked.
    pub skill_name: String,
    /// Summary of the user's input/query.
    pub input_summary: String,
    /// Summary of the adapter's output/response.
    pub output_summary: String,
    /// CNS span identifier for correlation (e.g., "cns.training.invoke.constraint-forces").
    #[serde(default)]
    pub cns_span: Option<String>,
    /// Confidence score for the invocation (0.0–1.0).
    #[serde(default)]
    pub confidence: Option<f64>,
    /// Whether the invocation was successful (default: true).
    #[serde(default)]
    pub success: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainCurateFeedbackRequest {
    /// Dataset name to filter QA pairs by.
    #[serde(default)]
    pub dataset: Option<String>,
    /// Source identifier to filter by.
    #[serde(default)]
    pub source: Option<String>,
    /// Path to write the corrected ChatML JSONL feedback file.
    pub output_path: String,
    /// Model to use for validation/correction (provider-prefixed).
    /// Defaults to the server's configured default model.
    #[serde(default)]
    pub model: Option<String>,
    /// Maximum number of QA pairs to review (default: 50).
    #[serde(default)]
    pub max_pairs: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainRetrainRequest {
    /// Path to the original training dataset.
    pub original_dataset_path: String,
    /// Path to the feedback JSONL file (from training_curate_feedback).
    pub feedback_path: String,
    /// Base model to fine-tune (provider-prefixed).
    pub base_model: String,
    /// Adapter name for the new version (e.g., "constraint-forces-v4").
    pub adapter_name: String,
    /// Skill name for the adapter registry.
    pub skill_name: String,
    /// Optional training hyperparameters. Uses defaults if not provided.
    #[serde(default)]
    pub params: Option<TrainingParams>,
    /// Path to write the merged dataset (default: auto-generated in cache dir).
    #[serde(default)]
    pub merged_output_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainIngestDatasetRequest {
    /// Path to the raw dataset file (JSONL, JSON, or TXT).
    pub dataset_path: String,
    /// Optional cache directory override (default: server's configured cache dir).
    #[serde(default)]
    pub cache_dir: Option<String>,
}

// ── Server ───────────────────────────────────────────────────────────────

pub struct TrainingServer {
    webid: WebID,
    replicant: String,
    daemon: Option<hkask_mcp::DaemonClient>,
    semantic: Option<SemanticMemory>,
    host: Box<dyn TrainingHost>,
    host_id: TrainingHostId,
    pipeline: Mutex<DatasetPipeline>,
    adapter_store: Arc<dyn AdapterStore>,
    job_store: Option<JobStore>,
    inference_config: InferenceConfig,
}

impl TrainingServer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        webid: WebID,
        replicant: String,
        daemon: Option<hkask_mcp::DaemonClient>,
        semantic: Option<SemanticMemory>,
        host: Box<dyn TrainingHost>,
        host_id: TrainingHostId,
        pipeline: DatasetPipeline,
        adapter_store: Arc<dyn AdapterStore>,
        job_store: Option<JobStore>,
        inference_config: InferenceConfig,
    ) -> Self {
        Self {
            webid,
            replicant,
            daemon,
            semantic,
            host,
            host_id,
            pipeline: Mutex::new(pipeline),
            adapter_store,
            job_store,
            inference_config,
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
                "detail": detail, "timestamp": now_rfc3339(),
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
        description = "Submit a training job for execution. Ingests, normalizes, and submits a dataset for LoRA fine-tuning via the configured host (axolotl or unsloth)."
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
        let normalized_path = match self.pipeline.lock().unwrap().ingest(&file_path) {
            Ok(path) => path,
            Err(e) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(format!("Dataset pipeline error: {e}"))
                        .to_json_string(),
                );
            }
        };

        // Validate token lengths — warn if examples exceed typical context windows.
        // Rough heuristic: 1 token ≈ 4 characters for English text.
        let mut token_warnings: Vec<serde_json::Value> = Vec::new();
        if let Ok(normalized_content) = std::fs::read_to_string(&normalized_path) {
            for (i, line) in normalized_content.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let approx_tokens = trimmed.len() / 4;
                // Warn at 2048 tokens (8K context), error at 4096 tokens (16K context)
                if approx_tokens > 4096 {
                    token_warnings.push(json!({
                        "line": i + 1,
                        "approx_tokens": approx_tokens,
                        "severity": "error",
                        "message": "Example likely exceeds 16K context window — may be truncated during training"
                    }));
                } else if approx_tokens > 2048 {
                    token_warnings.push(json!({
                        "line": i + 1,
                        "approx_tokens": approx_tokens,
                        "severity": "warning",
                        "message": "Example approaches 8K context limit — consider truncation"
                    }));
                }
            }
        }

        let job = TrainingJob {
            id: uuid::Uuid::new_v4().to_string(),
            dataset_path: normalized_path.clone(),
            base_model: base_model.clone(),
            params: params.unwrap_or_default(),
            status: TrainingJobStatus::Queued,
            created_at: chrono::Utc::now(),
            host: self.host_id,
        };

        // Persist job for survival across server restarts
        if let Some(ref job_store) = self.job_store {
            let params_json = serde_json::to_string(&job.params).unwrap_or_default();
            let status_str = format!("{:?}", TrainingJobStatus::Queued).to_lowercase();
            if let Err(e) = job_store.store(
                &job.id,
                &job.base_model,
                &job.dataset_path.to_string_lossy(),
                &params_json,
                &status_str,
                job.created_at.timestamp(),
                &format!("{:?}", job.host).to_lowercase(),
            ) {
                tracing::warn!(
                    target: "cns.training.job.persist",
                    job_id = %job.id,
                    error = %e,
                    "Failed to persist job"
                );
            }
        }

        match self.host.submit(&job).await {
            Ok(job_id) => {
                let mut result = json!({
                    "job_id": job_id,
                    "status": "queued",
                    "base_model": base_model,
                    "host": format!("{:?}", self.host_id),
                });
                if !token_warnings.is_empty() {
                    result["token_warnings"] = json!(token_warnings);
                    result["token_warning_count"] = json!(token_warnings.len());
                }
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

    #[tool(
        description = "Query the status of a training job by its ID. When a job completes, automatically registers the adapter in the persistent store if not already registered."
    )]
    async fn training_status(
        &self,
        Parameters(TrainStatusRequest { job_id }): Parameters<TrainStatusRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("training_status", &self.webid);
        match self.host.status(&job_id).await {
            Ok(status) => {
                let mut result = json!({
                    "job_id": job_id,
                    "status": serde_json::to_value(status).unwrap_or_default(),
                });

                // Persist status update
                if let Some(ref job_store) = self.job_store {
                    let status_str = format!("{:?}", status).to_lowercase();
                    if let Err(e) = job_store.update_status(&job_id, &status_str) {
                        tracing::warn!(
                            target: "cns.training.job.persist",
                            job_id = %job_id,
                            error = %e,
                            "Failed to update job status"
                        );
                    }
                }

                // Auto-register adapter on completion
                if status == TrainingJobStatus::Completed {
                    match self.adapter_store.get_metadata(&job_id).await {
                        Ok(Some(_)) => {
                            result["adapter_registered"] = json!(true);
                            result["adapter_note"] = json!("Already registered");
                        }
                        _ => {
                            // Try to get completion metadata from host
                            match self.host.completion_metadata(&job_id).await {
                                Ok(Some(meta)) => {
                                    let adapter = LoRAAdapter {
                                        id: job_id.clone(),
                                        name: meta
                                            .output_name
                                            .unwrap_or_else(|| format!("adapter-{}", &job_id[..8])),
                                        base_model: meta.base_model.clone(),
                                        dataset_hash: String::new(),
                                        training_job_id: job_id.clone(),
                                        created_at: chrono::Utc::now().timestamp(),
                                        size_bytes: 0,
                                        skill_name: String::new(),
                                        version: 1,
                                        metrics: Some(AdapterMetrics {
                                            loss: meta.loss,
                                            perplexity: None,
                                            training_duration_secs: meta.training_duration_secs,
                                            tokens_processed: meta.tokens_processed,
                                        }),
                                    };
                                    match self.adapter_store.store_metadata(&adapter).await {
                                        Ok(()) => {
                                            result["adapter_registered"] = json!(true);
                                            result["adapter_name"] = json!(adapter.name);
                                            result["base_model"] = json!(meta.base_model);

                                            // Store adapter weight blob if available locally
                                            match self.host.adapter_weight_path(&job_id).await {
                                                Ok(Some(weight_path)) => {
                                                    match tokio::fs::read(&weight_path).await {
                                                        Ok(blob) => {
                                                            let size = blob.len() as u64;
                                                            if let Err(e) = self
                                                                .adapter_store
                                                                .store_blob(&job_id, blob)
                                                                .await
                                                            {
                                                                tracing::warn!(
                                                                    target: "cns.training.adapter.blob",
                                                                    adapter_id = %job_id,
                                                                    error = %e,
                                                                    "Failed to store adapter blob"
                                                                );
                                                            } else {
                                                                result["blob_stored"] = json!(true);
                                                                result["blob_size_bytes"] =
                                                                    json!(size);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            tracing::warn!(
                                                                target: "cns.training.adapter.blob",
                                                                adapter_id = %job_id,
                                                                error = %e,
                                                                "Failed to read adapter weights"
                                                            );
                                                        }
                                                    }
                                                }
                                                _ => {
                                                    result["blob_stored"] = json!(false);
                                                    result["blob_note"] =
                                                        json!("No local weights (cloud host)");
                                                }
                                            }

                                            tracing::info!(
                                                target: "cns.training.adapter.created",
                                                adapter_id = %job_id,
                                                "Adapter auto-registered on completion"
                                            );
                                        }
                                        Err(e) => {
                                            result["adapter_registered"] = json!(false);
                                            result["adapter_error"] = json!(e.to_string());
                                        }
                                    }
                                }
                                _ => {
                                    result["adapter_registered"] = json!(false);
                                    result["adapter_note"] = json!(
                                        "Host does not support auto-registration. Use training_register_adapter to register manually."
                                    );
                                }
                            }
                        }
                    }
                }

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
        match self.host.cancel(&job_id).await {
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
        match self.host.list_adapters().await {
            Ok(adapter_ids) => {
                let mut metadata_list: Vec<serde_json::Value> = Vec::new();
                for id in &adapter_ids {
                    let entry = match self.adapter_store.get_metadata(id).await {
                        Ok(Some(adapter)) => json!({
                            "id": adapter.id,
                            "name": adapter.name,
                            "skill_name": adapter.skill_name,
                            "version": adapter.version,
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
                    };
                    metadata_list.push(entry);
                }
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

        // Delete from host storage (filesystem)
        if let Err(e) = self.host.delete_adapter(&adapter_id).await {
            // Non-fatal — host storage may already be gone, still clean up metadata
            tracing::warn!(
                target: "cns.training.adapter.deleted",
                adapter_id = %adapter_id,
                error = %e,
                "Host deletion failed, continuing with metadata cleanup"
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

    #[tool(
        description = "Assemble stored QA pairs into a ChatML JSONL training dataset file. Queries semantic memory for training_qa_pair triples, filters by dataset/source/bloom level, and writes a file ready for training_submit. Optionally splits into train/test."
    )]
    async fn training_assemble_dataset(
        &self,
        Parameters(AssembleDatasetRequest {
            dataset,
            source,
            bloom_level,
            output_path,
            train_split,
            max_examples,
            system_prompt,
        }): Parameters<AssembleDatasetRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("training_assemble_dataset", &self.webid);

        let Some(semantic) = &self.semantic else {
            return span.error(
                McpErrorKind::PermissionDenied,
                McpToolError::permission_denied(
                    "Semantic memory not available — set HKASK_MEMORY_DB and HKASK_DB_PASSPHRASE",
                )
                .to_json_string(),
            );
        };

        let triples = match semantic.query_by_attribute("training_qa_pair") {
            Ok(t) => t,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Failed to query QA triples: {}", e))
                        .to_json_string(),
                );
            }
        };

        if triples.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(
                    "No training_qa_pair triples found in semantic memory. Ingest QA pairs first with training_ingest_qa.",
                )
                .to_json_string(),
            );
        }

        // Parse and filter QA pairs
        let mut conversations: Vec<serde_json::Value> = Vec::new();
        for triple in &triples {
            let value = &triple.value;
            let q_ds = value.get("dataset").and_then(|v| v.as_str()).unwrap_or("");
            let q_source = value.get("source").and_then(|v| v.as_str()).unwrap_or("");
            let q_bloom = value
                .get("bloom_level")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Apply filters
            if let Some(ref ds) = dataset
                && q_ds != ds.as_str()
            {
                continue;
            }
            if let Some(ref src) = source
                && q_source != src.as_str()
            {
                continue;
            }
            if let Some(ref bl) = bloom_level
                && q_bloom != bl.as_str()
            {
                continue;
            }

            let question = value.get("question").and_then(|v| v.as_str()).unwrap_or("");
            let answer = value.get("answer").and_then(|v| v.as_str()).unwrap_or("");

            if question.is_empty() || answer.is_empty() {
                continue;
            }

            let mut messages = vec![
                json!({"role": "user", "content": question}),
                json!({"role": "assistant", "content": answer}),
            ];
            if let Some(ref sys) = system_prompt {
                messages.insert(0, json!({"role": "system", "content": sys}));
            }
            conversations.push(json!({ "messages": messages }));
        }

        if conversations.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("No QA pairs matched the given filters.")
                    .to_json_string(),
            );
        }

        let total = conversations.len();
        let limit = max_examples.unwrap_or(total).min(total);
        conversations.truncate(limit);

        let train_count = if let Some(split) = train_split {
            let split = split.clamp(0.0, 1.0);
            (limit as f64 * split) as usize
        } else {
            limit
        };

        // Write training file
        let write_jsonl =
            |path: &str, items: &[serde_json::Value]| -> Result<usize, std::io::Error> {
                let mut output = String::new();
                for item in items {
                    output.push_str(&serde_json::to_string(item).unwrap());
                    output.push('\n');
                }
                std::fs::write(path, output)?;
                Ok(items.len())
            };

        let train_items = &conversations[..train_count];
        match write_jsonl(&output_path, train_items) {
            Ok(n) => {
                let mut result = json!({
                    "train_examples": n,
                    "train_path": output_path,
                    "total_matched": total,
                });

                // Write test split if requested
                if train_count < limit {
                    let test_path = format!("{}.test.jsonl", output_path);
                    let test_items = &conversations[train_count..];
                    match write_jsonl(&test_path, test_items) {
                        Ok(m) => {
                            result["test_examples"] = json!(m);
                            result["test_path"] = json!(test_path);
                        }
                        Err(e) => {
                            result["test_write_error"] = json!(e.to_string());
                        }
                    }
                }

                self.record_experience(
                    "training_assemble_dataset",
                    &output_path,
                    "success",
                    result.clone(),
                );
                span.ok_json(result)
            }
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("Failed to write dataset file: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(
        description = "Generate decomposition traces from a skill document for LoRA fine-tuning. Uses the inference engine to produce varied scenario→decomposition→synthesis training examples in ChatML format. Each trace shows the process of transforming an ill-formed situation into answerable sub-questions."
    )]
    async fn training_generate_traces(
        &self,
        Parameters(GenerateTracesRequest {
            skill_document,
            skill_name,
            num_traces,
            bloom_levels,
            output_path,
            system_prompt,
            model,
        }): Parameters<GenerateTracesRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("training_generate_traces", &self.webid);

        let count = num_traces.unwrap_or(50);
        if count == 0 {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("num_traces must be > 0").to_json_string(),
            );
        }

        // Read skill document from file or use inline text
        let skill_text = if let Ok(content) = std::fs::read_to_string(&skill_document) {
            content
        } else {
            skill_document.clone()
        };

        let levels_str = bloom_levels
            .as_ref()
            .map(|l| l.join(", "))
            .unwrap_or_else(|| {
                "remembering, understanding, applying, analyzing, evaluating, creating".to_string()
            });

        let sys = system_prompt
            .unwrap_or_else(|| format!("You are an hKask agent trained in the {skill_name} skill. Apply it precisely and thoroughly."));

        // Chunking: split large skill documents to avoid context overflow.
        // Threshold of ~6000 chars leaves room for the prompt template (~2000 chars)
        // within typical 8K context windows.
        const CHUNK_THRESHOLD: usize = 6000;
        let chunks: Vec<String> = if skill_text.len() > CHUNK_THRESHOLD {
            tracing::info!(
                target: "cns.training.trace",
                skill = %skill_name,
                size = skill_text.len(),
                "Skill document exceeds chunk threshold, splitting"
            );
            split_into_chunks(&skill_text, CHUNK_THRESHOLD)
        } else {
            vec![skill_text.clone()]
        };

        let router = InferenceRouter::new(self.inference_config.clone());
        let params = LLMParameters {
            temperature: 0.7,
            max_tokens: 4096,
            ..Default::default()
        };

        let traces_per_chunk = (count as f64 / chunks.len() as f64).ceil() as usize;
        let mut all_cleaned = String::new();
        let mut total_valid = 0usize;
        let mut total_parse_errors = 0usize;
        let mut total_tokens_used = 0u64;

        for (chunk_idx, chunk_text) in chunks.iter().enumerate() {
            let chunk_label = if chunks.len() > 1 {
                format!(" (part {} of {})", chunk_idx + 1, chunks.len())
            } else {
                String::new()
            };

            let prompt = format!(
                "You are generating training data for fine-tuning an AI agent on the '{skill_name}' skill{chunk_label}.\n\n\
                 SKILL DOCUMENT{chunk_label}:\n{chunk_text}\n\n\
                 Generate {traces_per_chunk} training examples in ChatML JSONL format. \
                 Each example must be a DECOMPOSITION TRACE: an ill-formed situation that requires \
                 the skill's process to transform it into answerable sub-questions, then synthesize a resolution.\n\n\
                 STRUCTURE OF EACH TRACE:\n\
                 1. SITUATION: Present an ill-formed problem/scenario that triggers the skill.\n\
                 2. DECOMPOSITION: Walk through the skill's process step by step, showing how each \
                    step narrows the situation into specific, answerable sub-questions.\n\
                 3. SYNTHESIS: Answer the sub-questions and resolve the original situation.\n\n\
                 TARGET BLOOM LEVELS: {levels_str}\n\n\
                 VARY ACROSS:\n\
                 - Difficulty: novice (obvious application) to expert (subtle tradeoffs, conflicting principles)\n\
                 - Scenario types: direct application, violation detection, decision justification, \
                   error recovery, multi-turn dialogue\n\
                 - Context richness: minimal (snippet only) to rich (full context with distractors)\n\n\
                 OUTPUT FORMAT: Valid JSONL with one JSON object per line. Each object must have \
                 a 'messages' array with system, user, and assistant roles:\n\
                 {{\"messages\": [\
                   {{\"role\": \"system\", \"content\": \"{sys}\"}},\n\
                   {{\"role\": \"user\", \"content\": \"<the situation>\"}},\n\
                   {{\"role\": \"assistant\", \"content\": \"<the decomposition trace + synthesis>\"}}\n\
                 ]}}\n\n\
                 Output ONLY the JSONL, no preamble or explanation."
            );

            match router
                .generate_with_model(&prompt, &params, model.as_deref())
                .await
            {
                Ok(response) => {
                    total_tokens_used += response.usage.total_tokens as u64;
                    let cleaned = response
                        .text
                        .trim()
                        .trim_start_matches("```jsonl")
                        .trim_start_matches("```json")
                        .trim_start_matches("```")
                        .trim_end_matches("```")
                        .trim();

                    // Validate and accumulate
                    for (i, line) in cleaned.lines().enumerate() {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<serde_json::Value>(trimmed) {
                            Ok(v) if v.get("messages").is_some() => {
                                total_valid += 1;
                                all_cleaned.push_str(trimmed);
                                all_cleaned.push('\n');
                            }
                            Ok(_) => {
                                total_parse_errors += 1;
                                tracing::warn!(
                                    target: "cns.training.trace",
                                    chunk = chunk_idx + 1,
                                    line = i + 1,
                                    "Trace missing 'messages' field"
                                );
                            }
                            Err(e) => {
                                total_parse_errors += 1;
                                tracing::warn!(
                                    target: "cns.training.trace",
                                    chunk = chunk_idx + 1,
                                    line = i + 1,
                                    error = %e,
                                    "Failed to parse trace line"
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        target: "cns.training.trace",
                        chunk = chunk_idx + 1,
                        error = %e,
                        "Chunk generation failed, continuing with remaining chunks"
                    );
                }
            }
        }

        if total_valid == 0 {
            return span.error(
                McpErrorKind::Internal,
                McpToolError::internal(
                    "Inference returned no valid ChatML traces across all chunks. The model may not have understood the format.",
                )
                .to_json_string(),
            );
        }

        // Write accumulated traces to output file
        match std::fs::write(&output_path, &all_cleaned) {
            Ok(()) => {
                let result = json!({
                    "skill_name": skill_name,
                    "traces_requested": count,
                    "traces_generated": total_valid,
                    "parse_errors": total_parse_errors,
                    "chunks_processed": chunks.len(),
                    "output_path": output_path,
                    "tokens_used": total_tokens_used,
                });
                self.record_experience(
                    "training_generate_traces",
                    &skill_name,
                    "success",
                    result.clone(),
                );
                span.ok_json(result)
            }
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("Failed to write traces file: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(
        description = "Evaluate a trained adapter against a test dataset. Runs inference for each test example and scores accuracy using exact match, substring containment, or semantic comparison. The model must be deployed and available for inference (Together AI fine-tuned models are auto-deployed; local adapters require the inference engine to have the adapter loaded)."
    )]
    async fn training_evaluate(
        &self,
        Parameters(TrainEvaluateRequest {
            adapter_id,
            test_dataset_path,
            model,
            method,
            max_examples,
        }): Parameters<TrainEvaluateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("training_evaluate", &self.webid);

        let test_path = PathBuf::from(&test_dataset_path);
        if !test_path.exists() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!(
                    "Test dataset file not found: {}",
                    test_dataset_path
                ))
                .to_json_string(),
            );
        }

        let raw = match std::fs::read_to_string(&test_path) {
            Ok(r) => r,
            Err(e) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(format!("Failed to read test dataset: {}", e))
                        .to_json_string(),
                );
            }
        };

        // Parse test examples: extract user message as input, last assistant message as expected
        let mut examples: Vec<(String, String)> = Vec::new();
        for (i, line) in raw.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let record: serde_json::Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(
                        target: "cns.training.evaluate",
                        line = i + 1,
                        error = %e,
                        "Skipping unparseable test line"
                    );
                    continue;
                }
            };
            let messages = match record.get("messages").and_then(|m| m.as_array()) {
                Some(ms) => ms,
                None => continue,
            };

            // Build user prompt from all user turns
            let user_parts: Vec<&str> = messages
                .iter()
                .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
                .filter_map(|m| m.get("content").and_then(|c| c.as_str()))
                .collect();
            if user_parts.is_empty() {
                continue;
            }
            let input = user_parts.join("\n");

            // Expected answer is the last assistant message
            let expected = messages
                .iter()
                .rev()
                .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("assistant"))
                .and_then(|m| m.get("content").and_then(|c| c.as_str()))
                .unwrap_or("");

            if expected.is_empty() {
                continue;
            }
            examples.push((input, expected.to_string()));
        }

        if examples.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(
                    "No valid test examples found in dataset. Each line must have a 'messages' array with user and assistant turns.",
                )
                .to_json_string(),
            );
        }

        let limit = max_examples.unwrap_or(examples.len()).min(examples.len());
        examples.truncate(limit);

        let eval_method = method.as_deref().unwrap_or("exact_match");
        let router = InferenceRouter::new(self.inference_config.clone());

        let mut correct = 0;
        let mut errors = 0;
        let mut total_tokens = 0u64;
        let mut per_example_results: Vec<serde_json::Value> = Vec::new();

        for (i, (input, expected)) in examples.iter().enumerate() {
            let prompt = format!("{input}\n\nRespond concisely and accurately.");

            let params = LLMParameters {
                temperature: 0.0, // Deterministic for evaluation
                max_tokens: 512,
                ..Default::default()
            };

            match router.generate(&prompt, &params).await {
                Ok(response) => {
                    total_tokens += response.usage.total_tokens as u64;
                    let generated = response.text.trim();
                    let expected_trimmed = expected.trim();

                    let is_correct = match eval_method {
                        "contains" => generated.contains(expected_trimmed),
                        "semantic" => {
                            // Semantic evaluation: ask the model to judge correctness
                            let judge_prompt = format!(
                                "Judge whether the following response correctly answers the question.\n\n\
                                 QUESTION:\n{input}\n\n\
                                 EXPECTED ANSWER:\n{expected_trimmed}\n\n\
                                 GENERATED ANSWER:\n{generated}\n\n\
                                 Reply with ONLY 'CORRECT' or 'INCORRECT'."
                            );
                            match router.generate(&judge_prompt, &params).await {
                                Ok(judge) => judge.text.trim().to_uppercase().contains("CORRECT"),
                                Err(_) => false,
                            }
                        }
                        _ => generated == expected_trimmed,
                    };

                    if is_correct {
                        correct += 1;
                    }

                    per_example_results.push(json!({
                        "index": i,
                        "input": input,
                        "expected": expected_trimmed,
                        "generated": generated,
                        "correct": is_correct,
                        "tokens": response.usage.total_tokens,
                    }));
                }
                Err(e) => {
                    errors += 1;
                    tracing::warn!(
                        target: "cns.training.evaluate",
                        example = i,
                        error = %e,
                        "Inference failed for test example"
                    );
                    per_example_results.push(json!({
                        "index": i,
                        "input": input,
                        "expected": expected.trim(),
                        "error": e.to_string(),
                    }));
                }
            }
        }

        let total = correct + errors;
        let accuracy = if total > 0 {
            correct as f64 / total as f64
        } else {
            0.0
        };

        let result = json!({
            "adapter_id": adapter_id,
            "model": model,
            "method": eval_method,
            "total_examples": total,
            "correct": correct,
            "errors": errors,
            "accuracy": accuracy,
            "total_tokens_used": total_tokens,
            "per_example": per_example_results,
        });

        self.record_experience("training_evaluate", &adapter_id, "success", result.clone());
        span.ok_json(result)
    }

    #[tool(
        description = "Register a completed LoRA adapter in the persistent store. Call after training completes to record adapter metadata for future listing, evaluation, and composition. Stores both metadata and links the adapter to its originating training job."
    )]
    async fn training_register_adapter(
        &self,
        Parameters(TrainRegisterAdapterRequest {
            adapter_id,
            name,
            skill_name,
            base_model,
            dataset_hash,
            training_job_id,
            size_bytes,
            loss,
            perplexity,
            training_duration_secs,
            tokens_processed,
            version,
        }): Parameters<TrainRegisterAdapterRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("training_register_adapter", &self.webid);

        let metrics = if loss.is_some()
            || perplexity.is_some()
            || training_duration_secs.is_some()
            || tokens_processed.is_some()
        {
            Some(AdapterMetrics {
                loss,
                perplexity,
                training_duration_secs,
                tokens_processed,
            })
        } else {
            None
        };

        let adapter = LoRAAdapter {
            id: adapter_id.clone(),
            name: name.clone(),
            base_model: base_model.clone(),
            dataset_hash: dataset_hash.unwrap_or_default(),
            training_job_id: training_job_id.unwrap_or_default(),
            created_at: chrono::Utc::now().timestamp(),
            size_bytes: size_bytes.unwrap_or(0),
            skill_name: skill_name.clone(),
            version: version.unwrap_or(1),
            metrics,
        };

        match self.adapter_store.store_metadata(&adapter).await {
            Ok(()) => {
                let result = json!({
                    "adapter_id": adapter_id,
                    "name": name,
                    "skill_name": skill_name,
                    "base_model": base_model,
                    "registered": true,
                });
                self.record_experience(
                    "training_register_adapter",
                    &adapter_id,
                    "success",
                    result.clone(),
                );
                span.ok_json(result)
            }
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("Failed to register adapter: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(
        description = "Recommend a base model for fine-tuning based on task type, budget, latency, and license requirements. Returns ranked recommendations with rationale to guide model selection before calling training_submit."
    )]
    async fn training_recommend_model(
        &self,
        Parameters(TrainRecommendModelRequest {
            task_type,
            budget,
            latency,
            license,
            provider,
        }): Parameters<TrainRecommendModelRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("training_recommend_model", &self.webid);

        // Model knowledge base — ranked recommendations per task type
        // Updated 2026-06. Weights: license freedom, provider availability, cost, capability.
        let recommendations: Vec<serde_json::Value> = match task_type.to_lowercase().as_str() {
            "classification" => vec![
                json!({
                    "rank": 1, "model": "Qwen3.5-9B", "provider_prefix": "TOGETHER",
                    "rationale": "Strong instruction-following, Apache 2.0 license, broad provider support. Excellent for constraint classification and categorical tasks. ~$0.005/LoRA run on Together AI.",
                    "license": "apache2", "size": "9B", "cost_per_run": "~$0.005"
                }),
                json!({
                    "rank": 2, "model": "Llama-4-8B", "provider_prefix": "TOGETHER",
                    "rationale": "Latest Llama architecture, strong reasoning. Good for classification with nuanced categories. Slightly more expensive than Qwen3.5.",
                    "license": "llama4", "size": "8B", "cost_per_run": "~$0.01"
                }),
                json!({
                    "rank": 3, "model": "DeepSeek-V3-7B", "provider_prefix": "TOGETHER",
                    "rationale": "Excellent reasoning capabilities, strong for multi-step classification. MIT license. Good for procedural classification tasks.",
                    "license": "mit", "size": "7B", "cost_per_run": "~$0.008"
                }),
            ],
            "generation" => vec![
                json!({
                    "rank": 1, "model": "Qwen3.5-14B", "provider_prefix": "TOGETHER",
                    "rationale": "Larger variant with stronger generation capabilities. Apache 2.0. Good for trace generation and synthetic data creation.",
                    "license": "apache2", "size": "14B", "cost_per_run": "~$0.02"
                }),
                json!({
                    "rank": 2, "model": "Llama-4-12B", "provider_prefix": "TOGETHER",
                    "rationale": "Strong creative generation, good for diverse synthetic data. Llama 4 community license.",
                    "license": "llama4", "size": "12B", "cost_per_run": "~$0.03"
                }),
            ],
            "procedural" => vec![
                json!({
                    "rank": 1, "model": "Qwen3.5-9B", "provider_prefix": "TOGETHER",
                    "rationale": "Best cost/capability balance for procedural skill training. Apache 2.0. Proven with hKask constraint-forces and essentialist adapters.",
                    "license": "apache2", "size": "9B", "cost_per_run": "~$0.005"
                }),
                json!({
                    "rank": 2, "model": "DeepSeek-V3-7B", "provider_prefix": "TOGETHER",
                    "rationale": "Strong at following multi-step procedures. MIT license. Good for diagnose and tdd skill adapters.",
                    "license": "mit", "size": "7B", "cost_per_run": "~$0.008"
                }),
            ],
            "reasoning" => vec![
                json!({
                    "rank": 1, "model": "DeepSeek-V3-7B", "provider_prefix": "TOGETHER",
                    "rationale": "Top-tier reasoning capabilities. MIT license. Best for pragmatic-semantics, essentialist, and other analysis-heavy skills.",
                    "license": "mit", "size": "7B", "cost_per_run": "~$0.008"
                }),
                json!({
                    "rank": 2, "model": "Qwen3.5-9B", "provider_prefix": "TOGETHER",
                    "rationale": "Strong reasoning with broader provider support. Apache 2.0. Good fallback if DeepSeek is unavailable.",
                    "license": "apache2", "size": "9B", "cost_per_run": "~$0.005"
                }),
            ],
            "chat" => vec![
                json!({
                    "rank": 1, "model": "Qwen3.5-9B", "provider_prefix": "TOGETHER",
                    "rationale": "Well-rounded chat capabilities, Apache 2.0, broad provider support. Good general-purpose base for agent conversation skills.",
                    "license": "apache2", "size": "9B", "cost_per_run": "~$0.005"
                }),
                json!({
                    "rank": 2, "model": "Llama-4-8B", "provider_prefix": "TOGETHER",
                    "rationale": "Natural conversational tone, strong instruction following. Good for improv and coaching adapters.",
                    "license": "llama4", "size": "8B", "cost_per_run": "~$0.01"
                }),
            ],
            _ => vec![json!({
                "rank": 1, "model": "Qwen3.5-9B", "provider_prefix": "TOGETHER",
                "rationale": "Default recommendation: best all-around balance of capability, cost, and license freedom (Apache 2.0). Proven with hKask skill adapters.",
                "license": "apache2", "size": "9B", "cost_per_run": "~$0.005"
            })],
        };

        // Apply filters
        let latency_filter = latency.as_deref().unwrap_or("flexible");
        let budget_filter = budget.as_deref().unwrap_or("medium");
        let license_filter = license.as_deref().unwrap_or("any");
        let provider_filter = provider.as_deref().unwrap_or("any");

        let filtered: Vec<&serde_json::Value> = recommendations
            .iter()
            .filter(|r| {
                // Budget filter
                let cost = r.get("cost_per_run").and_then(|c| c.as_str()).unwrap_or("");
                match budget_filter {
                    "low" => cost.contains("$0.005"),
                    "medium" => !cost.contains("$0.03") && !cost.contains("$0.05"),
                    _ => true,
                }
            })
            .filter(|r| {
                // License filter
                if license_filter == "any" {
                    return true;
                }
                let lic = r.get("license").and_then(|l| l.as_str()).unwrap_or("");
                match license_filter {
                    "apache2" => lic == "apache2",
                    "mit" => lic == "mit" || lic == "apache2",
                    "open" => lic != "llama4",
                    _ => true,
                }
            })
            .filter(|r| {
                // Provider filter
                if provider_filter == "any" {
                    return true;
                }
                let pref = r
                    .get("provider_prefix")
                    .and_then(|p| p.as_str())
                    .unwrap_or("");
                pref.to_lowercase()
                    .contains(&provider_filter.to_lowercase())
            })
            .collect();

        let result = json!({
            "task_type": task_type,
            "filters_applied": {
                "budget": budget_filter,
                "latency": latency_filter,
                "license": license_filter,
                "provider": provider_filter,
            },
            "recommendations": filtered,
            "guidance": "For hKask skill adapters, Qwen3.5-9B on Together AI is the recommended default: Apache 2.0 license, ~$0.005 per LoRA run, 4-7 minute training time, and proven with constraint-forces (100% accuracy). Use DeepSeek-V3-7B for reasoning-heavy skills (pragmatic-semantics, essentialist). Use Qwen3.5-14B for generation-heavy skills (trace generation).",
        });

        span.ok_json(result)
    }

    #[tool(
        description = "Record an adapter invocation as an episodic experience for future training data curation. Stores input/output summaries with CNS span correlation and confidence. This is the first step in the continuous training loop — recorded invocations feed into training_curate_feedback and training_retrain."
    )]
    async fn training_record_invocation(
        &self,
        Parameters(TrainRecordInvocationRequest {
            adapter_id,
            skill_name,
            input_summary,
            output_summary,
            cns_span,
            confidence,
            success,
        }): Parameters<TrainRecordInvocationRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("training_record_invocation", &self.webid);

        let Some(ref daemon) = self.daemon else {
            return span.error(
                McpErrorKind::PermissionDenied,
                McpToolError::permission_denied(
                    "Daemon not available — episodic memory storage requires the hKask daemon",
                )
                .to_json_string(),
            );
        };

        let value = json!({
            "adapter_id": adapter_id,
            "skill_name": skill_name,
            "input_summary": input_summary,
            "output_summary": output_summary,
            "cns_span": cns_span,
            "success": success.unwrap_or(true),
            "timestamp": now_rfc3339(),
        });

        let conf = confidence.unwrap_or(0.85);

        match daemon
            .store_experience(
                &self.replicant,
                "adapter_invocation",
                "observed",
                &value,
                Some(conf),
            )
            .await
        {
            Ok(hkask_mcp::DaemonResponse::StoreResponse { stored: true, .. }) => {
                let result = json!({
                    "adapter_id": adapter_id,
                    "skill_name": skill_name,
                    "recorded": true,
                    "confidence": conf,
                });
                tracing::debug!(
                    target: "cns.training.invoke",
                    adapter_id = %adapter_id,
                    skill = %skill_name,
                    "Adapter invocation recorded"
                );
                span.ok_json(result)
            }
            Ok(other) => {
                tracing::warn!(
                    target: "cns.training.invoke",
                    adapter_id = %adapter_id,
                    response = ?other,
                    "Unexpected daemon response"
                );
                span.ok_json(json!({
                    "adapter_id": adapter_id,
                    "recorded": false,
                    "warning": "Unexpected daemon response"
                }))
            }
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("Failed to record invocation: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(
        description = "Curate feedback from stored QA pairs for continuous skills training. Queries semantic memory for training_qa_pair triples, validates each answer with inference, and generates corrected ChatML traces where the original answer is wrong or incomplete. Outputs a feedback JSONL file ready for training_retrain."
    )]
    async fn training_curate_feedback(
        &self,
        Parameters(TrainCurateFeedbackRequest {
            dataset,
            source,
            output_path,
            model,
            max_pairs,
        }): Parameters<TrainCurateFeedbackRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("training_curate_feedback", &self.webid);

        let Some(semantic) = &self.semantic else {
            return span.error(
                McpErrorKind::PermissionDenied,
                McpToolError::permission_denied(
                    "Semantic memory not available — set HKASK_MEMORY_DB and HKASK_DB_PASSPHRASE",
                )
                .to_json_string(),
            );
        };

        let triples = match semantic.query_by_attribute("training_qa_pair") {
            Ok(t) => t,
            Err(e) => {
                return span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Failed to query QA triples: {}", e))
                        .to_json_string(),
                );
            }
        };

        if triples.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(
                    "No training_qa_pair triples found. Ingest QA pairs first with training_ingest_qa.",
                )
                .to_json_string(),
            );
        }

        // Filter and collect QA pairs
        let mut pairs: Vec<(String, String)> = Vec::new();
        for triple in &triples {
            let value = &triple.value;
            let q_ds = value.get("dataset").and_then(|v| v.as_str()).unwrap_or("");
            let q_source = value.get("source").and_then(|v| v.as_str()).unwrap_or("");

            if let Some(ref ds) = dataset
                && q_ds != ds.as_str()
            {
                continue;
            }
            if let Some(ref src) = source
                && q_source != src.as_str()
            {
                continue;
            }

            let question = value.get("question").and_then(|v| v.as_str()).unwrap_or("");
            let answer = value.get("answer").and_then(|v| v.as_str()).unwrap_or("");
            if !question.is_empty() && !answer.is_empty() {
                pairs.push((question.to_string(), answer.to_string()));
            }
        }

        let limit = max_pairs.unwrap_or(50).min(pairs.len());
        pairs.truncate(limit);

        if pairs.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("No QA pairs matched the given filters.")
                    .to_json_string(),
            );
        }

        let router = InferenceRouter::new(self.inference_config.clone());
        let params = LLMParameters {
            temperature: 0.0,
            max_tokens: 1024,
            ..Default::default()
        };

        let mut corrected_traces: Vec<serde_json::Value> = Vec::new();
        let mut reviewed = 0usize;
        let mut corrections = 0usize;
        let mut total_tokens = 0u64;

        for (question, original_answer) in &pairs {
            reviewed += 1;

            // Validate: ask the model to judge if the original answer is correct
            let judge_prompt = format!(
                "Review this QA pair for correctness and completeness.\n\n\
                 QUESTION:\n{question}\n\n\
                 ORIGINAL ANSWER:\n{original_answer}\n\n\
                 If the answer is CORRECT and COMPLETE, reply with ONLY 'PASS'.\n\
                 If the answer is wrong, incomplete, or could be improved, reply with 'FAIL: <brief reason>'\n\
                 then provide the CORRECTED ANSWER on the next line prefixed with 'CORRECTED: '."
            );

            match router
                .generate_with_model(&judge_prompt, &params, model.as_deref())
                .await
            {
                Ok(response) => {
                    total_tokens += response.usage.total_tokens as u64;
                    let judge_text = response.text.trim();

                    if judge_text.starts_with("PASS") {
                        // Answer is fine — include as-is in feedback
                        corrected_traces.push(json!({
                            "messages": [
                                {"role": "user", "content": question},
                                {"role": "assistant", "content": original_answer}
                            ],
                            "review": "passed"
                        }));
                    } else {
                        // Extract corrected answer
                        let corrected = judge_text
                            .lines()
                            .find(|l| l.starts_with("CORRECTED:"))
                            .map(|l| l.trim_start_matches("CORRECTED:").trim())
                            .unwrap_or(original_answer.as_str());

                        corrections += 1;
                        corrected_traces.push(json!({
                            "messages": [
                                {"role": "user", "content": question},
                                {"role": "assistant", "content": corrected}
                            ],
                            "review": "corrected",
                            "original_answer": original_answer,
                            "judge_notes": judge_text
                        }));

                        tracing::info!(
                            target: "cns.training.curate",
                            question = %question.chars().take(80).collect::<String>(),
                            "Correction generated"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        target: "cns.training.curate",
                        error = %e,
                        "Validation inference failed, keeping original"
                    );
                    corrected_traces.push(json!({
                        "messages": [
                            {"role": "user", "content": question},
                            {"role": "assistant", "content": original_answer}
                        ],
                        "review": "unreviewed (inference error)"
                    }));
                }
            }
        }

        // Write feedback file
        let mut output = String::new();
        for trace in &corrected_traces {
            output.push_str(&serde_json::to_string(trace).unwrap());
            output.push('\n');
        }

        match std::fs::write(&output_path, &output) {
            Ok(()) => {
                let result = json!({
                    "output_path": output_path,
                    "pairs_reviewed": reviewed,
                    "corrections": corrections,
                    "pass_rate": if reviewed > 0 {
                        (reviewed - corrections) as f64 / reviewed as f64
                    } else {
                        1.0
                    },
                    "tokens_used": total_tokens,
                });
                self.record_experience(
                    "training_curate_feedback",
                    &output_path,
                    "success",
                    result.clone(),
                );
                span.ok_json(result)
            }
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("Failed to write feedback file: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(
        description = "Retrain an adapter with curated feedback for continuous skills training. Merges the original training dataset with a feedback JSONL file (from training_curate_feedback), submits a new training job with an incremented version number, and registers the new adapter on completion. This closes the continuous training loop: train → evaluate → curate → retrain."
    )]
    async fn training_retrain(
        &self,
        Parameters(TrainRetrainRequest {
            original_dataset_path,
            feedback_path,
            base_model,
            adapter_name,
            skill_name,
            params,
            merged_output_path,
        }): Parameters<TrainRetrainRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("training_retrain", &self.webid);

        // Validate input files exist
        let original = PathBuf::from(&original_dataset_path);
        if !original.exists() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!(
                    "Original dataset not found: {}",
                    original_dataset_path
                ))
                .to_json_string(),
            );
        }

        let feedback = PathBuf::from(&feedback_path);
        if !feedback.exists() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!(
                    "Feedback file not found: {}",
                    feedback_path
                ))
                .to_json_string(),
            );
        }

        // Read and merge datasets
        let original_content = match std::fs::read_to_string(&original) {
            Ok(c) => c,
            Err(e) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(format!(
                        "Failed to read original dataset: {}",
                        e
                    ))
                    .to_json_string(),
                );
            }
        };

        let feedback_content = match std::fs::read_to_string(&feedback) {
            Ok(c) => c,
            Err(e) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(format!("Failed to read feedback file: {}", e))
                        .to_json_string(),
                );
            }
        };

        // Merge: original lines + feedback lines, deduplicate by question content
        let mut merged = String::new();
        let mut seen_questions: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for content in [&original_content, &feedback_content] {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                // Extract question for dedup
                if let Ok(record) = serde_json::from_str::<serde_json::Value>(trimmed)
                    && let Some(messages) = record.get("messages").and_then(|m| m.as_array())
                {
                    let question = messages
                        .iter()
                        .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
                        .and_then(|m| m.get("content").and_then(|c| c.as_str()))
                        .unwrap_or("");
                    if !question.is_empty() && seen_questions.insert(question.to_string()) {
                        merged.push_str(trimmed);
                        merged.push('\n');
                    }
                }
            }
        }

        if merged.is_empty() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument("No valid examples found in either dataset")
                    .to_json_string(),
            );
        }

        // Write merged dataset
        let merged_path = merged_output_path
            .unwrap_or_else(|| format!("/tmp/hkask-retrain-{}.jsonl", &adapter_name));

        if let Err(e) = std::fs::write(&merged_path, &merged) {
            return span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("Failed to write merged dataset: {}", e))
                    .to_json_string(),
            );
        }

        // Determine version: look up previous adapter version and increment
        let version = match self.adapter_store.get_metadata(&skill_name).await {
            Ok(Some(prev)) => prev.version + 1,
            _ => 1,
        };

        // Ingest and normalize the merged dataset
        let normalized_path = match self
            .pipeline
            .lock()
            .unwrap()
            .ingest(&PathBuf::from(&merged_path))
        {
            Ok(path) => path,
            Err(e) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(format!("Dataset pipeline error: {}", e))
                        .to_json_string(),
                );
            }
        };

        let job = TrainingJob {
            id: uuid::Uuid::new_v4().to_string(),
            dataset_path: normalized_path.clone(),
            base_model: base_model.clone(),
            params: params.unwrap_or_default(),
            status: TrainingJobStatus::Queued,
            created_at: chrono::Utc::now(),
            host: self.host_id,
        };

        // Persist job
        if let Some(ref job_store) = self.job_store {
            let params_json = serde_json::to_string(&job.params).unwrap_or_default();
            let _ = job_store.store(
                &job.id,
                &job.base_model,
                &job.dataset_path.to_string_lossy(),
                &params_json,
                "queued",
                job.created_at.timestamp(),
                &format!("{:?}", job.host).to_lowercase(),
            );
        }

        // Pre-register the adapter metadata so it's ready when training completes
        let adapter = LoRAAdapter {
            id: job.id.clone(),
            name: adapter_name.clone(),
            base_model: base_model.clone(),
            dataset_hash: String::new(),
            training_job_id: job.id.clone(),
            created_at: chrono::Utc::now().timestamp(),
            size_bytes: 0,
            skill_name: skill_name.clone(),
            version,
            metrics: None,
        };

        if let Err(e) = self.adapter_store.store_metadata(&adapter).await {
            tracing::warn!(
                target: "cns.training.retrain",
                adapter_id = %job.id,
                error = %e,
                "Failed to pre-register adapter metadata"
            );
        }

        match self.host.submit(&job).await {
            Ok(job_id) => {
                let result = json!({
                    "job_id": job_id,
                    "status": "queued",
                    "base_model": base_model,
                    "adapter_name": adapter_name,
                    "skill_name": skill_name,
                    "version": version,
                    "merged_dataset": merged_path,
                    "original_examples": original_content.lines().filter(|l| !l.trim().is_empty()).count(),
                    "feedback_examples": feedback_content.lines().filter(|l| !l.trim().is_empty()).count(),
                    "merged_examples": merged.lines().filter(|l| !l.trim().is_empty()).count(),
                    "host": format!("{:?}", self.host_id),
                });
                self.record_experience(
                    "training_retrain",
                    &adapter_name,
                    "success",
                    result.clone(),
                );
                span.ok_json(result)
            }
            Err(e) => {
                tracing::error!(
                    target: "cns.training.job.fail",
                    error = %e,
                    "Retraining job submission failed"
                );
                span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("Retraining job failed: {}", e))
                        .to_json_string(),
                )
            }
        }
    }

    #[tool(
        description = "Ingest a raw dataset file into the normalized cache without submitting a training job. Detects format (ChatML, ShareGPT, Alpaca, raw text), normalizes to canonical ChatML, validates, and caches. Returns the cached path for use with training_submit or training_assemble_dataset."
    )]
    async fn training_ingest_dataset(
        &self,
        Parameters(TrainIngestDatasetRequest {
            dataset_path,
            cache_dir,
        }): Parameters<TrainIngestDatasetRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("training_ingest_dataset", &self.webid);

        let file_path = PathBuf::from(&dataset_path);
        if !file_path.exists() {
            return span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!("Dataset file not found: {}", dataset_path))
                    .to_json_string(),
            );
        }

        // Use provided cache dir or create a pipeline with the default
        let mut pipeline = if let Some(ref dir) = cache_dir {
            DatasetPipeline::new(PathBuf::from(dir))
        } else {
            self.pipeline.lock().unwrap().clone()
        };

        let format = hkask_mcp_training::dataset::DatasetFormat::detect(&file_path);

        match pipeline.ingest(&file_path) {
            Ok(normalized_path) => {
                let result = json!({
                    "dataset_path": dataset_path,
                    "normalized_path": normalized_path.to_string_lossy(),
                    "detected_format": format.map(|f| format!("{:?}", f)).unwrap_or_else(|| "unknown".to_string()),
                    "cached": true,
                });
                self.record_experience(
                    "training_ingest_dataset",
                    &dataset_path,
                    "success",
                    result.clone(),
                );
                span.ok_json(result)
            }
            Err(e) => span.error(
                McpErrorKind::InvalidArgument,
                McpToolError::invalid_argument(format!("Dataset ingest error: {}", e))
                    .to_json_string(),
            ),
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────

/// Split text into chunks at paragraph boundaries, each under `max_chars`.
/// Splits at double-newline boundaries first, then falls back to single-newline
/// if a paragraph exceeds the limit.
fn split_into_chunks(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    let mut current = String::new();

    for para in paragraphs {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        if current.len() + para.len() + 2 > max_chars && !current.is_empty() {
            chunks.push(current.trim().to_string());
            current = String::new();
        }
        if !current.is_empty() {
            current.push_str("\n\n");
        }
        current.push_str(para);

        // If a single paragraph exceeds the limit, split by sentences (newlines within)
        while current.len() > max_chars {
            if let Some(split_point) = current[..max_chars].rfind('\n') {
                let take = current[..split_point].trim().to_string();
                if !take.is_empty() {
                    chunks.push(take);
                }
                current = current[split_point + 1..].trim().to_string();
            } else {
                // No newline found — hard split at max_chars
                let take = current[..max_chars].trim().to_string();
                if !take.is_empty() {
                    chunks.push(take);
                }
                current = current[max_chars..].trim().to_string();
            }
        }
    }

    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }

    if chunks.is_empty() {
        vec![text.to_string()]
    } else {
        chunks
    }
}

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

    // Resolve host config from environment
    let host_id = std::env::var("HKASK_TRAINING_HOST")
        .ok()
        .and_then(|s| TrainingHostId::from_str(&s))
        .unwrap_or(TrainingHostId::Together);
    let harness_id = std::env::var("HKASK_TRAINING_HARNESS")
        .ok()
        .and_then(|s| TrainingHarnessId::from_str(&s))
        .unwrap_or(TrainingHarnessId::Axolotl);
    let cloud_dispatch = std::env::var("HKASK_TRAINING_CLOUD_DISPATCH")
        .map(|s| s == "1" || s == "true")
        .unwrap_or(false);
    let host_config = TrainingHostConfig {
        harness: harness_id,
        host: host_id,
        axolotl_path: std::env::var("HKASK_AXOLOTL_PATH").ok().map(PathBuf::from),
        python_path: std::env::var("HKASK_PYTHON_PATH").ok().map(PathBuf::from),
        cloud_dispatch,
        together_api_key: std::env::var("TOGETHER_API_KEY").unwrap_or_default(),
        runpod_api_key: std::env::var("RUNPOD_API_KEY").unwrap_or_default(),
        runpod_template_id: std::env::var("RUNPOD_TEMPLATE_ID").unwrap_or_default(),
        baseten_api_key: std::env::var("BASETEN_API_KEY").unwrap_or_default(),
        baseten_project_id: std::env::var("BASETEN_PROJECT_ID").unwrap_or_default(),
    };

    let cache_dir = PathBuf::from(
        std::env::var("HKASK_TRAINING_CACHE_DIR")
            .unwrap_or_else(|_| "/tmp/hkask-training-cache".to_string()),
    );
    let pipeline = DatasetPipeline::new(cache_dir);

    hkask_mcp::run_server(
        "hkask-mcp-training",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            let (semantic, adapter_store, job_store) = match ctx.credentials.get("HKASK_MEMORY_DB")
            {
                Some(path) => {
                    let passphrase =
                        ctx.credentials.get("HKASK_DB_PASSPHRASE").ok_or_else(|| {
                            anyhow::anyhow!("HKASK_MEMORY_DB set but HKASK_DB_PASSPHRASE missing")
                        })?;
                    let db = hkask_storage::Database::open(path, passphrase)
                        .map_err(|e| anyhow::anyhow!("Failed to open memory database: {}", e))?;
                    let conn = db.conn_arc();
                    let job_store = Some(JobStore::new(Arc::clone(&conn)));
                    let triple_store = hkask_storage::TripleStore::new(Arc::clone(&conn));
                    let embedding_store = hkask_storage::EmbeddingStore::new(Arc::clone(&conn));
                    let semantic = Some(hkask_memory::SemanticMemory::new(
                        triple_store,
                        embedding_store,
                    ));
                    let store = SqliteAdapterStore::new(db);
                    store
                        .migrate()
                        .map_err(|e| anyhow::anyhow!("Failed to migrate adapter store: {}", e))?;
                    (
                        semantic,
                        Arc::new(store) as Arc<dyn AdapterStore>,
                        job_store,
                    )
                }
                None => (
                    None,
                    Arc::new(InMemoryAdapterStore::new()) as Arc<dyn AdapterStore>,
                    None,
                ),
            };

            let host = create_host(&host_config)
                .map_err(|e| anyhow::anyhow!("Failed to create training host: {}", e))?;

            let inference_config = InferenceConfig::from_env();

            Ok(TrainingServer::new(
                ctx.webid,
                replicant.clone(),
                daemon_client.clone(),
                semantic,
                host,
                host_config.host,
                pipeline.clone(),
                adapter_store,
                job_store,
                inference_config,
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
    let result = hkask_mcp::verify_startup_gates(&client, replicant, "training", &[]).await?;
    tracing::info!(target: "hkask.mcp.training", replicant = %replicant,
        "P4 gates verified{}",
        if result.denied_tools.is_empty() { String::new() }
        else { format!(" — {} tool(s) denied: {:?}", result.denied_tools.len(), result.denied_tools) }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REQ: training-chunk-01 — split_into_chunks handles text under threshold
    #[test]
    fn chunk_under_threshold_returns_single() {
        let text = "Short skill document.\n\nWith two paragraphs.";
        let chunks = split_into_chunks(text, 6000);
        assert_eq!(chunks.len(), 1, "text under threshold should be one chunk");
        assert!(chunks[0].contains("Short skill document"));
        assert!(chunks[0].contains("With two paragraphs"));
    }

    /// REQ: training-chunk-02 — split_into_chunks splits at paragraph boundaries
    #[test]
    fn chunk_splits_at_paragraphs() {
        let para = "A".repeat(100);
        let text = format!("{}\n\n{}\n\n{}", para, para, para);
        let chunks = split_into_chunks(&text, 150);
        assert!(chunks.len() >= 2, "should split across paragraphs");
        for chunk in &chunks {
            assert!(!chunk.is_empty(), "no empty chunks");
        }
    }

    /// REQ: training-chunk-03 — split_into_chunks handles empty input
    #[test]
    fn chunk_empty_returns_single() {
        let chunks = split_into_chunks("", 100);
        assert_eq!(chunks.len(), 1);
    }

    /// REQ: training-chunk-04 — split_into_chunks preserves content across chunks
    #[test]
    fn chunk_preserves_content() {
        let p1 = "First paragraph about constraints.";
        let p2 = "Second paragraph about guardrails.";
        let p3 = "Third paragraph about guidelines.";
        let text = format!("{p1}\n\n{p2}\n\n{p3}");
        let chunks = split_into_chunks(&text, 50);
        let combined: String = chunks
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(combined.contains("constraints"));
        assert!(combined.contains("guardrails"));
        assert!(combined.contains("guidelines"));
    }
}
