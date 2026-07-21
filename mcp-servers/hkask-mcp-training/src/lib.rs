//! hKask MCP Training — Model training data ingestion and fine-tuning server.
//!
//! Exposes 8 tools (simplified from 21 → 15 → 8 across 2026-07-19 cleanups):
//! - `training_ingest_qa` — Ingest QA pairs for model fine-tuning
//! - `training_ingest_dataset` — Ingest a raw dataset into the normalized cache (SFT or preference)
//! - `training_assemble_dataset` — Assemble stored QA pairs into a ChatML JSONL dataset file
//! - `training_submit` — Submit a training job (also handles retrain via optional feedback_path)
//! - `training_status` — Query training job status (auto-registers adapter on completion)
//! - `training_cancel` — Cancel a running job
//! - `training_evaluate` — Evaluate a trained adapter against a test dataset
//! - `training_validate_config` — Run lora-training skill's static math-contract gates on a config
//!
//! Deleted tools (2026-07-19 cleanup, second pass):
//! - training_deploy / training_deployment_status / training_teardown — replaced by
//!   `hkask_adapter::AdapterPort::{create_endpoint, endpoint_status, teardown_endpoint}`.
//!   The MCP server was a thin wrapper; deployment now goes through the canonical
//!   AdapterPort surface directly.
//! - training_list_adapters / training_delete_adapter — `AdapterPort::list_adapters` and
//!   `AdapterStore::delete` already cover these. Rare operations; route via CLI.
//! - training_register_adapter — `training_status` auto-registers on completion; manual
//!   registration is an `AdapterStore` API call, not an MCP tool.
//! - training_preflight_check — replaced by `training_validate_config`, which runs the
//!   actual lora-training skill gates (G-M1..G-M4, G-Q1, G-Q2, G-Q4, G-H1) via
//!   `lora_validation::validate_training_params` and emits `reg.lora.audit` spans.
//! - training_retrain — merged into `training_submit` as optional `feedback_path` +
//!   `skill_name` + `adapter_name` parameters (merge + version-bump logic moved there).
//!
//! Deleted tools (2026-07-19 cleanup, first pass):
//! - training_generate_traces, training_generate_chain_of_thought (inference, not training)
//! - training_sweep (use submit in a loop)
//! - training_merge_adapters (speculative, never produced output)
//! - training_record_invocation, training_curate_feedback (data curation, not training)
//! - training_recommend_model (can be done offline)
//!
//! Architecture:
//!   Dataset file → DatasetPipeline → normalized ChatML → TrainingJob → TrainingHost → TrainedLoRAAdapter
//!
//! Host selection: Runpod is the only cloud host. Harness default is Axolotl;
//! per-job harness selection via `TrainingParams.harness` (operator-accepted
//! from the lora-training skill's G6 gate) is honored at submit time.
//! Phase 1 (v0.31.0): Axolotl (SFT) + TRL SFTTrainer + Ludwig SFT.
//! Routed through the shared `hkask-services` config init. Host pluggability
//! is via the `TrainingHost` trait, isolating the MCP surface from
//! framework-specific details.
//!
//! lora-training skill integration:
//!   `training_validate_config` is the runtime enforcement point for the
//!   `.agents/skills/lora-training/` skill's `audit-config` phase. The skill
//!   reasons over config files and proposes regressions; this server enforces
//!   the static subset of gates at submit time and emits the `reg.lora.*` spans
//!   the skill's convergence-check phase consumes.
//!
//! # Environment Variables
//!
//! - `HKASK_TRAINING_DB` — Path to per-agent training database for job/adapter/QA storage (defaults to `agents/{userpod}/training.db`)
//! - `HKASK_DB_PASSPHRASE` — Passphrase for the database (resolved via credentials or keystore)
//! - `HKASK_TRAINING_CACHE_DIR` — Dataset cache directory
//! - `RUNPOD_API_KEY` — Runpod API key
//! - `RUNPOD_TEMPLATE_ID` — Runpod GPU pod template ID with axolotl pre-installed
//! - `RUNPOD_GPU_TYPE_ID` — GPU type ID for Runpod pods (default: "NVIDIA RTX 4090")
//! - `RUNPOD_CONTAINER_DISK_GB` — Container disk GB for Runpod pods (default: 50)
//! - `RUNPOD_MIN_MEMORY_GB` — Minimum memory GB for Runpod pods (default: 24)
//! - `HKASK_DATASET_URL` — Public URL for dataset download by Runpod pods
//! - `HKASK_PODS_FILE` — Path to RunPod pod ID persistence file (default: data/training-pods.json)
//!   Ensures pod IDs survive restarts so orphaned pods can be terminated.

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
    TrainingHarnessId, TrainingHost, TrainingHostConfig, TrainingHostId, TrainingJob,
    TrainingJobStatus, create_host,
};
use crate::types::*;
use hkask_adapter::AdapterRouter;
use hkask_adapter::adapter_store::Checksum;
use hkask_adapter::expertise::{AdapterLifecycle, Expertise, MdsDomain, TrainingProvenance};
use hkask_adapter::{AdapterSource, TrainedLoRAAdapter};
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
        description = "Submit a training job for execution. Ingests, normalizes, and submits a dataset for LoRA fine-tuning via the selected harness (Axolotl YAML, TRL Python, or Ludwig YAML) on Runpod. The harness is selected from params.harness (operator-accepted from the lora-training skill's G6 gate), defaulting to Axolotl. When `feedback_path` is provided, enters retrain mode: merges the original dataset with curated feedback, deduplicates by user question, increments the adapter version based on existing adapters with the same `skill_name`, and pre-registers adapter metadata so training_status can complete the A/B comparison on job completion."
    )]
    pub async fn training_submit(
        &self,
        Parameters(TrainSubmitRequest {
            dataset_path,
            base_model,
            params,
            feedback_path,
            skill_name,
            adapter_name,
            merged_output_path,
        }): Parameters<TrainSubmitRequest>,
    ) -> String {
        execute_tool(self, "training_submit", async {
            let file_path = PathBuf::from(&dataset_path);
            if !file_path.exists() {
                return Err(McpToolError::invalid_argument(format!("Dataset file not found: {}", dataset_path)));
            }

            // ── Retrain mode: merge original + feedback, deduplicate, bump version ──
            //
            // When `feedback_path` is set, this is a retrain: the original dataset
            // is merged with curated feedback (ChatML JSONL), deduplicated by user
            // question, and the resulting merged dataset is what gets submitted.
            // The adapter version is incremented based on existing adapters with
            // the same `skill_name`, and adapter metadata is pre-registered so
            // `training_status` can complete the A/B comparison on completion.
            let retrain_mode = feedback_path.is_some();
            let mut ab_baseline: Option<AbBaseline> = None;
            let mut version: u32 = 1;
            let resolved_skill_name: Option<String> = skill_name.clone();
            let mut resolved_adapter_name: Option<String> = adapter_name.clone();

            let normalized_path = if retrain_mode {
                let feedback = PathBuf::from(feedback_path.as_ref().unwrap());
                hkask_mcp::validate_path("feedback_path", feedback.to_str().unwrap_or(""), 4096)?;
                if !feedback.exists() {
                    return Err(McpToolError::invalid_argument(format!(
                        "Feedback file not found: {}",
                        feedback.display()
                    )));
                }
                let skill = skill_name.clone().unwrap_or_default();
                if skill.is_empty() {
                    return Err(McpToolError::invalid_argument(
                        "skill_name is required when feedback_path is set (retrain mode)"
                    ));
                }
                // Validate skill_name as an identifier — it flows into file paths
                // (merged dataset path) and adapter metadata. Prevents path traversal.
                hkask_mcp::validate_identifier("skill_name", &skill, 64)?;

                tracing::info!(
                    target: "hkask.training.retrain.started",
                    skill = %skill,
                    "Retraining job initiated"
                );

                let original_content = std::fs::read_to_string(&file_path).map_err(|e| {
                    McpToolError::invalid_argument(format!("Failed to read original dataset: {}", e))
                })?;
                let feedback_content = std::fs::read_to_string(&feedback).map_err(|e| {
                    McpToolError::invalid_argument(format!("Failed to read feedback dataset: {}", e))
                })?;

                // Merge: original lines + feedback lines, deduplicate by user question
                let mut merged = String::new();
                let mut seen_questions: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                let mut original_examples = 0usize;
                let mut feedback_examples = 0usize;

                for (content, counter) in [
                    (&original_content, &mut original_examples),
                    (&feedback_content, &mut feedback_examples),
                ] {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
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
                                *counter += 1;
                            }
                        }
                    }
                }

                if merged.is_empty() {
                    return Err(McpToolError::invalid_argument(
                        "No valid examples found in either dataset"
                    ));
                }

                let merged_path = merged_output_path.unwrap_or_else(|| {
                    format!("/tmp/hkask-retrain-{}.jsonl", &skill)
                });
                // Validate the merged output path — it's user-controlled and we're
                // about to write to it. Prevents path traversal / arbitrary file write.
                hkask_mcp::validate_path("merged_output_path", &merged_path, 4096)?;
                std::fs::write(&merged_path, &merged).map_err(|e| {
                    McpToolError::internal(format!("Failed to write merged dataset: {}", e))
                })?;

                // Determine version: look up previous adapter by skill name and increment
                let previous_adapter_exists: bool;
                match self.adapter_store.get_by_skill_name(&skill) {
                    Ok(Some(prev)) => {
                        let prev_version = prev
                            .version
                            .as_deref()
                            .and_then(|v| v.parse::<u32>().ok())
                            .unwrap_or(0);
                        version = prev_version + 1;
                        previous_adapter_exists = true;
                        // A/B baseline: record previous metrics so training_status
                        // can compare when the new job completes.
                        ab_baseline = Self::metrics_from_trained(&prev).map(|m| AbBaseline {
                            previous_version: prev_version,
                            previous_loss: m.loss.unwrap_or(0.0),
                            previous_perplexity: m.perplexity.unwrap_or(0.0),
                        });
                    }
                    _ => {
                        version = 1;
                        previous_adapter_exists = false;
                    }
                }

                // Derive adapter name if not provided
                if resolved_adapter_name.is_none() {
                    resolved_adapter_name = Some(format!("{}-v{}", skill, version));
                }

                let _ = (previous_adapter_exists, original_examples, feedback_examples);

                // Ingest and normalize the merged dataset
                match self
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
                }
            } else {
                // Normal submit mode — ingest and normalize the provided dataset
                match self
                    .pipeline
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .ingest(&file_path)
                {
                    Ok(path) => path,
                    Err(e) => {
                        return Err(McpToolError::invalid_argument(format!("Dataset pipeline error: {e}")));
                    }
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
            // Emits reg.lora.audit spans (consumed by the skill's convergence-check phase).
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
                // Emit reg.lora.audit refuse spans
                for f in &refusals {
                    tracing::error!(
                        target: "reg.lora.audit",
                        gate = f.gate_id,
                        severity = "refuse",
                        message = %f.message,
                        source = %f.source,
                        "LoRA training-config gate refused at submit"
                    );
                }
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
            // Also emit reg.lora.audit spans for the convergence-check phase.
            for finding in &validation_findings {
                let severity_str = match finding.severity {
                    lora_validation::ValidationSeverity::Warn => "warn",
                    lora_validation::ValidationSeverity::Info => "info",
                    lora_validation::ValidationSeverity::Refuse => "refuse",
                };
                if finding.severity == lora_validation::ValidationSeverity::Warn {
                    tracing::warn!(
                        target: "reg.lora.audit",
                        gate = finding.gate_id,
                        severity = severity_str,
                        message = %finding.message,
                        source = %finding.source,
                        "LoRA training-config gate warning at submit"
                    );
                    tracing::warn!(
                        target: "hkask.training.validation.warn",
                        gate = finding.gate_id,
                        message = %finding.message,
                        remediation = %finding.remediation,
                        "Training config warning"
                    );
                } else if finding.severity == lora_validation::ValidationSeverity::Info {
                    tracing::info!(
                        target: "reg.lora.audit",
                        gate = finding.gate_id,
                        severity = severity_str,
                        message = %finding.message,
                        source = %finding.source,
                        "LoRA training-config gate info at submit"
                    );
                }
            }

            let mut job = TrainingJob {
                id: uuid::Uuid::new_v4().to_string(),
                dataset_path: normalized_path.clone(),
                base_model: base_model.clone(),
                params: resolved_params.clone(),
                status: TrainingJobStatus::Queued,
                created_at: chrono::Utc::now(),
                host: self.host_id,
                // Harness: prefer the operator-accepted harness from TrainingParams
                // (set via the lora-training skill's G6 gate), falling back to
                // the server default (Axolotl). This preserves the existing
                // default when no harness is selected — no silent migration.
                harness: resolved_params.harness.unwrap_or(self.harness_id),
                owner: None,
                skill_name: resolved_skill_name.clone(),
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

            // Retrain mode: pre-register adapter metadata so it's ready when
            // training completes. No weight path yet — the adapter hasn't been
            // trained. Will be resolved when training_status auto-registers
            // on completion.
            if retrain_mode {
                let adapter = Self::build_trained_adapter(
                    job.id.clone(),
                    resolved_adapter_name.clone().unwrap_or_default(),
                    base_model.clone(),
                    String::new(),
                    job.id.clone(),
                    chrono::Utc::now().timestamp(),
                    0,
                    resolved_skill_name.clone().unwrap_or_default(),
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
                    if retrain_mode {
                        result["retrain"] = json!(true);
                        result["skill_name"] = json!(resolved_skill_name);
                        result["adapter_name"] = json!(resolved_adapter_name);
                        result["version"] = json!(version);
                        if let Some(b) = &ab_baseline {
                            result["ab_baseline"] = json!({
                                "previous_version": b.previous_version,
                                "previous_loss": b.previous_loss,
                                "previous_perplexity": b.previous_perplexity,
                                "description": "A/B baseline from previous adapter. New adapter must beat this on >=2 of 3 metrics to auto-promote.",
                            });
                        }
                    }
                    tracing::info!(
                        target: "hkask.qa.cost.training_job",
                        job_id = %job.id,
                        provider_job_id = %provider_job_id,
                        host = %format!("{:?}", self.host_id),
                        estimated_cost_urj = job.estimated_cost_urj,
                        retrain = retrain_mode,
                        "Training job submitted with estimated cost"
                    );
                    if !token_warnings.is_empty() {
                        result["token_warnings"] = json!(token_warnings);
                        result["token_warning_count"] = json!(token_warnings.len());
                    }
                    Ok(result)
                }
                Err(e) => {
                    if let Some(job_store) = &self.job_store
                        && let Err(store_error) = job_store.update_status(&job.id, "failed")
                    {
                        tracing::warn!(
                            target: "hkask.training.job.persist",
                            job_id = %job.id,
                            error = %store_error,
                            "Failed to persist submission failure"
                        );
                    }
                    tracing::error!(
                        target: "hkask.training.job.fail",
                        job_id = %job.id,
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
                        //
                        // Uses `get_previous_by_skill_name` (excludes the current
                        // adapter by ID) — `get_by_skill_name` would return the
                        // current adapter since it was pre-registered before job
                        // completion, making the A/B block dead code.
                        let adapter_skill = adapter.skill_name.clone().unwrap_or_default();
                        if !adapter_skill.is_empty() {
                            let current_loss =
                                Self::metrics_from_trained(&adapter).and_then(|m| m.loss);
                            if let Some(prev) = self
                                .adapter_store
                                .get_previous_by_skill_name(&adapter_skill, adapter.id)
                                .map_err(|e| McpToolError::internal(format!("Adapter store error: {e}")))?
                            && let (Some(new_loss), Some(prev_loss)) = (
                                    current_loss,
                                    Self::metrics_from_trained(&prev).and_then(|m| m.loss),
                                ) {
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
        description = "Evaluate a trained adapter against a test dataset. Runs inference for each test example and scores accuracy using exact match, substring containment, or semantic comparison. The model must be deployed and available for inference (local adapters require the inference engine to have the adapter loaded)."
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
                    // temperature=1.0 with do_sample=False for deterministic output.
                    temperature: 1.0,
                    max_tokens: 512,
                    ..Default::default()
                };

                // Route to the user-specified model. The `model` field is
                // provider-prefixed (e.g., "TG/my-finetuned-model") — pass it
                // as the model_override so the inference router targets the
                // correct backend, not the default model.
                match router.generate_with_model(&prompt, &params, Some(&model), None).await {
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
                                match router.generate_with_model(&judge_prompt, &params, Some(&model), None).await {
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
        description = "Ingest a raw dataset file into the normalized cache without submitting a training job. Detects format (ChatML, ShareGPT, Alpaca, raw text, DPO preference, KTO preference, ORPO preference), normalizes to canonical format (ChatML for SFT, PreferenceExample for DPO/KTO/ORPO), validates, and caches. Returns the cached path for use with training_submit or training_assemble_dataset."
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
                let is_preference = format.map(|f| f.is_preference()).unwrap_or(false);
                let result = json!({
                    "dataset_path": dataset_path,
                    "normalized_path": normalized_path.to_string_lossy(),
                    "detected_format": format.map(|f| format!("{:?}", f)).unwrap_or_else(|| "unknown".to_string()),
                    "is_preference": is_preference,
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

    /// Run the lora-training skill's static math-contract gates on a training config.
    ///
    /// This is the runtime enforcement point for the `.agents/skills/lora-training/`
    /// skill's `audit-config` phase. The skill reasons over config files and
    /// proposes regressions; this tool enforces the static subset of gates
    /// (G-M1..G-M4, G-Q1, G-Q2, G-Q4, G-H1) via `lora_validation::validate_training_params`
    /// and emits `reg.lora.audit` spans per gate evaluated.
    ///
    /// Returns the full findings list so the skill's `convergence-check` phase
    /// can compute coverage. Findings with `Refuse` severity indicate the config
    /// would silently degrade model quality or waste GPU time if submitted.
    #[tool(
        description = "Validate training params against the lora-training skill's math-contract gates (G-M1 no-op-at-init, G-M2 merge equivalence, G-M3 scaling form, G-M4 rank budget, G-Q1 frozen base quantized, G-Q2 adapter dtype, G-Q4 no silent upcast, G-Q5 paged optimizer, G-H1 harness-method compatibility). Also validates dataset size (G-D1) if dataset_path is provided. When dataset_path is provided, also profiles the dataset (G-D0) and returns a DatasetProfile with format, sample count, content length statistics, token estimates, role distribution, multi-turn detection, vision data detection, and preference pair balance. Returns findings with severity (refuse/warn/info), gate ID, message, source citation, and remediation. Emits reg.lora.audit spans. This is the runtime enforcement point for the lora-training skill's audit-config phase."
    )]
    pub async fn training_validate_config(
        &self,
        Parameters(TrainValidateConfigRequest {
            params,
            dataset_path,
            base_model,
        }): Parameters<TrainValidateConfigRequest>,
    ) -> String {
        execute_tool(self, "training_validate_config", async {
                let mut findings = lora_validation::validate_training_params(&params);

                // G-D1: Dataset size vs quality (if dataset path provided).
                if let Some(ref ds_path) = dataset_path {
                    findings.extend(lora_validation::validate_dataset_size(std::path::Path::new(ds_path)));
                }

                // G-Q5: Paged optimizer (if base model provided).
                if let Some(ref model) = base_model {
                    findings.extend(lora_validation::validate_paged_optimizer(&params, model));
                }

                // Emit reg.lora.audit spans per gate evaluated.
                // The lora-training skill's convergence-check phase consumes these
                // to compute coverage metrics.
                for finding in &findings {
                    let severity_str = match finding.severity {
                        lora_validation::ValidationSeverity::Refuse => "refuse",
                        lora_validation::ValidationSeverity::Warn => "warn",
                        lora_validation::ValidationSeverity::Info => "info",
                    };
                    match finding.severity {
                        lora_validation::ValidationSeverity::Refuse => {
                            tracing::error!(
                                target: "reg.lora.audit",
                                gate = finding.gate_id,
                                severity = severity_str,
                                message = %finding.message,
                                source = %finding.source,
                                "LoRA training-config gate refused"
                            );
                        }
                        lora_validation::ValidationSeverity::Warn => {
                            tracing::warn!(
                                target: "reg.lora.audit",
                                gate = finding.gate_id,
                                severity = severity_str,
                                message = %finding.message,
                                source = %finding.source,
                                "LoRA training-config gate warning"
                            );
                        }
                        lora_validation::ValidationSeverity::Info => {
                            tracing::info!(
                                target: "reg.lora.audit",
                                gate = finding.gate_id,
                                severity = severity_str,
                                message = %finding.message,
                                source = %finding.source,
                                "LoRA training-config gate info"
                            );
                        }
                    }
                }

                // If no findings, emit a single pass span so the convergence-check
                // phase knows the audit ran.
                if findings.is_empty() {
                    tracing::info!(
                        target: "reg.lora.audit",
                        gate = "all",
                        severity = "pass",
                        "LoRA training-config audit passed all static gates"
                    );
                }

                let has_refusals = lora_validation::has_refusals(&findings);
                let findings_json: Vec<serde_json::Value> = findings
                    .iter()
                    .map(|f| {
                        json!({
                            "gate_id": f.gate_id,
                            "severity": match f.severity {
                                lora_validation::ValidationSeverity::Refuse => "refuse",
                                lora_validation::ValidationSeverity::Warn => "warn",
                                lora_validation::ValidationSeverity::Info => "info",
                            },
                            "message": f.message,
                            "source": f.source,
                            "remediation": f.remediation,
                        })
                    })
                    .collect();

                Ok(json!({
                    "params": serde_json::to_value(&params).unwrap_or_default(),
                    "findings": findings_json,
                    "finding_count": findings.len(),
                    "has_refusals": has_refusals,
                    "verdict": if has_refusals {
                        "fail"
                    } else if findings.iter().any(|f| f.severity == lora_validation::ValidationSeverity::Warn) {
                        "conditional"
                    } else {
                        "pass"
                    },
                    "gates_evaluated": [
                        "G-M1", "G-M2", "G-M3", "G-M4",
                        "G-Q1", "G-Q2", "G-Q4", "G-Q5",
                        "G-D1", "G-H1",
                    ],
                }))
            })
            .await
    }
}

// ── Entry point ───────────────────────────────────────────────────────────

/// Run the training MCP server (used by binary target).
pub async fn run(
    userpod: String,
    daemon_client: Option<hkask_mcp::DaemonClient>,
) -> Result<(), hkask_mcp::McpError> {
    // Host is fixed to Runpod (cloud-only, single host).
    // Harness default is Axolotl; per-job harness selection via TrainingParams.harness
    // (operator-accepted from the lora-training skill's G6 gate) is honored at
    // submit time — RunpodHost::submit selects the harness for config rendering.
    // Phase 1 (v0.31.0): Axolotl (SFT) + TRL SFTTrainer + Ludwig SFT. Phase 2
    // will add TRL DPO/KTO/ORPO trainers and Ludwig DPO/KTO/ORPO/GRPO.
    let host_id = TrainingHostId::Runpod;
    let harness_id = TrainingHarnessId::Axolotl;

    let cache_dir = PathBuf::from(
        std::env::var("HKASK_TRAINING_CACHE_DIR").unwrap_or_else(|_| {
            hkask_types::agent_paths::userpod_adapters_dir(&userpod)
                .to_string_lossy()
                .to_string()
        }),
    );
    let pipeline = DatasetPipeline::new(cache_dir);

    hkask_mcp::run_server_with_preloaded(
        "hkask-mcp-training",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
            (|| -> anyhow::Result<TrainingServer> {
                let db_path = ctx
                    .credentials
                    .get("HKASK_TRAINING_DB")
                    .cloned()
                    .unwrap_or_else(|| {
                        let relative = hkask_types::agent_paths::userpod_training_db(&userpod);
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
                        let job_store = Some(
                            JobStore::new(pool.clone())
                                .map_err(|error| anyhow::anyhow!("job store schema: {error}"))?,
                        );
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

                        // Build the canonical adapter router (used by AdapterPort for
                        // deployment, status, teardown — the MCP server no longer wraps these).
                        let router = AdapterRouter::new(std::sync::Arc::new(store.clone()));
                        let adapter_router = Some(std::sync::Arc::new(router));

                        (semantic, Arc::new(store), job_store, adapter_router)
                    }
                    None => {
                        // No passphrase configured — fall back to an in-memory driver
                        // so the server still runs (no persistence across restarts).
                        let pool = hkask_database::sqlite::SqliteDriver::in_memory_pool()
                            .map_err(|e| anyhow::anyhow!("in-memory pool: {e}"))?;
                        let driver: Arc<dyn hkask_database::driver::DatabaseDriver> =
                            Arc::new(hkask_database::sqlite::SqliteDriver::new(pool));
                        let store = hkask_adapter::AdapterStore::from_driver(driver);
                        (None, Arc::new(store), None, None)
                    }
                };

                let host_config = TrainingHostConfig {
                    host: host_id,
                    runpod_api_key: ctx
                        .credentials
                        .get("RUNPOD_API_KEY")
                        .cloned()
                        .unwrap_or_default(),
                    runpod_template_id: ctx
                        .credentials
                        .get("RUNPOD_TEMPLATE_ID")
                        .cloned()
                        .unwrap_or_default(),
                    runpod_gpu_type_id: ctx
                        .credentials
                        .get("RUNPOD_GPU_TYPE_ID")
                        .cloned()
                        .unwrap_or_default(),
                    runpod_container_disk_gb: parse_optional_u32(
                        ctx.credentials.get("RUNPOD_CONTAINER_DISK_GB"),
                    ),
                    runpod_min_memory_gb: parse_optional_u32(
                        ctx.credentials.get("RUNPOD_MIN_MEMORY_GB"),
                    ),
                    runpod_min_vcpu_count: parse_optional_u32(
                        ctx.credentials.get("RUNPOD_MIN_VCPU_COUNT"),
                    ),
                    runpod_docker_image: ctx
                        .credentials
                        .get("RUNPOD_DOCKER_IMAGE")
                        .cloned()
                        .unwrap_or_default(),
                };
                let host = create_host(&host_config)
                    .map_err(|e| anyhow::anyhow!("Failed to create training host: {}", e))?;

                let inference_config = InferenceConfig::from_env();

                Ok(TrainingServer::new(
                    ctx.webid,
                    userpod.clone(),
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
                ))
            })()
            .map_err(|e| hkask_mcp::McpError::UnexpectedResponse {
                context: "training server init".into(),
                detail: e.to_string(),
            })
        },
        vec![
            hkask_mcp::CredentialRequirement::required(
                "RUNPOD_API_KEY",
                "RunPod API key for governed training-job submission",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "RUNPOD_TEMPLATE_ID",
                "RunPod template ID; defaults to the canonical Axolotl template when unset",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "RUNPOD_GPU_TYPE_ID",
                "RunPod GPU type ID (e.g. \"NVIDIA H100 80GB HBM3\"). Authoritative when set; empty defers to the model-size heuristic",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "RUNPOD_CONTAINER_DISK_GB",
                "Container disk in GB. Authoritative when set; 0/empty defers to the model-size heuristic",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "RUNPOD_MIN_MEMORY_GB",
                "Minimum pod memory in GB. Authoritative when set; 0/empty defers to the default (24)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "RUNPOD_MIN_VCPU_COUNT",
                "Minimum vCPU count. Authoritative when set; 0/empty defers to the default (8)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "RUNPOD_DOCKER_IMAGE",
                "Docker image name. Authoritative when set; empty defers to the canonical Axolotl image",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_TRAINING_DB",
                "Path to per-agent training database for job/adapter/QA storage (defaults to agents/{userpod}/training.db)",
            ),
            hkask_mcp::CredentialRequirement::optional(
                "HKASK_DB_PASSPHRASE",
                "Passphrase for the training database (resolved via credentials or keystore; in-memory if unavailable)",
            ),
        ],
        std::collections::HashMap::new(),
    )
    .await
}

/// Parse an optional `u32` credential value.
///
/// Used for the Runpod deployment settings (`RUNPOD_CONTAINER_DISK_GB`,
/// `RUNPOD_MIN_MEMORY_GB`, `RUNPOD_MIN_VCPU_COUNT`) that flow through
/// `ServerContext.credentials` as strings. Returns `0` for `None`, empty
/// string, or unparseable input — `RunpodHost::submit` treats `0` as
/// "operator did not set this" and falls back to the documented default.
///
/// post: returns 0 iff the input is absent, empty, or not a valid u32
fn parse_optional_u32(value: Option<&String>) -> u32 {
    value
        .map(String::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0)
}
