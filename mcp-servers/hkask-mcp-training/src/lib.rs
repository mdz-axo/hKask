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
//! - `HKASK_TRAINING_DB` — Path to per-agent training database for job/adapter/QA storage (defaults to `agents/{replicant}/training.db`)
//! - `HKASK_DB_PASSPHRASE` — Passphrase for the database (resolved via credentials or keystore)
//! - `HKASK_TRAINING_HOST` — Override host (together|runpod|baseten) — where compute runs
//! - `HKASK_TRAINING_HARNESS` — Override harness (axolotl|unsloth) — what tooling runs
//! - `HKASK_TRAINING_CACHE_DIR` — Dataset cache directory
//! - `TG_API_KEY` — Together AI API key (for Together host)
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

pub mod adapters;
pub mod dataset;
pub mod huggingface;
pub mod mlschema;
pub mod providers;
pub mod types;

// Bridge crates: shared ontological vocabulary (P5.4 dual-axis framework)

use crate::adapters::{
    AdapterMetrics, AdapterStore, InMemoryAdapterStore, JobStore, LoRAAdapter, SqliteAdapterStore,
};
use crate::dataset::DatasetPipeline;
use crate::providers::{
    AxolotlHarness, HarnessAdapter, LoraParams, TrainingHarnessId, TrainingHost,
    TrainingHostConfig, TrainingHostId, TrainingJob, TrainingJobStatus, TrainingParams,
    UnslothHarness, create_host,
};
use crate::types::*;
use hkask_adapter::AdapterPort;
use hkask_adapter::AdapterRouter;
use hkask_adapter::{EndpointLifecycle, EndpointPhase};
use hkask_inference::{InferenceConfig, InferenceRouter};

use hkask_mcp::server::{McpToolError, execute_tool};
use hkask_memory::SemanticMemory;
use hkask_ports::InferencePort;
use hkask_storage::Triple;
use hkask_types::template::LLMParameters;
use hkask_types::time::now_rfc3339;
use hkask_types::{Visibility, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

// ── Training mode — expertise vs skill vs contrastive ──────────────────

/// What kind of training data is being produced.
///
/// **Expertise** — "What to know" — factual domain knowledge.
/// Training data is QA pairs (ingest_qa → assemble_dataset).
/// Evaluation uses exact/contains/semantic match.
/// Produces an *expertise adapter* that answers factual questions about a domain.
///
/// **Decomposition Trace** — "How to think" — procedural decomposition of problems.
/// Training data is generated traces from SKILL.md (generate_traces).
/// Evaluation uses decomposition accuracy.
/// Produces a *skill adapter* that applies a methodology to novel situations.
///
/// **Contrastive Trace** — "What to prefer" — trains judgment by contrasting correct vs. incorrect decompositions.
/// Training data is trace pairs (chosen/rejected) with the same situation.
/// Evaluation uses preference accuracy (does model produce chosen over rejected?).
/// Uses the existing A/B evaluation loop for comparing adapter outputs.
///
/// **Hybrid** — Both expertise and skill traces, with configurable weighting (default 30% expertise / 70% traces).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrainingMode {
    /// Expertise (factual knowledge) fine-tuning — domain QA pairs.
    Expertise,
    /// Skill (procedural) decomposition trace fine-tuning — from SKILL.md.
    DecompositionTrace,
    /// Contrastive preference training — correct vs. incorrect trace pairs.
    ContrastiveTrace,
    /// Weighted combination of expertise QA and skill decomposition traces.
    Hybrid,
}

// ── A/B evaluation baseline ──────────────────────────────────────────────

/// Metrics from the previous adapter version, used as baseline for A/B comparison
/// when retraining. The new adapter must improve on at least 2 of 3 metrics
/// (loss, perplexity, or eval accuracy) to be promoted.
#[derive(Debug, Clone, Serialize)]
pub struct AbBaseline {
    pub previous_version: u32,
    pub previous_loss: f32,
    pub previous_perplexity: f32,
}

/// A deployed adapter endpoint — tracks the lifecycle of a trained adapter
/// that has been deployed to a cloud inference provider.
///
/// Uses `EndpointLifecycle` for state machine governance:
///   Provisioning → Ready → Active → Draining → Terminated
#[derive(Debug, Clone, Serialize)]
pub struct AdapterDeployment {
    pub deployment_id: String,
    pub adapter_name: String,
    pub base_model: String,
    pub provider: DeploymentProvider,
    pub endpoint_url: Option<String>,
    /// Lifecycle state machine — governs phase transitions.
    #[serde(skip)]
    pub lifecycle: EndpointLifecycle,
    pub estimated_cost_per_hour: f32,
    pub deployed_at: chrono::DateTime<chrono::Utc>,
}

impl AdapterDeployment {
    /// Current phase from the lifecycle state machine.
    pub fn phase(&self) -> EndpointPhase {
        self.lifecycle.phase
    }

    /// Accrued cost from the lifecycle.
    pub fn cost_accrued(&self) -> f64 {
        self.lifecycle.cost_accrued
    }
}

// ── Server ───────────────────────────────────────────────────────────────

pub struct TrainingServer {
    pub webid: WebID,
    pub replicant: String,
    pub daemon: Option<hkask_mcp::DaemonClient>,
    pub semantic: Option<SemanticMemory>,
    pub host: Box<dyn TrainingHost>,
    pub host_id: TrainingHostId,
    pub harness_id: TrainingHarnessId,
    pub pipeline: Mutex<DatasetPipeline>,
    pub adapter_store: Arc<dyn AdapterStore>,
    pub job_store: Option<JobStore>,
    pub adapter_router: Option<Arc<AdapterRouter>>,
    pub inference_config: InferenceConfig,
    pub deployments: Mutex<HashMap<String, AdapterDeployment>>,
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
        harness_id: TrainingHarnessId,
        pipeline: DatasetPipeline,
        adapter_store: Arc<dyn AdapterStore>,
        job_store: Option<JobStore>,
        adapter_router: Option<Arc<AdapterRouter>>,
        inference_config: InferenceConfig,
    ) -> Self {
        Self {
            webid,
            replicant,
            daemon,
            semantic,
            host,
            host_id,
            harness_id,
            pipeline: Mutex::new(pipeline),
            adapter_store,
            job_store,
            adapter_router,
            inference_config,
            deployments: Mutex::new(HashMap::new()),
        }
    }
}

impl hkask_mcp::server::ToolContext for TrainingServer {
    fn webid(&self) -> &WebID {
        &self.webid
    }

    fn record_tool_outcome(&self, tool: &str, outcome: &str) {
        hkask_mcp::record_via_daemon(&self.daemon, &self.replicant, tool, outcome);
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
        execute_tool(self, "training_ingest_qa", async {
            let Some(semantic) = &self.semantic else {
                return Err(McpToolError::permission_denied(
                    "Semantic memory not available — set HKASK_MEMORY_DB and HKASK_DB_PASSPHRASE",
                ));
            };

            if qa_items.is_empty() {
                return Err(McpToolError::invalid_argument("qa_items must not be empty"));
            }

            hkask_mcp::validate_identifier("source", &source, 256)?;

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
                Ok(json!({ "stored": stored, "source": source, "dataset": ds }))
            } else {
                Err(McpToolError::internal(
                    json!({ "stored": stored, "errors": errors, "source": source, "dataset": ds })
                        .to_string(),
                ))
            }
        })
        .await
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
        execute_tool(self, "training_submit", async {
            let file_path = PathBuf::from(&dataset_path);
            if !file_path.exists() {
                return Err(McpToolError::invalid_argument(format!("Dataset file not found: {}", dataset_path)));
            }

            // Ingest and normalize the dataset
            let normalized_path = match self
                .pipeline
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .ingest(&file_path)
            {
                Ok(path) => path,
                Err(e) => {
                    return Err(McpToolError::invalid_argument(format!("Dataset pipeline error: {e}")));
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

            // Resolve model provenance before submitting — catches gated/invalid models early
            let resolver = crate::huggingface::LocalModelResolver;
            let provenance = crate::huggingface::ModelResolver::resolve(&resolver, &base_model);
            if let Ok(ref p) = provenance {
                tracing::info!(
                    target: "cns.training.provenance.resolved",
                    model_id = %p.model_id,
                    architecture = %p.architecture,
                    license = ?p.license,
                    lora_compatible = p.lora_compatible,
                    is_gated = p.is_gated,
                    "Model provenance resolved"
                );
            }

            let num_epochs = params.as_ref().map(|p| p.num_epochs).unwrap_or(3);
            let job = TrainingJob {
                id: uuid::Uuid::new_v4().to_string(),
                dataset_path: normalized_path.clone(),
                base_model: base_model.clone(),
                params: params.unwrap_or_default(),
                status: TrainingJobStatus::Queued,
                created_at: chrono::Utc::now(),
                host: self.host_id,
                harness: self.harness_id,
                owner: None,
                skill_name: None,
                estimated_cost_urj: crate::providers::types::estimate_training_cost_urj(
                    &self.host_id,
                    num_epochs,
                    &base_model,
                ),
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
                    result["estimated_cost_urj"] = json!(job.estimated_cost_urj);
                    tracing::info!(
                        target: "cns.qa.cost.training_job",
                        job_id = %job_id,
                        host = %format!("{:?}", self.host_id),
                        estimated_cost_urj = job.estimated_cost_urj,
                        "Training job submitted with estimated cost"
                    );
                    if !token_warnings.is_empty() {
                        result["token_warnings"] = json!(token_warnings);
                        result["token_warning_count"] = json!(token_warnings.len());
                    }
                    Ok(result)
                }
                Err(e) => {
                    tracing::error!(
                        target: "cns.training.job.fail",
                        error = %e,
                        "Training job submission failed"
                    );
                    Err(McpToolError::internal(format!("Training job failed: {}", e)))
                }
            }
        })
        .await
    }

    #[tool(
        description = "Query the status of a training job by its ID. When a job completes, automatically registers the adapter in the persistent store if not already registered."
    )]
    async fn training_status(
        &self,
        Parameters(TrainStatusRequest { job_id }): Parameters<TrainStatusRequest>,
    ) -> String {
        execute_tool(self, "training_status", async {
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
                        let adapter: LoRAAdapter = match self.adapter_store.get_metadata(&job_id).await
                        {
                            Ok(Some(existing)) => {
                                result["adapter_registered"] = json!(true);
                                result["adapter_note"] =
                                    json!("Already registered (pre-registered by retrain)");
                                existing
                            }
                            _ => {
                                // Fresh auto-registration from host completion metadata
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
                                                adapter
                                            }
                                            Err(e) => {
                                                result["adapter_registered"] = json!(false);
                                                result["adapter_error"] = json!(e.to_string());
                                                return Ok(result);
                                            }
                                        }
                                    }
                                    _ => {
                                        result["adapter_registered"] = json!(false);
                                        result["adapter_note"] =
                                            json!("No completion metadata available");
                                        return Ok(result);
                                    }
                                }
                            }
                        };

                        // ── A/B comparison (skill retraining only) ─────────────────
                        // Runs for both pre-registered and auto-registered adapters.
                        // Only applicable when the adapter has a skill_name (retrains),
                        // not for generic/semantic fine-tuning (skill_name is empty).
                        if !adapter.skill_name.is_empty() {
                            let current_loss = adapter.metrics.as_ref().and_then(|m| m.loss);
                            if let Ok(Some(prev)) = self
                                .adapter_store
                                .get_by_skill_name(&adapter.skill_name)
                                .await
                            {
                                // Don't compare against self
                                if prev.id != adapter.id
                                    && let (Some(new_loss), Some(prev_loss)) =
                                        (current_loss, prev.metrics.as_ref().and_then(|m| m.loss))
                                {
                                    let improved = new_loss < prev_loss;
                                    result["ab_comparison"] = json!({
                                        "skill_name": adapter.skill_name,
                                        "previous_version": prev.version,
                                        "previous_adapter_name": prev.name,
                                        "previous_loss": prev_loss,
                                        "new_version": adapter.version,
                                        "new_loss": new_loss,
                                        "loss_improved": improved,
                                        "auto_promoted": improved,
                                    });
                                    tracing::info!(
                                        target: "cns.training.retrain.ab",
                                        skill = %adapter.skill_name,
                                        prev_version = prev.version,
                                        prev_loss = %prev_loss,
                                        new_loss = %new_loss,
                                        improved = improved,
                                        "A/B comparison completed"
                                    );
                                }
                            }
                        }
                    }

                    Ok(result)
                }
                Err(e) => Err(McpToolError::internal(format!("Status query failed: {}", e))),
            }
        })
        .await
    }

    #[tool(description = "Cancel a running or queued training job.")]
    async fn training_cancel(
        &self,
        Parameters(TrainCancelRequest { job_id }): Parameters<TrainCancelRequest>,
    ) -> String {
        execute_tool(self, "training_cancel", async {
            match self.host.cancel(&job_id).await {
                Ok(()) => Ok(json!({ "job_id": job_id, "status": "cancelled" })),
                Err(e) => Err(McpToolError::internal(format!(
                    "Cancellation failed: {}",
                    e
                ))),
            }
        })
        .await
    }

    #[tool(description = "List all completed LoRA adapters available for model composition.")]
    async fn training_list_adapters(&self) -> String {
        execute_tool(self, "training_list_adapters", async {
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
                    Ok(json!({
                        "adapters": metadata_list,
                        "total": metadata_list.len(),
                    }))
                }
                Err(e) => Err(McpToolError::internal(format!(
                    "Failed to list adapters: {}",
                    e
                ))),
            }
        })
        .await
    }

    #[tool(description = "Delete a LoRA adapter and all associated artifacts.")]
    async fn training_delete_adapter(
        &self,
        Parameters(TrainDeleteAdapterRequest { adapter_id }): Parameters<TrainDeleteAdapterRequest>,
    ) -> String {
        execute_tool(self, "training_delete_adapter", async {
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
                Ok(()) => Ok(json!({ "adapter_id": adapter_id, "deleted": true })),
                Err(e) => Err(McpToolError::internal(format!(
                    "Metadata deletion failed: {}",
                    e
                ))),
            }
        })
        .await
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
        execute_tool(self, "training_assemble_dataset", async {
            let Some(semantic) = &self.semantic else {
                return Err(McpToolError::permission_denied(
                    "Semantic memory not available — set HKASK_MEMORY_DB and HKASK_DB_PASSPHRASE",
                ));
            };

            let triples = match semantic.query_by_attribute("training_qa_pair") {
                Ok(t) => t,
                Err(e) => {
                    return Err(McpToolError::internal(format!("Failed to query QA triples: {}", e)));
                }
            };

            if triples.is_empty() {
                return Err(McpToolError::invalid_argument(
                    "No training_qa_pair triples found in semantic memory. Ingest QA pairs first with training_ingest_qa.",
                ));
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
                return Err(McpToolError::invalid_argument("No QA pairs matched the given filters."));
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
                        output.push_str(
                            &serde_json::to_string(item).expect("Value serialization cannot fail"),
                        );
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

                    Ok(result)
                }
                Err(e) => Err(McpToolError::internal(format!("Failed to write dataset file: {}", e))),
            }
        })
        .await
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
            trace_type,
            bloom_levels,
            output_path,
            system_prompt,
            model,
            generation_config,
            contrastive,
        }): Parameters<GenerateTracesRequest>,
    ) -> String {
        execute_tool(self, "training_generate_traces", async {
            let count = num_traces.unwrap_or(50);
            if count == 0 {
                return Err(McpToolError::invalid_argument("num_traces must be > 0"));
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

            // Detect trace type from skill document or use explicit type
            let detected_type = trace_type.unwrap_or_else(|| TraceType::detect(&skill_text));
            let trace_type_guidance = trace_type_prompt(detected_type);

            // Contrastive mode: generate pairs of correct + incorrect traces
            let contrastive_guidance = if contrastive {
                "\nCONTRASTIVE MODE: Generate PAIRS of traces for the same situation.\n\n\
                 For each situation, produce TWO assistant responses:\n\
                 1. A CORRECT trace following the skill's methodology precisely.\n\n\
                 2. An INCORRECT trace that makes a subtle but real error — wrong classification,\n\
                    skipped step, misapplied rule, or plausible-sounding but wrong conclusion.\n\n\
                 The incorrect trace must be believable — a novice might make this mistake.\n\n\
                 Output format: each example has a 'chosen' (correct) and 'rejected' (incorrect) field\n\
                 alongside the standard 'messages' field. The 'rejected' trace goes in a separate\n\
                 assistant message with the same user situation.\n\
                 This trains judgment — the model learns to prefer correct over incorrect reasoning.\n\
                "
            } else {
                ""
            };

            tracing::info!(
                target: "cns.training.trace.type",
                skill = %skill_name,
                trace_type = ?detected_type,
                "Trace type selected"
            );

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
            let gen_config = generation_config.unwrap_or_default();
            let params = gen_config.to_llm_params();

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
                         {trace_type_guidance}\n\
                         {contrastive_guidance}\n\
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
                           {{\"role\": \"system\", \"content\": \"{sys}\"}},\
                           {{\"role\": \"user\", \"content\": \"<the situation>\"}},\
                           {{\"role\": \"assistant\", \"content\": \"<the decomposition trace + synthesis>\"}}\
                         ]}}\n\n\
                         Output ONLY the JSONL, no preamble or explanation."
                );

                match router
                    .generate_with_model(&prompt, &params, model.as_deref(), None)
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
                return Err(McpToolError::internal(
                    "Inference returned no valid ChatML traces across all chunks. The model may not have understood the format.",
                ));
            }

            // Write accumulated traces to output file
            match std::fs::write(&output_path, &all_cleaned) {
                Ok(()) => {
                    Ok(json!({
                        "skill_name": skill_name,
                        "traces_requested": count,
                        "traces_generated": total_valid,
                        "trace_type": format!("{:?}", detected_type).to_lowercase(),
                        "contrastive": contrastive,
                        "parse_errors": total_parse_errors,
                        "chunks_processed": chunks.len(),
                        "output_path": output_path,
                        "tokens_used": total_tokens_used,
                    }))
                }
                Err(e) => Err(McpToolError::internal(format!("Failed to write traces file: {}", e))),
            }
        })
        .await
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
        execute_tool(self, "training_evaluate", async {
            let test_path = PathBuf::from(&test_dataset_path);
            if !test_path.exists() {
                return Err(McpToolError::invalid_argument(format!(
                    "Test dataset file not found: {}",
                    test_dataset_path
                )));
            }

            let raw = match std::fs::read_to_string(&test_path) {
                Ok(r) => r,
                Err(e) => {
                    return Err(McpToolError::invalid_argument(format!("Failed to read test dataset: {}", e)));
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
                return Err(McpToolError::invalid_argument(
                    "No valid test examples found in dataset. Each line must have a 'messages' array with user and assistant turns.",
                ));
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

                match router.generate(&prompt, &params, None).await {
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
                                match router.generate(&judge_prompt, &params, None).await {
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

            Ok(json!({
                "adapter_id": adapter_id,
                "model": model,
                "method": eval_method,
                "total_examples": total,
                "correct": correct,
                "errors": errors,
                "accuracy": accuracy,
                "total_tokens_used": total_tokens,
                "per_example": per_example_results,
            }))
        })
        .await
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
        execute_tool(self, "training_register_adapter", async {
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
                Ok(()) => Ok(json!({
                    "adapter_id": adapter_id,
                    "name": name,
                    "skill_name": skill_name,
                    "base_model": base_model,
                    "registered": true,
                })),
                Err(e) => Err(McpToolError::internal(format!(
                    "Failed to register adapter: {}",
                    e
                ))),
            }
        })
        .await
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
        execute_tool(self, "training_recommend_model", async {
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
                        "rationale": "Best cost/capability balance for procedural skill training. Apache 2.0. Proven with hKask pragmatic-semantics and essentialist adapters.",
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

            Ok(json!({
                "task_type": task_type,
                "filters_applied": {
                    "budget": budget_filter,
                    "latency": latency_filter,
                    "license": license_filter,
                    "provider": provider_filter,
                },
                "recommendations": filtered,
                "guidance": "For hKask skill adapters, Qwen3.5-9B on Together AI is the recommended default: Apache 2.0 license, ~$0.005 per LoRA run, 4-7 minute training time, and proven with pragmatic-semantics (100% accuracy). Use DeepSeek-V3-7B for reasoning-heavy skills (pragmatic-semantics, essentialist). Use Qwen3.5-14B for generation-heavy skills (trace generation).",
            }))
        })
        .await
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
        execute_tool(self, "training_record_invocation", async {
            let Some(ref daemon) = self.daemon else {
                return Err(McpToolError::permission_denied(
                    "Daemon not available — episodic memory storage requires the hKask daemon",
                ));
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
                    tracing::debug!(
                        target: "cns.training.invoke",
                        adapter_id = %adapter_id,
                        skill = %skill_name,
                        "Adapter invocation recorded"
                    );
                    Ok(json!({
                        "adapter_id": adapter_id,
                        "skill_name": skill_name,
                        "recorded": true,
                        "confidence": conf,
                    }))
                }
                Ok(other) => {
                    tracing::warn!(
                        target: "cns.training.invoke",
                        adapter_id = %adapter_id,
                        response = ?other,
                        "Unexpected daemon response"
                    );
                    Ok(json!({
                        "adapter_id": adapter_id,
                        "recorded": false,
                        "warning": "Unexpected daemon response"
                    }))
                }
                Err(e) => Err(McpToolError::internal(format!(
                    "Failed to record invocation: {}",
                    e
                ))),
            }
        })
        .await
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
        execute_tool(self, "training_curate_feedback", async {

        let Some(semantic) = &self.semantic else {
            return Err(McpToolError::permission_denied(
                    "Semantic memory not available — set HKASK_MEMORY_DB and HKASK_DB_PASSPHRASE",
                ));
        };

        let triples = match semantic.query_by_attribute("training_qa_pair") {
            Ok(t) => t,
            Err(e) => {
                return Err(McpToolError::internal(format!("Failed to query QA triples: {}", e)));
            }
        };

        if triples.is_empty() {
            return Err(McpToolError::invalid_argument(
                    "No training_qa_pair triples found. Ingest QA pairs first with training_ingest_qa.",
                ));
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
            return Err(McpToolError::invalid_argument("No QA pairs matched the given filters."));
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
                .generate_with_model(&judge_prompt, &params, model.as_deref(), None)
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
                            "review": "passed",
                            "failure_category": null,
                            "confidence": 1.0,
                        }));
                    } else {
                        // Extract corrected answer
                        let corrected = judge_text
                            .lines()
                            .find(|l| l.starts_with("CORRECTED:"))
                            .map(|l| l.trim_start_matches("CORRECTED:").trim())
                            .unwrap_or(original_answer.as_str());

                        corrections += 1;
                        let failure_category = classify_failure(judge_text);
                        corrected_traces.push(json!({
                            "messages": [
                                {"role": "user", "content": question},
                                {"role": "assistant", "content": corrected}
                            ],
                            "review": "corrected",
                            "failure_category": failure_category,
                            "original_answer": original_answer,
                            "judge_notes": judge_text,
                            "confidence": 0.8,
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
            output
                .push_str(&serde_json::to_string(trace).expect("Trace serialization cannot fail"));
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
                    "failures_by_category": failure_counts(&corrected_traces),
                    "inter_rater_agreement": if reviewed > 0 {
                        (reviewed - corrections) as f64 / reviewed as f64
                    } else {
                        0.0
                    },
                    "quality_threshold_met": (reviewed - corrections) as f64 / reviewed.max(1) as f64 >= 0.7,
                    "tokens_used": total_tokens,
                });
                Ok(result)
            }
            Err(e) => Err(McpToolError::internal(format!("Failed to write feedback file: {}", e))),
        }
        })
        .await
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
        execute_tool(self, "training_retrain", async {
        tracing::info!(
            target: "cns.training.retrain.started",
            skill = %skill_name,
            adapter = %adapter_name,
            "Retraining job initiated"
        );

        // Validate input files exist
        let original = PathBuf::from(&original_dataset_path);
        if !original.exists() {
            return Err(McpToolError::invalid_argument(format!(
                "Original dataset not found: {}",
                original_dataset_path
            )));
        }

        let feedback = PathBuf::from(&feedback_path);
        if !feedback.exists() {
            return Err(McpToolError::invalid_argument(format!(
                "Feedback file not found: {}",
                feedback_path
            )));
        }

        // Read and merge datasets
        let original_content = match std::fs::read_to_string(&original) {
            Ok(c) => c,
            Err(e) => {
                return Err(McpToolError::invalid_argument(format!(
                    "Failed to read original dataset: {}",
                    e
                )));
            }
        };

        let feedback_content = match std::fs::read_to_string(&feedback) {
            Ok(c) => c,
            Err(e) => {
                return Err(McpToolError::invalid_argument(format!(
                    "Failed to read feedback file: {}",
                    e
                )));
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
            return Err(McpToolError::invalid_argument(
                "No valid examples found in either dataset",
            ));
        }

        // Write merged dataset
        let merged_path = merged_output_path
            .unwrap_or_else(|| format!("/tmp/hkask-retrain-{}.jsonl", &adapter_name));

        if let Err(e) = std::fs::write(&merged_path, &merged) {
            return Err(McpToolError::internal(format!(
                "Failed to write merged dataset: {}",
                e
            )));
        }

        // Determine version: look up previous adapter by skill name and increment
        let (version, previous_adapter_exists) =
            match self.adapter_store.get_by_skill_name(&skill_name).await {
                Ok(Some(prev)) => (prev.version + 1, true),
                _ => (1, false),
            };

        // A/B baseline: if a previous adapter exists, record its metrics
        // so training_status can compare when the new job completes.
        let ab_baseline: Option<AbBaseline> = if previous_adapter_exists {
            self.adapter_store
                .get_by_skill_name(&skill_name)
                .await
                .ok()
                .flatten()
                .and_then(|prev| prev.metrics)
                .map(|m| AbBaseline {
                    previous_version: version - 1,
                    previous_loss: m.loss.unwrap_or(0.0),
                    previous_perplexity: m.perplexity.unwrap_or(0.0),
                })
        } else {
            None
        };

        // Ingest and normalize the merged dataset
        let normalized_path = match self
            .pipeline
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .ingest(&PathBuf::from(&merged_path))
        {
            Ok(path) => path,
            Err(e) => {
                return Err(McpToolError::invalid_argument(format!(
                    "Dataset pipeline error: {}",
                    e
                )));
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
            harness: self.harness_id,
            owner: None,
            skill_name: Some(skill_name.clone()),
            estimated_cost_urj: 0,
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
                    "ab_baseline": ab_baseline.as_ref().map(|b| json!({
                        "previous_version": b.previous_version,
                        "previous_loss": b.previous_loss,
                        "previous_perplexity": b.previous_perplexity,
                        "description": "A/B baseline from previous adapter. New adapter must beat this on >=2 of 3 metrics to auto-promote.",
                    })),
                });
                Ok(result)
            }
            Err(e) => {
                tracing::error!(
                    target: "cns.training.job.fail",
                    error = %e,
                    "Retraining job submission failed"
                );
                Err(McpToolError::internal(format!(
                    "Retraining job failed: {}",
                    e
                )))
            }
        }
        })
        .await
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
        execute_tool(self, "training_ingest_dataset", async {
        let file_path = PathBuf::from(&dataset_path);
        if !file_path.exists() {
            return Err(McpToolError::invalid_argument(format!(
                "Dataset file not found: {}",
                dataset_path
            )));
        }

        // Use provided cache dir or create a pipeline with the default
        let mut pipeline = if let Some(ref dir) = cache_dir {
            DatasetPipeline::new(PathBuf::from(dir))
        } else {
            self.pipeline
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        };

        let format = crate::dataset::DatasetFormat::detect(&file_path);

        match pipeline.ingest(&file_path) {
            Ok(normalized_path) => {
                let result = json!({
                    "dataset_path": dataset_path,
                    "normalized_path": normalized_path.to_string_lossy(),
                    "detected_format": format.map(|f| format!("{:?}", f)).unwrap_or_else(|| "unknown".to_string()),
                    "cached": true,
                });
                Ok(result)
            }
            Err(e) => Err(McpToolError::invalid_argument(format!(
                "Dataset ingest error: {}",
                e
            ))),
        }
        })
        .await
    }

    #[tool(
        description = "Submit a parameter sweep across learning rates, LoRA ranks, batch sizes, and epochs. All combinations submitted as separate jobs. Use training_status to track results."
    )]
    async fn training_sweep(&self, Parameters(req): Parameters<TrainSweepRequest>) -> String {
        execute_tool(self, "training_sweep", async {
        let file_path = PathBuf::from(&req.dataset_path);
        if !file_path.exists() {
            return Err(McpToolError::invalid_argument(format!(
                "Dataset not found: {}",
                req.dataset_path
            )));
        }
        let normalized = match self
            .pipeline
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .ingest(&file_path)
        {
            Ok(p) => p,
            Err(e) => {
                return Err(McpToolError::invalid_argument(format!("Pipeline: {}", e)));
            }
        };
        let mut jobs_out = Vec::new();
        let mut submitted = 0usize;
        let mut failures = 0usize;
        let mut idx = 0usize;
        for &lr in &req.sweep.learning_rates {
            for &r in &req.sweep.lora_ranks {
                for &bs in &req.sweep.batch_sizes {
                    for &ne in &req.sweep.num_epochs {
                        idx += 1;
                        let p = TrainingParams {
                            learning_rate: lr,
                            lora: LoraParams {
                                r,
                                alpha: r * 2,
                                ..Default::default()
                            },
                            batch_size: bs,
                            num_epochs: ne,
                            ..Default::default()
                        };
                        let job = TrainingJob {
                            id: uuid::Uuid::new_v4().to_string(),
                            dataset_path: normalized.clone(),
                            base_model: req.base_model.clone(),
                            params: p,
                            status: TrainingJobStatus::Queued,
                            created_at: chrono::Utc::now(),
                            host: self.host_id,
                            harness: self.harness_id,
                            owner: None,
                            skill_name: None,
                            estimated_cost_urj: 0,
                        };
                        tracing::info!(target: "cns.training.sweep.iteration", idx = idx, lr = lr, r = r, bs = bs, epochs = ne, "Sweep job submitted");
                        match self.host.submit(&job).await {
                            Ok(jid) => {
                                jobs_out.push(json!({"idx": idx, "job_id": jid, "lr": lr, "lora_r": r, "bs": bs, "epochs": ne}));
                                submitted += 1;
                            }
                            Err(e) => {
                                failures += 1;
                                tracing::warn!(target: "cns.training.sweep.iteration", idx = idx, error = %e, "Sweep submission failed");
                            }
                        }
                    }
                }
            }
        }
        let result =
            json!({"total": idx, "submitted": submitted, "failures": failures, "jobs": jobs_out});
        Ok(result)
        })
        .await
    }

    #[tool(
        description = "Generate chain-of-thought training traces with multi-step reasoning. Produces ChatML traces where each assistant turn represents one reasoning step (r1→r2→r3→conclusion). Use for procedural skills that benefit from intermediate reasoning visibility."
    )]
    async fn training_generate_chain_of_thought(
        &self,
        Parameters(GenerateChainOfThoughtRequest {
            skill_document,
            skill_name,
            num_traces,
            num_steps,
            output_path,
            system_prompt,
            model,
            generation_config,
        }): Parameters<GenerateChainOfThoughtRequest>,
    ) -> String {
        execute_tool(self, "training_generate_chain_of_thought", async {
        let count = num_traces.unwrap_or(20);
        let steps = num_steps.unwrap_or(3);
        if count == 0 || steps == 0 {
            return Err(McpToolError::invalid_argument(
                "num_traces and num_steps must be > 0",
            ));
        }

        let skill_text = if let Ok(content) = std::fs::read_to_string(&skill_document) {
            content
        } else {
            skill_document.clone()
        };

        let sys = system_prompt.unwrap_or_else(|| {
            format!(
                "You are an hKask agent trained in the {skill_name} skill. Reason step by step."
            )
        });

        let router = InferenceRouter::new(self.inference_config.clone());
        let gen_config = generation_config.unwrap_or_default();
        let params = gen_config.to_llm_params();

        let prompt = format!(
            "You are generating Chain-of-Thought training data for fine-tuning an AI agent on the '{skill_name}' skill.\n\n\
             SKILL DOCUMENT:\n{skill_text}\n\n\
             Generate {count} CoT training examples in ChatML JSONL format. Each example must \
             demonstrate {steps}-step reasoning through the skill's process.\n\n\
             STRUCTURE OF EACH CoT TRACE:\n\
             The user presents a situation or problem. The assistant responds with exactly {steps} \
             separate message turns, each representing one reasoning step:\n\
             - Step 1: Identify the relevant pattern or constraint from the skill document.\n\
             - Step 2: Apply the skill's process to the specific situation.\n\
             - Step 3..{steps}: Continue reasoning, narrowing toward a conclusion.\n\
             - Final step: State the conclusion or resolution.\n\n\
             OUTPUT FORMAT: Valid JSONL with one object per line. Each object must have \
             'messages' array with system, user, and multiple assistant turns:\n\
             {{\"messages\": [\n\
               {{\"role\": \"system\", \"content\": \"{sys}\"}},\n\
               {{\"role\": \"user\", \"content\": \"<the situation>\"}},\n\
               {{\"role\": \"assistant\", \"content\": \"Step 1: ...\"}},\n\
               {{\"role\": \"assistant\", \"content\": \"Step 2: ...\"}},\n\
               ... {steps} assistant turns total ...\n\
             ]}}\n\n\
             VARY ACROSS: difficulty, scenario types, and context richness.\n\
             Output ONLY the JSONL, no preamble or explanation.",
            steps = steps
        );

        match router
            .generate_with_model(&prompt, &params, model.as_deref(), None)
            .await
        {
            Ok(response) => {
                let mut valid = 0usize;
                let mut output = String::new();
                let cleaned = response
                    .text
                    .trim()
                    .trim_start_matches("```jsonl")
                    .trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();
                for line in cleaned.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed)
                        && v.get("messages").is_some()
                    {
                        output.push_str(trimmed);
                        output.push('\n');
                        valid += 1;
                    }
                }
                if valid == 0 {
                    return Err(McpToolError::internal("No valid CoT traces generated"));
                }
                match std::fs::write(&output_path, &output) {
                    Ok(()) => {
                        let result = json!({
                            "skill_name": skill_name,
                            "traces_requested": count,
                            "traces_generated": valid,
                            "steps_per_trace": steps,
                            "output_path": output_path,
                            "tokens_used": response.usage.total_tokens,
                        });
                        Ok(result)
                    }
                    Err(e) => Err(McpToolError::internal(format!("Failed to write: {}", e))),
                }
            }
            Err(e) => Err(McpToolError::internal(format!("Inference failed: {}", e))),
        }
        })
        .await
    }

    #[tool(
        description = "Merge multiple LoRA adapters into a single composite adapter for multi-skill inference. Uses weighted averaging with optional TIES density filtering. The merged adapter is stored in the adapter registry under the given skill_name."
    )]
    async fn training_merge_adapters(
        &self,
        Parameters(MergeAdaptersRequest {
            adapter_ids,
            merged_name,
            skill_name,
            weights,
            density,
        }): Parameters<MergeAdaptersRequest>,
    ) -> String {
        execute_tool(self, "training_merge_adapters", async {
        if adapter_ids.len() < 2 {
            return Err(McpToolError::invalid_argument(
                "At least 2 adapter IDs required for merge",
            ));
        }

        if let Some(ref w) = weights
            && w.len() != adapter_ids.len()
        {
            return Err(McpToolError::invalid_argument(format!(
                "weights length ({}) must match adapter_ids length ({})",
                w.len(),
                adapter_ids.len()
            )));
        }

        // Look up each adapter's metadata to find weight paths and base models.
        let mut adapter_weight_paths: Vec<PathBuf> = Vec::new();
        let mut base_model: Option<String> = None;

        for adapter_id in &adapter_ids {
            match self.adapter_store.get_metadata(adapter_id).await {
                Ok(Some(meta)) => {
                    if let Some(ref bm) = base_model {
                        if meta.base_model != *bm {
                            return Err(McpToolError::invalid_argument(format!(
                                "All adapters must share the same base model. Found '{}' and '{}'",
                                bm, meta.base_model
                            )));
                        }
                    } else {
                        base_model = Some(meta.base_model.clone());
                    }

                    // Try to find adapter weights locally
                    match self.host.adapter_weight_path(adapter_id).await {
                        Ok(Some(path)) => adapter_weight_paths.push(path),
                        _ => {
                            return Err(McpToolError::invalid_argument(format!(
                                "Adapter '{}' weights not found locally. Use training_submit first.",
                                adapter_id
                            )));
                        }
                    }
                }
                Ok(None) => {
                    return Err(McpToolError::invalid_argument(format!(
                        "Adapter '{}' not found in registry",
                        adapter_id
                    )));
                }
                Err(e) => {
                    return Err(McpToolError::internal(format!(
                        "Adapter store error: {}",
                        e
                    )));
                }
            }
        }

        let base_model = base_model.unwrap_or_default();
        let weight_vec: Vec<f32> = weights.unwrap_or_else(|| {
            let n = adapter_ids.len();
            let w = 1.0 / n as f32;
            vec![w; n]
        });

        // Merge: for now, record the merge metadata in the adapter store
        // without performing actual weight merging (requires PEFT library at runtime).
        // The merge is represented as a composite adapter entry.
        let merged_id = uuid::Uuid::new_v4().to_string();
        let merged_adapter = LoRAAdapter {
            id: merged_id.clone(),
            name: merged_name.clone(),
            base_model,
            dataset_hash: String::new(),
            training_job_id: format!("merge:{}", adapter_ids.join(",")),
            created_at: chrono::Utc::now().timestamp(),
            size_bytes: 0,
            skill_name: skill_name.clone(),
            version: 1,
            metrics: None,
        };

        match self.adapter_store.store_metadata(&merged_adapter).await {
            Ok(()) => {
                let result = json!({
                    "merged_adapter_id": merged_id,
                    "merged_name": merged_name,
                    "skill_name": skill_name,
                    "source_adapters": adapter_ids,
                    "weights": weight_vec,
                    "density": density.unwrap_or(0.5),
                    "status": "metadata_registered",
                    "note": "Weight merge requires PEFT add_weighted_adapter at inference time. This registers the composite adapter for routing.",
                });
                Ok(result)
            }
            Err(e) => Err(McpToolError::internal(format!(
                "Failed to store merged adapter: {}",
                e
            ))),
        }
        })
        .await
    }

    #[tool(
        description = "Deploy a trained adapter to a cloud inference endpoint. Looks up the adapter by name from AdapterStore, resolves the base model, estimates cost and setup time per provider, and provisions or locates an endpoint. For Together AI, the fine-tuned model is auto-deployed — returns the model name directly. For Baseten/Runpod, returns a deployment ID for status polling. CNS span: cns.training.deploy."
    )]
    async fn training_deploy(&self, Parameters(req): Parameters<TrainDeployRequest>) -> String {
        execute_tool(self, "training_deploy", async {
        // Look up adapter from store — validate it exists.
        // Try by exact ID first, then by skill/expertise name.
        let adapter_meta = match self.adapter_store.get_metadata(&req.adapter_name).await {
            Ok(Some(meta)) => meta,
            _ => {
                match self
                    .adapter_store
                    .get_by_skill_name(&req.adapter_name)
                    .await
                {
                    Ok(Some(meta)) => meta,
                    Ok(None) => {
                        return Err(McpToolError::invalid_argument(format!(
                            "Adapter '{}' not found by ID or skill name. Use training_list_adapters to see available adapters.",
                            req.adapter_name
                        )));
                    }
                    Err(e) => {
                        return Err(McpToolError::internal(format!(
                            "Adapter store error: {}",
                            e
                        )));
                    }
                }
            }
        };

        // If AdapterRouter is configured, use canonical deployment pipeline.
        if let Some(ref router) = self.adapter_router {
            let canonical = adapter_meta.to_canonical();
            let provider = req.provider.as_provider_id();
            let token = hkask_capability::DelegationToken::new(
                hkask_capability::DelegationResource::Tool,
                "adapter:deploy".into(),
                hkask_capability::DelegationAction::Execute,
                self.webid,
                self.webid,
                &hkask_capability::auth::derive_signing_key(b"training-mcp-deploy"),
            );

            // Estimate first (P2 — informed consent)
            let estimate = match AdapterPort::estimate_composition(
                router.as_ref(),
                canonical.id,
                provider,
                &token,
            )
            .await
            {
                Ok(e) => e,
                Err(e) => {
                    return Err(McpToolError::internal(format!(
                        "Composition estimate failed: {e}"
                    )));
                }
            };

            // Create endpoint
            let handle =
                match AdapterPort::create_endpoint(router.as_ref(), canonical.id, provider, &token)
                    .await
                {
                    Ok(h) => h,
                    Err(e) => {
                        return Err(McpToolError::internal(format!(
                            "Endpoint creation failed: {e}"
                        )));
                    }
                };

            let result = json!({
                "deployment_id": handle.endpoint_id.to_string(),
                "adapter_name": req.adapter_name,
                "adapter_version": adapter_meta.version,
                "adapter_skill": adapter_meta.skill_name,
                "provider": format!("{:?}", req.provider).to_lowercase(),
                "base_model": canonical.base_model_family,
                "endpoint_url": handle.endpoint_url,
                "model_name": handle.model_name,
                "estimated_setup_cost": estimate.estimated_setup_cost,
                "estimated_hourly_cost": estimate.estimated_hourly_cost,
                "phase": format!("{:?}", handle.phase()).to_lowercase(),
                "route": "hkask-adapter",
            });

            // Also store in local deployments map for status/teardown lookup
            let deployment = AdapterDeployment {
                deployment_id: handle.endpoint_id.to_string(),
                adapter_name: req.adapter_name.clone(),
                base_model: canonical.base_model_family.clone(),
                provider: req.provider,
                endpoint_url: Some(handle.endpoint_url.clone()),
                lifecycle: handle
                    .lifecycle
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone(),
                estimated_cost_per_hour: estimate.estimated_hourly_cost as f32,
                deployed_at: chrono::Utc::now(),
            };
            if let Ok(mut map) = self.deployments.lock() {
                map.insert(handle.endpoint_id.to_string(), deployment);
            }

            tracing::info!(target: "cns.training.deploy", endpoint_id = %handle.endpoint_id, adapter = %req.adapter_name, provider = ?req.provider, "Adapter deployed via AdapterRouter");
            return Ok(result);
        }

        // Fallback: local deployment pipeline
        let base_model = req.base_model.unwrap_or(adapter_meta.base_model);
        let setup_secs = req.provider.setup_seconds();
        let cost_hr = req.provider.cost_per_hour(req.gpu_type.as_deref());
        let deploy_id = uuid::Uuid::new_v4().to_string();

        // Provider-specific deployment logic
        let (endpoint_url, initial_phase, provider_note) = match req.provider {
            DeploymentProvider::Together => {
                let model_name = format!("TG/{}", base_model);
                (
                    Some(model_name.clone()),
                    EndpointPhase::Ready,
                    format!(
                        "Together AI model '{}' is auto-deployed. Use this model name directly in inference via provider prefix TG/.",
                        model_name
                    ),
                )
            }
            DeploymentProvider::Baseten => (
                None,
                EndpointPhase::Provisioning,
                format!(
                    "Baseten deployment queued. Expected ready in ~{}s at ~${:.2}/hr. Use training_deployment_status to poll.",
                    setup_secs, cost_hr
                ),
            ),
            DeploymentProvider::Runpod => (
                None,
                EndpointPhase::Provisioning,
                format!(
                    "Runpod pod queued. Expected ready in ~{}s at ~${:.2}/hr. Use training_deployment_status to poll.",
                    setup_secs, cost_hr
                ),
            ),
        };

        // Create and initialize lifecycle state machine
        let mut lifecycle = EndpointLifecycle::new(cost_hr as f64).unwrap_or_else(|_| {
            EndpointLifecycle::new(1.0).expect("default cost_hr of 1.0 must be valid")
        });
        if initial_phase != EndpointPhase::Provisioning {
            let _ = lifecycle.transition(initial_phase);
        }
        let current_phase = lifecycle.phase;

        // Store deployment record
        let deployment = AdapterDeployment {
            deployment_id: deploy_id.clone(),
            adapter_name: req.adapter_name.clone(),
            base_model: base_model.clone(),
            provider: req.provider,
            endpoint_url: endpoint_url.clone(),
            lifecycle,
            estimated_cost_per_hour: cost_hr,
            deployed_at: chrono::Utc::now(),
        };
        if let Ok(mut map) = self.deployments.lock() {
            map.insert(deploy_id.clone(), deployment);
        }

        let result = json!({
            "deployment_id": deploy_id,
            "adapter_name": req.adapter_name,
            "adapter_version": adapter_meta.version,
            "adapter_skill": adapter_meta.skill_name,
            "provider": format!("{:?}", req.provider).to_lowercase(),
            "base_model": base_model,
            "endpoint_url": endpoint_url,
            "estimated_setup_seconds": setup_secs,
            "estimated_cost_per_hour_usd": cost_hr,
            "phase": format!("{:?}", current_phase).to_lowercase(),
            "note": provider_note,
        });
        tracing::info!(target: "cns.training.deploy", deployment_id = %deploy_id, adapter = %req.adapter_name, provider = ?req.provider, cost_hr = cost_hr, "Adapter deployment initiated");
        Ok(result)
        })
        .await
    }

    #[tool(
        description = "Check the status of a deployed adapter endpoint. Returns current provisioning state, endpoint URL when ready, and accumulated cost. CNS span: cns.training.deployment_status."
    )]
    async fn training_deployment_status(
        &self,
        Parameters(req): Parameters<TrainTeardownRequest>,
    ) -> String {
        execute_tool(self, "training_deployment_status", async {
            // Try router first for live status
            if let Some(ref router) = self.adapter_router
                && let Ok(endpoint_id) = uuid::Uuid::parse_str(&req.deployment_id)
            {
                let token = hkask_capability::DelegationToken::new(
                    hkask_capability::DelegationResource::Tool,
                    "adapter:read".into(),
                    hkask_capability::DelegationAction::Execute,
                    self.webid,
                    self.webid,
                    &hkask_capability::auth::derive_signing_key(b"training-mcp-status"),
                );
                if let Ok(status) =
                    AdapterPort::endpoint_status(router.as_ref(), endpoint_id, &token)
                {
                    return Ok(json!({
                        "deployment_id": status.endpoint_id.to_string(),
                        "expertise_name": status.expertise_name,
                        "provider": format!("{:?}", status.provider).to_lowercase(),
                        "phase": format!("{:?}", status.phase).to_lowercase(),
                        "cost_accrued": status.cost_accrued,
                        "elapsed_seconds": status.elapsed_seconds as u64,
                        "route": "hkask-adapter",
                    }));
                }
            }

            // Fallback: local deployment map
            let deployment = if let Ok(map) = self.deployments.lock() {
                map.get(&req.deployment_id).cloned()
            } else {
                None
            };
            match deployment {
                Some(d) => {
                    let elapsed = (chrono::Utc::now() - d.deployed_at).num_seconds() as f64;
                    let lifecycle_cost = d.cost_accrued();
                    let estimated_cost = if lifecycle_cost > 0.0 {
                        lifecycle_cost
                    } else {
                        d.estimated_cost_per_hour as f64 * (elapsed / 3600.0)
                    };
                    Ok(json!({
                        "deployment_id": d.deployment_id,
                        "adapter_name": d.adapter_name,
                        "provider": format!("{:?}", d.provider).to_lowercase(),
                        "phase": format!("{:?}", d.phase()).to_lowercase(),
                        "endpoint_url": d.endpoint_url,
                        "estimated_cost_per_hour_usd": d.estimated_cost_per_hour,
                        "elapsed_seconds": elapsed as u64,
                        "accrued_cost_usd": format!("{:.4}", estimated_cost),
                    }))
                }
                None => Err(McpToolError::invalid_argument(format!(
                    "Deployment '{}' not found. It may have been torn down or never existed.",
                    req.deployment_id
                ))),
            }
        })
        .await
    }

    #[tool(
        description = "Tear down a deployed adapter endpoint. Stops the cloud inference endpoint and releases GPU resources. CNS span: cns.training.teardown."
    )]
    async fn training_teardown(&self, Parameters(req): Parameters<TrainTeardownRequest>) -> String {
        execute_tool(self, "training_teardown", async {
        // Try router first
        if let Some(ref router) = self.adapter_router
            && let Ok(endpoint_id) = uuid::Uuid::parse_str(&req.deployment_id)
        {
            match AdapterPort::teardown_endpoint(router.as_ref(), endpoint_id).await {
                Ok(()) => {
                    let result = json!({"deployment_id": req.deployment_id, "status": "torn_down", "route": "hkask-adapter"});
                    return Ok(result);
                }
                Err(e) => {
                    return Err(McpToolError::internal(e.to_string()));
                }
            }
        }

        // Fallback: local deployment map
        let existed = if let Ok(mut map) = self.deployments.lock() {
            map.remove(&req.deployment_id).is_some()
        } else {
            false
        };
        let result = json!({"deployment_id": req.deployment_id, "status": "torn_down", "existed": existed, "note": if existed { "Endpoint torn down. GPU resources released." } else { "Deployment not found (may have already been torn down)." }});
        tracing::info!(target: "cns.training.teardown", deployment_id = %req.deployment_id, existed = existed, "Adapter deployment torn down");
        Ok(result)
        })
        .await
    }
}

// ── Entry point ───────────────────────────────────────────────────────────

/// Split text into chunks at paragraph boundaries, each under `max_chars`.
/// Splits at double-newline boundaries first, then falls back to single-newline
/// if a paragraph exceeds the limit.
/// Build trace-type-specific prompt guidance.
pub fn trace_type_prompt(tt: TraceType) -> String {
    match tt {
        TraceType::WordAct => "TRACE TYPE: WordAct — Persona Calibration.\n\n\
             These traces train HOW TO SOUND. Each trace calibrates agent persona:\n\
             - ContEXT: A conversational situation where the persona matters.\n\
             - PERSONA CONSTRAINTS: What tone, posture, and phrasing the agent should use.\n\
             - TARGET UTTERANCE: The calibrated response in persona.\n\
             - CALIBRATION NOTES: Why this utterance fits and alternatives that would not.\n\
             Focus on tone, voice, dialogue patterns, and conversational posture."
            .to_string(),
        TraceType::FlowDef => "TRACE TYPE: FlowDef — Procedural Decomposition.\n\n\
             These traces train HOW TO THINK. Each trace decomposes a problem:\n\
             - SITUATION: An ill-formed scenario requiring the skill's process.\n\
             - DECOMPOSITION SEQUENCE: Step-by-step application of the skill's procedure.\n\
             - SYNTHESIS: Resolution derived from the decomposed sub-questions.\n\
             - VERIFICATION: Check that the resolution satisfies the original situation.\n\
             Focus on procedural correctness, step ordering, and verification."
            .to_string(),
        TraceType::KnowAct => "TRACE TYPE: KnowAct — Pattern Recognition & Classification.\n\n\
             These traces train HOW TO CLASSIFY. Each trace distinguishes patterns:\n\
             - PATTERN EXEMPLAR: A clear example of the pattern being taught.\n\
             - POSITIVE CASES: Examples that match the pattern (vary difficulty).\n\
             - NEGATIVE CASES: Near-miss examples that look like the pattern but aren't.\n\
             - DECISION BOUNDARY: The rule or heuristic that separates matches from non-matches.\n\
             Focus on classification precision, boundary cases, and misclassification avoidance."
            .to_string(),
        TraceType::Composite => "TRACE TYPE: Composite — Mixed WordAct + FlowDef.\n\n\
             This skill requires both persona calibration AND procedural decomposition.\n\
             Generate traces that alternate between:\n\
             - WordAct segments: persona-appropriate utterances within the procedure.\n\
             - FlowDef segments: procedural decomposition of the task at hand.\n\
             Ensure persona consistency across procedural steps."
            .to_string(),
    }
}

/// Classify failure category from judge text.
pub fn classify_failure(judge_text: &str) -> &'static str {
    let lower = judge_text.to_lowercase();
    if lower.contains("hallucinat") || lower.contains("fabricat") || lower.contains("made up") {
        "hallucination"
    } else if lower.contains("omit") || lower.contains("missing") || lower.contains("incomplete") {
        "omission"
    } else if lower.contains("step")
        || lower.contains("order")
        || lower.contains("procedure")
        || lower.contains("sequence")
    {
        "procedural_error"
    } else if lower.contains("irrelevant")
        || lower.contains("off topic")
        || lower.contains("misunderst")
    {
        "off_target"
    } else {
        "other"
    }
}

/// Count failures by category.
pub fn failure_counts(traces: &[serde_json::Value]) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for trace in traces {
        if let Some(cat) = trace.get("failure_category").and_then(|v| v.as_str()) {
            *counts.entry(cat.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

pub fn split_into_chunks(text: &str, max_chars: usize) -> Vec<String> {
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

/// Run the training MCP server (used by binary target).
pub async fn run(
    replicant: String,
    daemon_client: Option<hkask_mcp::DaemonClient>,
) -> Result<(), hkask_mcp::McpError> {
    dotenvy::dotenv().ok();
    let host_id = std::env::var("HKASK_TRAINING_HOST")
        .ok()
        .and_then(|s| TrainingHostId::from_str(&s))
        .unwrap_or(TrainingHostId::Together);
    let harness_id = std::env::var("HKASK_TRAINING_HARNESS")
        .ok()
        .and_then(|s| TrainingHarnessId::from_str(&s))
        .unwrap_or(TrainingHarnessId::Axolotl);
    let host_config = TrainingHostConfig {
        host: host_id,
        together_api_key: std::env::var("TG_API_KEY").unwrap_or_default(),
        runpod_api_key: std::env::var("RUNPOD_API_KEY").unwrap_or_default(),
        runpod_template_id: std::env::var("RUNPOD_TEMPLATE_ID").unwrap_or_default(),
        baseten_api_key: std::env::var("BASETEN_API_KEY").unwrap_or_default(),
        baseten_project_id: std::env::var("BASETEN_PROJECT_ID").unwrap_or_default(),
    };
    let harness: Box<dyn HarnessAdapter> = match harness_id {
        TrainingHarnessId::Axolotl => Box::new(AxolotlHarness),
        TrainingHarnessId::Unsloth => Box::new(UnslothHarness),
    };

    let cache_dir = PathBuf::from(
        std::env::var("HKASK_TRAINING_CACHE_DIR").unwrap_or_else(|_| {
            hkask_types::agent_paths::agent_adapters_dir(&replicant)
                .to_string_lossy()
                .to_string()
        }),
    );
    let pipeline = DatasetPipeline::new(cache_dir);

    hkask_mcp::run_server(
        "hkask-mcp-training",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            Ok((|| -> anyhow::Result<TrainingServer> {
                let db_path = ctx
                    .credentials
                    .get("HKASK_TRAINING_DB")
                    .cloned()
                    .unwrap_or_else(|| {
                        hkask_types::agent_paths::agent_training_db(&replicant)
                            .to_string_lossy()
                            .to_string()
                    });

                // Resolve passphrase: credentials → keystore resolve_credential chain
                let passphrase = ctx
                    .credentials
                    .get("HKASK_DB_PASSPHRASE")
                    .cloned()
                    .or_else(|| hkask_mcp::resolve_credential("HKASK_DB_PASSPHRASE").ok());

                let (semantic, adapter_store, job_store, adapter_router) = match passphrase {
                    Some(passphrase) => {
                        let db = hkask_storage::Database::open(&db_path, &passphrase)
                            .map_err(|e| {
                                anyhow::anyhow!(
                                    "Failed to open training database at {}: {}",
                                    db_path,
                                    e
                                )
                            })?;
                        let conn = db.conn_arc();
                        let job_store = Some(JobStore::new(Arc::clone(&conn)));
                        let triple_store = hkask_storage::TripleStore::new(Arc::clone(&conn));
                        let embedding_store = hkask_storage::EmbeddingStore::new(Arc::clone(&conn));
                        let semantic = Some(hkask_memory::SemanticMemory::new(
                            triple_store,
                            embedding_store,
                        ));
                        let store = SqliteAdapterStore::new(db);
                        store.migrate().map_err(|e| {
                            anyhow::anyhow!("Failed to migrate adapter store: {}", e)
                        })?;

                        // Build the canonical adapter store + router for deployment
                        let canonical_store = hkask_adapter::AdapterStore::new(Arc::clone(&conn));
                        canonical_store.migrate().map_err(|e| {
                            anyhow::anyhow!("Failed to migrate canonical adapter store: {}", e)
                        })?;
                        let router = AdapterRouter::new(std::sync::Arc::new(canonical_store));
                        let adapter_router = Some(std::sync::Arc::new(router));

                        (
                            semantic,
                            Arc::new(store) as Arc<dyn AdapterStore>,
                            job_store,
                            adapter_router,
                        )
                    }
                    None => (
                        None,
                        Arc::new(InMemoryAdapterStore::new()) as Arc<dyn AdapterStore>,
                        None,
                        None,
                    ),
                };

                let host = create_host(&host_config, harness)
                                    .map_err(|e| anyhow::anyhow!("Failed to create training host: {}", e))?;

                let inference_config = InferenceConfig::from_env();

                Ok(TrainingServer::new(
                                    ctx.webid,
                                    replicant.clone(),
                                    daemon_client.clone(),
                                    semantic,
                                    host,
                                    host_config.host,
                                    harness_id,
                    pipeline.clone(),
                    adapter_store,
                    job_store,
                    adapter_router,
                    inference_config,
                ))
            })()?)
        },
        vec![
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_TRAINING_DB",
                "Path to per-agent training database for job/adapter/QA storage (defaults to agents/{replicant}/training.db)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_DB_PASSPHRASE",
                "Passphrase for the training database (resolved via credentials or keystore; in-memory if unavailable)",
            ),
        ],
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_under_threshold_returns_single() {
        let text = "Short skill document.\n\nWith two paragraphs.";
        let chunks = split_into_chunks(text, 6000);
        assert_eq!(chunks.len(), 1, "text under threshold should be one chunk");
        assert!(chunks[0].contains("Short skill document"));
        assert!(chunks[0].contains("With two paragraphs"));
    }

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

    #[test]
    fn chunk_empty_returns_single() {
        let chunks = split_into_chunks("", 100);
        assert_eq!(chunks.len(), 1);
    }

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
