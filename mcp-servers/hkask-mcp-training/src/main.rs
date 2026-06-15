//! hKask MCP Training — Model training data ingestion and fine-tuning server.
//!
//! Starts an MCP server over stdio exposing 8 tools:
//! - `training_ingest_qa` — Ingest QA pairs for future model fine-tuning
//! - `training_submit` — Submit a training job for execution
//! - `training_status` — Query training job status
//! - `training_cancel` — Cancel a running or queued job
//! - `training_list_adapters` — List completed LoRA adapters
//! - `training_delete_adapter` — Remove a LoRA adapter
//! - `training_assemble_dataset` — Assemble stored QA pairs into a ChatML JSONL dataset file
//! - `training_generate_traces` — Generate decomposition traces from skill documents
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

use hkask_inference::{InferenceConfig, InferenceRouter};
use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_mcp::validate_field;
use hkask_mcp_training::adapters::{AdapterStore, InMemoryAdapterStore};
use hkask_mcp_training::dataset::DatasetPipeline;
use hkask_mcp_training::providers::{
    ProviderConfig, TrainingJob, TrainingJobStatus, TrainingParams, TrainingProvider,
    TrainingProviderId, create_provider,
};
use hkask_memory::SemanticMemory;
use hkask_storage::Triple;
use hkask_types::ports::InferencePort;
use hkask_types::{LLMParameters, McpErrorKind, Visibility, WebID};
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

// ── Server ───────────────────────────────────────────────────────────────

pub struct TrainingServer {
    webid: WebID,
    replicant: String,
    daemon: Option<hkask_mcp::DaemonClient>,
    semantic: Option<SemanticMemory>,
    provider: Box<dyn TrainingProvider>,
    provider_id: TrainingProviderId,
    pipeline: Mutex<DatasetPipeline>,
    adapter_store: Arc<dyn AdapterStore>,
    inference_config: InferenceConfig,
}

impl TrainingServer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        webid: WebID,
        replicant: String,
        daemon: Option<hkask_mcp::DaemonClient>,
        semantic: Option<SemanticMemory>,
        provider: Box<dyn TrainingProvider>,
        provider_id: TrainingProviderId,
        pipeline: DatasetPipeline,
        adapter_store: Arc<dyn AdapterStore>,
        inference_config: InferenceConfig,
    ) -> Self {
        Self {
            webid,
            replicant,
            daemon,
            semantic,
            provider,
            provider_id,
            pipeline: Mutex::new(pipeline),
            adapter_store,
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
                let mut metadata_list: Vec<serde_json::Value> = Vec::new();
                for id in &adapter_ids {
                    let entry = match self.adapter_store.get_metadata(id).await {
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

            conversations.push(json!({
                "messages": [
                    {"role": "user", "content": question},
                    {"role": "assistant", "content": answer}
                ]
            }));
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
            model: _model,
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

        let prompt = format!(
            "You are generating training data for fine-tuning an AI agent on the '{skill_name}' skill.\n\n\
             SKILL DOCUMENT:\n{skill_text}\n\n\
             Generate {count} training examples in ChatML JSONL format. \
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

        let router = InferenceRouter::new(self.inference_config.clone());
        let params = LLMParameters {
            temperature: 0.7,
            max_tokens: 4096,
            ..Default::default()
        };

        match router.generate(&prompt, &params).await {
            Ok(response) => {
                // Strip markdown code fences if present
                let cleaned = response
                    .text
                    .trim()
                    .trim_start_matches("```jsonl")
                    .trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();

                // Validate each line is parseable JSON with messages array
                let mut valid_count = 0;
                let mut parse_errors = 0;
                for (i, line) in cleaned.lines().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<serde_json::Value>(trimmed) {
                        Ok(v) if v.get("messages").is_some() => {
                            valid_count += 1;
                        }
                        Ok(_) => {
                            parse_errors += 1;
                            tracing::warn!(
                                target: "cns.training.trace",
                                line = i + 1,
                                "Trace missing 'messages' field"
                            );
                        }
                        Err(e) => {
                            parse_errors += 1;
                            tracing::warn!(
                                target: "cns.training.trace",
                                line = i + 1,
                                error = %e,
                                "Failed to parse trace line"
                            );
                        }
                    }
                }

                if valid_count == 0 {
                    return span.error(
                        McpErrorKind::Internal,
                        McpToolError::internal(
                            "Inference returned no valid ChatML traces. The model may not have understood the format.",
                        )
                        .to_json_string(),
                    );
                }

                // Write valid traces to output file
                match std::fs::write(&output_path, cleaned) {
                    Ok(()) => {
                        let result = json!({
                            "skill_name": skill_name,
                            "traces_requested": count,
                            "traces_generated": valid_count,
                            "parse_errors": parse_errors,
                            "output_path": output_path,
                            "tokens_used": response.usage.total_tokens,
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
            Err(e) => span.error(
                McpErrorKind::Unavailable,
                McpToolError::unavailable(format!("Inference failed: {}", e)).to_json_string(),
            ),
        }
    }

    fn provider_id(&self) -> TrainingProviderId {
        self.provider_id
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
        together_api_key: std::env::var("TOGETHER_API_KEY").unwrap_or_default(),
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

            let inference_config = InferenceConfig::from_env();

            Ok(TrainingServer::new(
                ctx.webid,
                replicant.clone(),
                daemon_client.clone(),
                semantic,
                provider,
                provider_config.provider,
                pipeline.clone(),
                Arc::clone(&adapter_store),
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
