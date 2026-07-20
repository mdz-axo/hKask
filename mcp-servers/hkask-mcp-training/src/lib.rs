//! hKask MCP Training — Model training data ingestion and fine-tuning server.
//!
//! Exposes 14 tools (simplified from 21 — 7 speculative/inference/dup tools deleted):
//! - `training_ingest_qa` — Ingest QA pairs for model fine-tuning
//! - `training_submit` — Submit a training job via pluggable host
//! - `training_status` — Query training job status (auto-registers adapter on completion)
//! - `training_cancel` — Cancel a running job
//! - `training_list_adapters` — List completed LoRA adapters
//! - `training_delete_adapter` — Remove a LoRA adapter
//! - `training_assemble_dataset` — Assemble stored QA pairs into a ChatML JSONL dataset file
//! - `training_evaluate` — Evaluate a trained adapter against a test dataset
//! - `training_register_adapter` — Register a completed adapter in persistent storage
//! - `training_retrain` — Retrain an adapter with curated feedback (closes the continuous loop)
//! - `training_ingest_dataset` — Ingest a raw dataset into the normalized cache
//! - `training_preflight_check` — Pre-flight checks on a trained adapter before eval/deployment
//! - `training_deploy` — Deploy a trained adapter to a cloud inference endpoint
//! - `training_deployment_status` — Check the status of a deployed adapter endpoint
//! - `training_teardown` — Tear down a deployed adapter endpoint
//!
//! Deleted tools (2026-07-19 cleanup): training_generate_traces, training_generate_chain_of_thought
//! (inference jobs, not training), training_sweep (use submit in a loop), training_merge_adapters
//! (speculative, never produced output), training_record_invocation, training_curate_feedback
//! (data curation, not training), training_recommend_model (can be done offline).
//!
//! Architecture:
//!   Dataset file → DatasetPipeline → normalized ChatML → TrainingJob → TrainingHost → TrainedLoRAAdapter
//!
//! Host selection via config (training.host in settings.json), routed through
//! the shared `hkask-services` config init. Host pluggability is via the
//! `TrainingHost` trait, isolating the MCP surface from framework-specific details.
//!
//! # Environment Variables
//!
//! - `HKASK_TRAINING_DB` — Path to per-agent training database for job/adapter/QA storage (defaults to `agents/{replicant}/training.db`)
//! - `HKASK_DB_PASSPHRASE` — Passphrase for the database (resolved via credentials or keystore)
//! - `HKASK_TRAINING_HOST` — Override host (together|runpod|tinker) — where compute runs
//! - `HKASK_TRAINING_HARNESS` — Override harness (axolotl|unsloth|tinker) — what tooling runs
//! - `HKASK_TRAINING_CACHE_DIR` — Dataset cache directory
//! - `TG_API_KEY` — Together AI API key (for Together host)
//! - `RUNPOD_API_KEY` — Runpod API key (for Runpod host)
//! - `RUNPOD_TEMPLATE_ID` — Runpod GPU pod template ID with axolotl pre-installed
//! - `RUNPOD_GPU_TYPE_ID` — GPU type ID for Runpod pods (default: "NVIDIA RTX 4090")
//! - `RUNPOD_CONTAINER_DISK_GB` — Container disk GB for Runpod pods (default: 50)
//! - `RUNPOD_MIN_MEMORY_GB` — Minimum memory GB for Runpod pods (default: 24)
//! - `HKASK_DATASET_URL` — Public URL for dataset download by Runpod pods
//! - `HKASK_PYTHON_PATH` — Path to python3 interpreter (for Unsloth/Tinker host)
//! - `HKASK_PODS_FILE` — Path to RunPod pod ID persistence file (default: data/training-pods.json)
//!   Ensures pod IDs survive restarts so orphaned pods can be terminated.
//! - `TOGETHER_POLL_MAX_ATTEMPTS` — Max poll attempts for Together AI fine-tune jobs (default: 720)
//! - `TOGETHER_POLL_INTERVAL_SECS` — Poll interval in seconds (default: 30)

#![allow(unused_crate_dependencies)] // Bin target — deps used in main.rs, lint checks lib target only

pub mod adapters;
pub mod dataset;
pub mod huggingface;
pub mod lora_validation;
pub mod mlschema;
pub mod providers;
pub mod types;

// Bridge crates: shared ontological vocabulary (P5.4 dual-axis framework)

use crate::adapters::{AdapterMetrics, JobStore};
use crate::dataset::DatasetPipeline;
use crate::huggingface::HuggingFaceTraining;
use crate::providers::{
    AxolotlHarness, HarnessAdapter, TinkerHarness, TrainingHarnessId, TrainingHost,
    TrainingHostConfig, TrainingHostId, TrainingJob, TrainingJobStatus, UnslothHarness,
    create_host,
};
use crate::types::*;
use hkask_adapter::AdapterPort;
use hkask_adapter::AdapterRouter;
use hkask_adapter::adapter_store::Checksum;
use hkask_adapter::expertise::{AdapterLifecycle, Expertise, MdsDomain, TrainingProvenance};
use hkask_adapter::{AdapterSource, TrainedLoRAAdapter};
use hkask_adapter::{EndpointLifecycle, EndpointPhase};
use hkask_inference::{InferenceConfig, InferenceRouter};

use hkask_mcp::server::{McpToolError, execute_tool};
use hkask_memory::SemanticMemory;
use hkask_ports::InferencePort;
use hkask_storage::HMem;
use hkask_types::Visibility;
use hkask_types::template::LLMParameters;
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use serde_json::json;
use sha2::Digest;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

// ── Server ───────────────────────────────────────────────────────────────

hkask_mcp::mcp_server!(
    pub struct TrainingServer {
        pub semantic: Option<SemanticMemory>,
        pub host: Box<dyn TrainingHost>,
        pub host_id: TrainingHostId,
        pub harness_id: TrainingHarnessId,
        pub pipeline: Mutex<DatasetPipeline>,
        pub adapter_store: Arc<hkask_adapter::AdapterStore>,
        pub job_store: Option<JobStore>,
        pub adapter_router: Option<Arc<AdapterRouter>>,
        pub inference_config: InferenceConfig,
        pub deployments: Mutex<HashMap<String, AdapterDeployment>>,
    }
);

// ── Tools ────────────────────────────────────────────────────────────────

/// Compute SHA-256 checksum of the adapter weights file.
/// Returns `None` if the file cannot be read.
fn compute_adapter_checksum(path: &std::path::Path) -> Option<Checksum> {
    use sha2::Digest;
    let data = std::fs::read(path).ok()?;
    let hash = sha2::Sha256::digest(&data);
    Some(Checksum::from_hex(&format!("{:x}", hash)))
}

#[tool_router(server_handler)]
impl TrainingServer {
    /// Build a `TrainedLoRAAdapter` from training tool parameters.
    ///
    /// Constructs the canonical adapter type directly, with provenance metadata
    /// linking back to the originating training job.
    ///
    /// `checksum` and `storage_path` are computed from the adapter weights file
    /// when `adapter_weight_path` is provided. When `None`, placeholder values
    /// are used (zero checksum, empty path) — the adapter cannot be deployed
    /// via `AdapterRouter` until real values are provided.
    #[allow(clippy::too_many_arguments)]
    fn build_trained_adapter(
        id: String,
        name: String,
        base_model: String,
        dataset_hash: String,
        training_job_id: String,
        created_at_ts: i64,
        size_bytes: u64,
        skill_name: String,
        version: u32,
        metrics: Option<AdapterMetrics>,
        adapter_weight_path: Option<&std::path::Path>,
    ) -> TrainedLoRAAdapter {
        let metrics_json = metrics
            .as_ref()
            .and_then(|m| serde_json::to_value(m).ok())
            .unwrap_or_default();
        let created_at = chrono::DateTime::from_timestamp(created_at_ts, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default();
        let provenance = TrainingProvenance {
            training_run_id: training_job_id,
            training_source: String::new(),
            completed_at: created_at.clone(),
            base_model_family: base_model.clone(),
            dataset_hash: if dataset_hash.is_empty() {
                None
            } else {
                Some(dataset_hash)
            },
            training_metrics: metrics_json,
        };
        let expertise = Expertise::new(
            if name.trim().is_empty() {
                format!("adapter-{}", &id[..8.min(id.len())])
            } else {
                name.clone()
            },
            MdsDomain::CodeGeneration,
            serde_json::Value::Null,
            provenance,
        )
        .unwrap_or_else(|_| Expertise {
            name,
            domain: MdsDomain::CodeGeneration,
            capability_manifest: serde_json::Value::Null,
            training_source: TrainingProvenance {
                training_run_id: String::new(),
                training_source: String::new(),
                completed_at: String::new(),
                base_model_family: String::new(),
                dataset_hash: None,
                training_metrics: serde_json::Value::Null,
            },
        });
        let uuid = uuid::Uuid::parse_str(&id).unwrap_or_else(|_| uuid::Uuid::new_v4());
        // Compute checksum and storage_path from the adapter weights file.
        // When no path is provided, use placeholder values — the adapter cannot
        // be deployed via AdapterRouter until real values are set.
        let (checksum, storage_path) = match adapter_weight_path {
            Some(path) => {
                let storage = path.to_string_lossy().to_string();
                let hash = compute_adapter_checksum(path)
                    .unwrap_or_else(|| Checksum::from_hex("0000000000000000"));
                (hash, storage)
            }
            None => (Checksum::from_hex("0000000000000000"), String::new()),
        };
        TrainedLoRAAdapter {
            id: uuid,
            expertise,
            checksum,
            storage_path,
            base_model_family: base_model,
            version: Some(version.to_string()),
            source: AdapterSource::HuggingFace {
                repo: format!("hkask-training/{}", uuid),
            },
            size_bytes: if size_bytes > 0 {
                Some(size_bytes)
            } else {
                None
            },
            owner: hkask_types::id::WebID::from_persona(b"training-pipeline"),
            skill_name: if skill_name.is_empty() {
                None
            } else {
                Some(skill_name)
            },
            lifecycle: AdapterLifecycle::Durable,
            created_at,
        }
    }

    /// Parse the `training_metrics` JSON value from a `TrainedLoRAAdapter` back into
    /// `AdapterMetrics`. Returns `None` if the value is null or cannot be deserialized.
    fn metrics_from_trained(adapter: &TrainedLoRAAdapter) -> Option<AdapterMetrics> {
        serde_json::from_value(adapter.expertise.training_source.training_metrics.clone()).ok()
    }

    /// Resolve the adapter weight path for a given adapter ID from the training host.
    /// Returns `None` for cloud hosts where weights are server-side only.
    async fn resolve_adapter_path(&self, adapter_id: &str) -> Option<std::path::PathBuf> {
        self.host
            .adapter_weight_path(adapter_id)
            .await
            .ok()
            .flatten()
    }

    #[tool(
        description = "Ingest QA pairs for model training. Stores question-answer pairs with provenance in semantic memory for future fine-tuning dataset assembly."
    )]
    pub async fn training_ingest_qa(
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
                // Entity uses "manual" segment to avoid collision with docproc_ingest_qa's
                // `training:qa:{ds}:{source}:{i}` entities. Both write `training_qa_pair`
                // h_mems; the namespace separation prevents silent overwrites when both
                // tools write to the same dataset+source. See TRN-007.
                let entity = format!("training:qa:manual:{ds}:{source}:{i}");
                let level = qa.bloom_level.as_deref().unwrap_or("factual");
                let value = json!({
                    "question": qa.question,
                    "answer": qa.answer,
                    "bloom_level": level,
                    "source": source,
                    "dataset": ds,
                });

                let h_mem = HMem::new(&entity, "training_qa_pair", value, self.webid)
                    .with_visibility(Visibility::Public)
                    .with_confidence(1.0);

                match semantic.store(h_mem) {
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
    pub async fn training_submit(
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
                    target: "hkask.training.provenance.resolved",
                    model_id = %p.model_id,
                    architecture = %p.architecture,
                    license = ?p.license,
                    lora_compatible = p.lora_compatible,
                    is_gated = p.is_gated,
                    "Model provenance resolved"
                );
            }

            let num_epochs = params.as_ref().map(|p| p.num_epochs).unwrap_or(3);
            let resolved_params = params.unwrap_or_default();

            // Validate training params against LoRA/QLoRA math-contract gates
            // (lora-training skill: G-M1..G-M5, G-Q1..G-Q6). Static subset only —
            // runtime gates (gradient flow, param count) are checked post-training.
            let validation_findings = lora_validation::validate_training_params(&resolved_params);
            if lora_validation::has_refusals(&validation_findings) {
                let refusals: Vec<_> = validation_findings
                    .iter()
                    .filter(|f| f.severity == lora_validation::ValidationSeverity::Refuse)
                    .collect();
                let messages: Vec<String> = refusals
                    .iter()
                    .map(|f| format!("{}: {}", f.gate_id, f.message))
                    .collect();
                tracing::warn!(
                    target: "hkask.training.validation.refused",
                    gate_count = refusals.len(),
                    gates = ?refusals.iter().map(|f| f.gate_id).collect::<Vec<_>>(),
                    "Training job refused — math-contract gate violation"
                );
                return Err(McpToolError::invalid_argument(format!(
                    "Training config failed math-contract validation: {}",
                    messages.join("; ")
                )));
            }
            // Log warnings and info findings — do not block submission.
            for finding in &validation_findings {
                if finding.severity == lora_validation::ValidationSeverity::Warn {
                    tracing::warn!(
                        target: "hkask.training.validation.warn",
                        gate = finding.gate_id,
                        message = %finding.message,
                        remediation = %finding.remediation,
                        "Training config warning"
                    );
                }
            }

            let mut job = TrainingJob {
                id: uuid::Uuid::new_v4().to_string(),
                dataset_path: normalized_path.clone(),
                base_model: base_model.clone(),
                params: resolved_params,
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
                artifacts: None,
            };

            if self.host_id == TrainingHostId::Runpod {
                let bytes = std::fs::read(&normalized_path).map_err(|error| {
                    McpToolError::internal(format!("Read normalized dataset for publication: {error}"))
                })?;
                let dataset_sha256 = format!("{:x}", sha2::Sha256::digest(&bytes));
                let training = HuggingFaceTraining::from_env().map_err(|error| {
                    McpToolError::failed_precondition(format!("Configure Hugging Face training artifacts: {error}"))
                })?;
                let dataset = training
                    .publish_dataset(&job.id, bytes, &dataset_sha256)
                    .await
                    .map_err(|error| McpToolError::internal(format!("Publish training dataset: {error}")))?;
                job.artifacts = Some(
                    training
                        .prepare_training_artifacts(&job.id, dataset)
                        .await
                        .map_err(|error| McpToolError::internal(format!("Prepare training artifacts: {error}")))?,
                );
            }

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
                        target: "hkask.training.job.persist",
                        job_id = %job.id,
                        error = %e,
                        "Failed to persist job"
                    );
                }
            }

            if let (Some(job_store), Some(artifacts)) = (&self.job_store, &job.artifacts) {
                job_store.update_artifacts(&job.id, artifacts).map_err(|error| {
                    McpToolError::internal(format!("Persist training artifacts: {error}"))
                })?;
            }

            match self.host.submit(&job).await {
                Ok(provider_job_id) => {
                    if let Some(job_store) = &self.job_store {
                        job_store.update_provider_job_id(&job.id, &provider_job_id).map_err(|error| {
                            McpToolError::internal(format!("Persist provider job ID: {error}"))
                        })?;
                    }
                    let mut result = json!({
                        "job_id": job.id,
                        "provider_job_id": provider_job_id,
                        "status": "queued",
                        "base_model": base_model,
                        "host": format!("{:?}", self.host_id),
                    });
                    result["estimated_cost_urj"] = json!(job.estimated_cost_urj);
                    tracing::info!(
                        target: "hkask.qa.cost.training_job",
                        job_id = %job.id,
                        provider_job_id = %provider_job_id,
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
                        target: "hkask.training.job.fail",
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
    pub async fn training_status(
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
                                target: "hkask.training.job.persist",
                                job_id = %job_id,
                                error = %e,
                                "Failed to update job status"
                            );
                        }
                    }

                    // Auto-register adapter on completion
                    if status == TrainingJobStatus::Completed {
                        let adapter: TrainedLoRAAdapter = match self
                            .adapter_store
                            .get_by_id(uuid::Uuid::parse_str(&job_id).unwrap_or_default())
                            .map_err(|e| McpToolError::internal(format!("Adapter store error: {e}")))?
                        {
                            Some(existing) => {
                                result["adapter_registered"] = json!(true);
                                result["adapter_note"] =
                                    json!("Already registered (pre-registered by retrain)");
                                existing
                            }
                            None => {
                                // Fresh auto-registration from host completion metadata
                                match self.host.completion_metadata(&job_id).await {
                                    Ok(Some(meta)) => {
                                        // Resolve adapter weight path from the training host
                                        let weight_path = self.resolve_adapter_path(&job_id).await;
                                        let adapter = Self::build_trained_adapter(
                                            job_id.clone(),
                                            meta.output_name
                                                .unwrap_or_else(|| format!("adapter-{}", &job_id[..8])),
                                            meta.base_model.clone(),
                                            String::new(),
                                            job_id.clone(),
                                            chrono::Utc::now().timestamp(),
                                            0,
                                            String::new(),
                                            1,
                                            Some(AdapterMetrics {
                                                loss: meta.loss,
                                                perplexity: None,
                                                training_duration_secs: meta.training_duration_secs,
                                                tokens_processed: meta.tokens_processed,
                                            }),
                                            weight_path.as_deref(),
                                        );
                                        match self
                                            .adapter_store
                                            .store(&adapter)
                                            .map_err(|e| McpToolError::internal(format!("Adapter store error: {e}")))
                                        {
                                            Ok(()) => {
                                                result["adapter_registered"] = json!(true);
                                                result["adapter_name"] = json!(adapter.expertise.name);
                                                result["base_model"] = json!(meta.base_model);

                                                // Store adapter weight blob if available locally
                                                match self.host.adapter_weight_path(&job_id).await {
                                                    Ok(Some(weight_path)) => {
                                                        match tokio::fs::read(&weight_path).await {
                                                            Ok(blob) => {
                                                                let size = blob.len() as u64;
                                                                if let Err(e) = self
                                                                    .adapter_store
                                                                    .store_blob(adapter.id, &blob)
                                                                    .map_err(|e| McpToolError::internal(format!("Adapter store error: {e}")))
                                                                {
                                                                    tracing::warn!(
                                                                        target: "hkask.training.adapter.blob",
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
                                                                    target: "hkask.training.adapter.blob",
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
                                                    target: "hkask.training.adapter.created",
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
                        let adapter_skill = adapter.skill_name.clone().unwrap_or_default();
                        if !adapter_skill.is_empty() {
                            let current_loss =
                                Self::metrics_from_trained(&adapter).and_then(|m| m.loss);
                            if let Some(prev) = self
                                .adapter_store
                                .get_by_skill_name(&adapter_skill)
                                .map_err(|e| McpToolError::internal(format!("Adapter store error: {e}")))?
                            {
                                // Don't compare against self
                                if prev.id != adapter.id
                                    && let (Some(new_loss), Some(prev_loss)) = (
                                        current_loss,
                                        Self::metrics_from_trained(&prev).and_then(|m| m.loss),
                                    )
                                {
                                    let improved = new_loss < prev_loss;
                                    result["ab_comparison"] = json!({
                                        "skill_name": adapter_skill,
                                        "previous_version": prev.version,
                                        "previous_adapter_name": prev.expertise.name,
                                        "previous_loss": prev_loss,
                                        "new_version": adapter.version,
                                        "new_loss": new_loss,
                                        "loss_improved": improved,
                                        "auto_promoted": improved,
                                    });
                                    tracing::info!(
                                        target: "hkask.training.retrain.ab",
                                        skill = %adapter_skill,
                                        prev_version = ?prev.version,
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
    pub async fn training_cancel(
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
    pub async fn training_list_adapters(&self) -> String {
        execute_tool(self, "training_list_adapters", async {
            match self.host.list_adapters().await {
                Ok(adapter_ids) => {
                    let mut metadata_list: Vec<serde_json::Value> = Vec::new();
                    for id in &adapter_ids {
                        let entry = match self
                            .adapter_store
                            .get_by_id(uuid::Uuid::parse_str(id).unwrap_or_default())
                            .map_err(|e| McpToolError::internal(format!("Adapter store error: {e}")))?
                        {
                            Some(adapter) => {
                                let metrics = Self::metrics_from_trained(&adapter).map(|m| json!({
                                    "loss": m.loss,
                                    "perplexity": m.perplexity,
                                    "training_duration_secs": m.training_duration_secs,
                                    "tokens_processed": m.tokens_processed,
                                }));
                                json!({
                                    "id": adapter.id.to_string(),
                                    "name": adapter.expertise.name,
                                    "skill_name": adapter.skill_name.unwrap_or_default(),
                                    "version": adapter.version.unwrap_or_default(),
                                    "base_model": adapter.base_model_family,
                                    "dataset_hash": adapter.expertise.training_source.dataset_hash.unwrap_or_default(),
                                    "training_job_id": adapter.expertise.training_source.training_run_id,
                                    "created_at": adapter.created_at,
                                    "size_bytes": adapter.size_bytes.unwrap_or_default(),
                                    "metrics": metrics,
                                })
                            }
                            None => json!({"id": id, "warning": "metadata not found in store"}),
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
    pub async fn training_delete_adapter(
        &self,
        Parameters(TrainDeleteAdapterRequest { adapter_id }): Parameters<TrainDeleteAdapterRequest>,
    ) -> String {
        execute_tool(self, "training_delete_adapter", async {
            // Delete from host storage (filesystem)
            if let Err(e) = self.host.delete_adapter(&adapter_id).await {
                // Non-fatal — host storage may already be gone, still clean up metadata
                tracing::warn!(
                    target: "hkask.training.adapter.deleted",
                    adapter_id = %adapter_id,
                    error = %e,
                    "Host deletion failed, continuing with metadata cleanup"
                );
            }

            // Delete from adapter store (metadata + blob)
            match self
                .adapter_store
                .delete(uuid::Uuid::parse_str(&adapter_id).unwrap_or_default())
            {
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
        description = "Assemble stored QA pairs into a ChatML JSONL training dataset file. Queries semantic memory for training_qa_pair h_mems, filters by dataset/source/bloom level, and writes a file ready for training_submit. Optionally splits into train/test."
    )]
    pub async fn training_assemble_dataset(
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

            let h_mems = match semantic.query_by_attribute("training_qa_pair") {
                Ok(t) => t,
                Err(e) => {
                    return Err(McpToolError::internal(format!("Failed to query QA h_mems: {}", e)));
                }
            };

            if h_mems.is_empty() {
                return Err(McpToolError::invalid_argument(
                    "No training_qa_pair h_mems found in semantic memory. Ingest QA pairs first with training_ingest_qa.",
                ));
            }

            // Parse and filter QA pairs
            let mut conversations: Vec<serde_json::Value> = Vec::new();
            for h_mem in &h_mems {
                let value = &h_mem.value;
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
        description = "Evaluate a trained adapter against a test dataset. Runs inference for each test example and scores accuracy using exact match, substring containment, or semantic comparison. The model must be deployed and available for inference (Together AI fine-tuned models are auto-deployed; local adapters require the inference engine to have the adapter loaded)."
    )]
    pub async fn training_evaluate(
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
                            target: "hkask.training.evaluate",
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
                    // Unsloth generate() rejects temperature=0.0; use 1.0 with
                    // do_sample=False for deterministic output.
                    temperature: 1.0,
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
                            target: "hkask.training.evaluate",
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
        description = "Run pre-flight checks on a trained LoRA adapter before evaluation or deployment. Verifies (1) adapter_config.json parses and init_lora_weights is valid, (2) adapter_model.safetensors exists and is non-empty, (3) optional sanity check — a test prompt produces output > min_response_chars. Emits hkask.training.preflight telemetry spans. Fail-fast: returns on first failure."
    )]
    pub async fn training_preflight_check(
        &self,
        Parameters(TrainPreflightCheckRequest {
            adapter_path,
            model,
            test_prompt,
            min_response_chars,
        }): Parameters<TrainPreflightCheckRequest>,
    ) -> String {
        execute_tool(self, "training_preflight_check", async {
                        let adapter_dir = PathBuf::from(&adapter_path);
                        let mut checks: Vec<serde_json::Value> = Vec::new();
                        let mut all_pass = true;

                        // ── Check 1: adapter_config.json loads and init_lora_weights is valid ──
                        let config_path = adapter_dir.join("adapter_config.json");
                        let check1 = if !config_path.exists() {
                            all_pass = false;
                            json!({
                                "check": "load",
                                "status": "fail",
                                "reason": format!("adapter_config.json not found at {}", config_path.display())
                            })
                        } else {
                            match std::fs::read_to_string(&config_path) {
                                Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
                                    Ok(cfg) => {
                                        let init = cfg.get("init_lora_weights");
                                        let init_valid = match init {
                                            Some(serde_json::Value::Bool(true)) => true,
                                            Some(serde_json::Value::String(s)) => s == "pissa_niter_4" || s == "pissa_niter_8",
                                            _ => false,
                                        };
                                        if init_valid {
                                            json!({
                                                "check": "load",
                                                "status": "pass",
                                                "init_lora_weights": init,
                                                "r": cfg.get("r"),
                                                "lora_alpha": cfg.get("lora_alpha"),
                                                "base_model": cfg.get("base_model_name_or_path")
                                            })
                                        } else {
                                            all_pass = false;
                                            json!({
                                                "check": "load",
                                                "status": "fail",
                                                "reason": "init_lora_weights must be true (standard LoRA) or pissa_niter_N (PiSSA). Got: ".to_string() + &init.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string())
                                            })
                                        }
                                    }
                                    Err(e) => {
                                        all_pass = false;
                                        json!({
                                            "check": "load",
                                            "status": "fail",
                                            "reason": format!("adapter_config.json is not valid JSON: {e}")
                                        })
                                    }
                                },
                                Err(e) => {
                                    all_pass = false;
                                    json!({
                                        "check": "load",
                                        "status": "fail",
                                        "reason": format!("Cannot read adapter_config.json: {e}")
                                    })
                                }
                            }
                        };
                        tracing::info!(
                            target: "hkask.training.preflight",
                            check = "load",
                            status = check1.get("status").and_then(|s| s.as_str()).unwrap_or("unknown"),
                            "Pre-flight check: load"
                        );
                        checks.push(check1);

                        if !all_pass {
                            return Ok(json!({
                                "adapter_path": adapter_path,
                                "all_pass": false,
                                "checks": checks,
                                "failed_at": "load"
                            }));
                        }

                        // ── Check 2: adapter_model.safetensors exists and is non-empty ──
                        let weights_path = adapter_dir.join("adapter_model.safetensors");
                        let check2 = if !weights_path.exists() {
                            all_pass = false;
                            json!({
                                "check": "weights", "status": "fail",
                                "reason": format!("adapter_model.safetensors not found at {}", weights_path.display())
                            })
                        } else {
                            let size = std::fs::metadata(&weights_path).map(|m| m.len()).unwrap_or(0);
                            if size < 1024 {
                                all_pass = false;
                                json!({
                                    "check": "weights", "status": "fail",
                                    "reason": format!("adapter_model.safetensors is too small: {size} bytes"),
                                    "size_bytes": size
                                })
                            } else {
                                json!({
                                    "check": "weights", "status": "pass", "size_bytes": size})
                            }
                        };
                        tracing::info!(
                            target: "hkask.training.preflight",
                            check = "weights",
                            status = check2.get("status").and_then(|s| s.as_str()).unwrap_or("unknown"),
                            "Pre-flight check: weights"
                        );
                        checks.push(check2);

                        if !all_pass {
                            return Ok(json!({
                                "adapter_path": adapter_path,
                                "all_pass": false,
                                "checks": checks,
                                "failed_at": "weights"
                            }));
                        }

                        // ── Check 3 (optional): sanity — generate 1 example and check output length ──
                        if let Some(_model_id) = model {
                            let prompt = test_prompt.unwrap_or_else(|| {
                                "Generate a simple Rust function that adds two numbers.\n\nProvide just the code."
                                    .to_string()
                            });
                            let min_chars = min_response_chars.unwrap_or(50);

                            let router = InferenceRouter::new(self.inference_config.clone());
                            let params = LLMParameters {
                                temperature: 1.0,
                                max_tokens: 512,
                                ..Default::default()
                            };

                            let check3 = match router.generate(&prompt, &params, None).await {
                                Ok(response) => {
                                    let text = response.text.trim();
                                                                        let len = text.chars().count();
                                                                        if len >= min_chars {
                                        json!({
                                            "check": "sanity",
                                            "status": "pass",
                                            "response_chars": len,
                                            "response_preview": text.chars().take(200).collect::<String>()
                                        })
                                    } else {
                                        all_pass = false;
                                        json!({
                                            "check": "sanity",
                                            "status": "fail",
                                            "reason": format!("Response too short: {len} chars (minimum {min_chars})"),
                                            "response_chars": len,
                                            "response_preview": text
                                        })
                                    }
                                }
                                Err(e) => {
                                    all_pass = false;
                                    json!({
                                        "check": "sanity",
                                        "status": "fail",
                                        "reason": format!("Inference failed: {e}")
                                    })
                                }
                            };
                            tracing::info!(
                                target: "hkask.training.preflight",
                                check = "sanity",
                                status = check3.get("status").and_then(|s| s.as_str()).unwrap_or("unknown"),
                                "Pre-flight check: sanity"
                            );
                            checks.push(check3);
                        } else {
                            checks.push(json!({
                                "check": "sanity",
                                "status": "skipped",
                                "reason": "model not provided"
                            }));
                        }

                        Ok(json!({
                            "adapter_path": adapter_path,
                            "all_pass": all_pass,
                            "checks": checks
                        }))
                    })
                    .await
    }

    #[tool(
        description = "Register a completed LoRA adapter in the persistent store. Call after training completes to record adapter metadata for future listing, evaluation, and composition. Stores both metadata and links the adapter to its originating training job."
    )]
    pub async fn training_register_adapter(
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

            // Resolve adapter weight path from the training host
            let weight_path = self.resolve_adapter_path(&adapter_id).await;
            let adapter = Self::build_trained_adapter(
                adapter_id.clone(),
                name.clone(),
                base_model.clone(),
                dataset_hash.unwrap_or_default(),
                training_job_id.unwrap_or_default(),
                chrono::Utc::now().timestamp(),
                size_bytes.unwrap_or(0),
                skill_name.clone(),
                version.unwrap_or(1),
                metrics,
                weight_path.as_deref(),
            );

            match self
                .adapter_store
                .store(&adapter)
                .map_err(|e| McpToolError::internal(format!("Adapter store error: {e}")))
            {
                Ok(()) => Ok(json!({
                    "adapter_id": adapter_id,
                    "name": name,
                    "skill_name": skill_name,
                    "base_model": base_model,
                    "registered": true,
                })),
                Err(e) => Err(e),
            }
        })
        .await
    }

    #[tool(
        description = "Retrain an adapter with curated feedback for continuous skills training. Merges the original training dataset with a feedback JSONL file (from training_curate_feedback), submits a new training job with an incremented version number, and registers the new adapter on completion. This closes the continuous training loop: train → evaluate → curate → retrain."
    )]
    pub async fn training_retrain(
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
        if self.host_id == TrainingHostId::Runpod {
            return Err(McpToolError::failed_precondition(
                "RunPod retraining requires a published Hugging Face artifact path; use training_submit",
            ));
        }
        tracing::info!(
            target: "hkask.training.retrain.started",
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
            match self.adapter_store.get_by_skill_name(&skill_name) {
                Ok(Some(prev)) => {
                    let prev_version = prev
                        .version
                        .as_deref()
                        .and_then(|v| v.parse::<u32>().ok())
                        .unwrap_or(0);
                    (prev_version + 1, true)
                }
                _ => (1, false),
            };

        // A/B baseline: if a previous adapter exists, record its metrics
        // so training_status can compare when the new job completes.
        let ab_baseline: Option<AbBaseline> = if previous_adapter_exists {
            self.adapter_store
                .get_by_skill_name(&skill_name)
                .ok()
                .flatten()
                .as_ref()
                .and_then(Self::metrics_from_trained)
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
            artifacts: None,
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

        // Pre-register the adapter metadata so it's ready when training completes.
        // No weight path yet — the adapter hasn't been trained. Will be resolved
        // when training_status auto-registers on completion.
        let adapter = Self::build_trained_adapter(
            job.id.clone(),
            adapter_name.clone(),
            base_model.clone(),
            String::new(),
            job.id.clone(),
            chrono::Utc::now().timestamp(),
            0,
            skill_name.clone(),
            version,
            None,
            None,
        );

        if let Err(e) = self
            .adapter_store
            .store(&adapter)
            .map_err(|e| McpToolError::internal(format!("Adapter store error: {e}")))
        {
            tracing::warn!(
                target: "hkask.training.retrain",
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
                    target: "hkask.training.job.fail",
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
    pub async fn training_ingest_dataset(
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
        description = "Deploy a trained adapter to a cloud inference endpoint. Looks up the adapter by name from AdapterStore, resolves the base model, estimates cost and setup time per provider, and provisions or locates an endpoint. For Together AI, the fine-tuned model is auto-deployed — returns the model name directly. For Runpod, returns a deployment ID for status polling. CNS span: cns.training.deploy."
    )]
    pub async fn training_deploy(&self, Parameters(req): Parameters<TrainDeployRequest>) -> String {
        execute_tool(self, "training_deploy", async {
        // Look up adapter from store — validate it exists.
        // Try by exact ID first, then by skill/expertise name.
        let adapter_meta = match self
            .adapter_store
            .get_by_id(uuid::Uuid::parse_str(&req.adapter_name).unwrap_or_default())
            .map_err(|e| McpToolError::internal(format!("Adapter store error: {e}")))?
        {
            Some(meta) => meta,
            None => match self
                .adapter_store
                .get_by_skill_name(&req.adapter_name)
                .map_err(|e| McpToolError::internal(format!("Adapter store error: {e}")))?
            {
                Some(meta) => meta,
                None => {
                    return Err(McpToolError::invalid_argument(format!(
                        "Adapter '{}' not found by ID or skill name. Use training_list_adapters to see available adapters.",
                        req.adapter_name
                    )));
                }
            },
        };

        // If AdapterRouter is configured, use canonical deployment pipeline.
        if let Some(ref router) = self.adapter_router {
            let canonical = adapter_meta.clone();
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

            tracing::info!(target: "hkask.training.deploy", endpoint_id = %handle.endpoint_id, adapter = %req.adapter_name, provider = ?req.provider, "Adapter deployed via AdapterRouter");
            return Ok(result);
        }

        // Fallback: local deployment pipeline
        let base_model = req.base_model.unwrap_or(adapter_meta.base_model_family.clone());
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
        tracing::info!(target: "hkask.training.deploy", deployment_id = %deploy_id, adapter = %req.adapter_name, provider = ?req.provider, cost_hr = cost_hr, "Adapter deployment initiated");
        Ok(result)
        })
        .await
    }

    #[tool(
        description = "Check the status of a deployed adapter endpoint. Returns current provisioning state, endpoint URL when ready, and accumulated cost. CNS span: cns.training.deployment_status."
    )]
    pub async fn training_deployment_status(
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
    pub async fn training_teardown(
        &self,
        Parameters(req): Parameters<TrainTeardownRequest>,
    ) -> String {
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
        tracing::info!(target: "hkask.training.teardown", deployment_id = %req.deployment_id, existed = existed, "Adapter deployment torn down");
        Ok(result)
        })
        .await
    }
}

// ── Entry point ───────────────────────────────────────────────────────────

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
        tinker_api_key: std::env::var("TINKER_API_KEY").unwrap_or_default(),
        tinker_python_path: std::env::var("HKASK_PYTHON_PATH").unwrap_or_default(),
    };
    let harness: Box<dyn HarnessAdapter> = match harness_id {
        TrainingHarnessId::Axolotl => Box::new(AxolotlHarness),
        TrainingHarnessId::Unsloth => Box::new(UnslothHarness),
        TrainingHarnessId::Tinker => Box::new(TinkerHarness),
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
            (|| -> anyhow::Result<TrainingServer> {
                let db_path = ctx
                    .credentials
                    .get("HKASK_TRAINING_DB")
                    .cloned()
                    .unwrap_or_else(|| {
                        let relative = hkask_types::agent_paths::agent_training_db(&replicant);
                        hkask_types::agent_paths::resolve_under_data_dir(&relative)
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
                        let pool = db.sqlite_pool().map_err(|e| anyhow::anyhow!("pool: {e}"))?;
                        let job_store = Some(JobStore::new(pool.clone()));
                        let hmem_driver: Arc<dyn hkask_database::driver::DatabaseDriver> =
                            Arc::new(hkask_database::sqlite::SqliteDriver::new(pool.clone()));
                        let h_mem_store = hkask_storage::HMemStore::from_driver(Arc::clone(&hmem_driver));
                        let embedding_store = hkask_storage::EmbeddingStore::from_driver(
                            Arc::new(hkask_database::sqlite::SqliteDriver::new(pool)),
                            1024,
                        );
                        let semantic = Some(hkask_memory::SemanticMemory::new(
                            h_mem_store,
                            embedding_store,
                        ));
                        // Canonical adapter store: hkask_adapter::AdapterStore stores
                        // TrainedLoRAAdapter in trained_adapters + active_endpoints + lora_blobs.
                        // Schema initialized by from_driver().
                        let store = hkask_adapter::AdapterStore::from_driver(hmem_driver);

                        // Build the canonical adapter router for deployment
                        let router = AdapterRouter::new(std::sync::Arc::new(store.clone()));
                        let adapter_router = Some(std::sync::Arc::new(router));

                        (
                            semantic,
                            Arc::new(store),
                            job_store,
                            adapter_router,
                        )
                    }
                    None => {
                        // No passphrase configured — fall back to an in-memory driver
                        // so the server still runs (no persistence across restarts).
                        let pool = hkask_database::sqlite::SqliteDriver::in_memory_pool()
                            .map_err(|e| anyhow::anyhow!("in-memory pool: {e}"))?;
                        let driver: Arc<dyn hkask_database::driver::DatabaseDriver> =
                            Arc::new(hkask_database::sqlite::SqliteDriver::new(pool));
                        let store = hkask_adapter::AdapterStore::from_driver(driver);
                        (
                            None,
                            Arc::new(store),
                            None,
                            None,
                        )
                    }
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
                                    Mutex::new(pipeline.clone()),
                                    adapter_store,
                                    job_store,
                                    adapter_router,
                                    inference_config,
                                    Mutex::new(HashMap::new()),
                                ))
            })()
                .map_err(|e| hkask_mcp::McpError::UnexpectedResponse {
                    context: "training server init".into(),
                    detail: e.to_string(),
                })
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
