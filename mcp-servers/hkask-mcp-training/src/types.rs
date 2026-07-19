//! Request types for the Training MCP server — all tool input structs and their supporting types.
//! Fourteen tools (simplified from 21 on 2026-07-19): ingest_qa, submit, status,
//! cancel, delete_adapter, assemble_dataset, evaluate, register_adapter, retrain,
//! ingest_dataset, preflight_check, deploy, deployment_status, teardown.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use chrono;
use hkask_adapter::{EndpointLifecycle, EndpointPhase};

use crate::providers::TrainingParams;
use hkask_inference::ProviderId;

// ── Deployment provider ─────────────────────────────────────────────────

/// Cloud provider for adapter deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentProvider {
    /// Together AI — fine-tuned models auto-deployed. ~30s setup.
    Together,
    /// Runpod — GPU pod with adapter weights mounted. ~3min setup.
    Runpod,
}

impl DeploymentProvider {
    /// Map to `hkask_inference::ProviderId` for use with `hkask-adapter::AdapterRouter`.
    pub fn as_provider_id(&self) -> ProviderId {
        match self {
            DeploymentProvider::Together => ProviderId::Together,
            DeploymentProvider::Runpod => ProviderId::Runpod,
        }
    }

    /// Estimated setup time in seconds.
    pub fn setup_seconds(&self) -> u64 {
        match self {
            DeploymentProvider::Together => 30,
            DeploymentProvider::Runpod => 300,
        }
    }

    /// Estimated cost per hour in USD.
    pub fn cost_per_hour(&self, gpu: Option<&str>) -> f32 {
        match self {
            DeploymentProvider::Together => 0.0,
            DeploymentProvider::Runpod => match gpu.unwrap_or("RTX 4090") {
                "A100" => 1.99,
                "H100" => 2.99,
                _ => 0.79,
            },
        }
    }
}
#[derive(Debug, Deserialize, JsonSchema)]
pub struct QaItem {
    pub question: String,
    pub answer: String,
    #[serde(default)]
    pub bloom_level: Option<String>,
}

// ── Request structs ──────────────────────────────────────────────────────

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
    /// Human-readable name for the adapter (e.g., "pragmatic-semantics-v1").
    pub name: String,
    /// Skill name this adapter serves (e.g., "pragmatic-semantics").
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
pub struct TrainRetrainRequest {
    /// Path to the original training dataset.
    pub original_dataset_path: String,
    /// Path to the feedback JSONL file (from training_curate_feedback).
    pub feedback_path: String,
    /// Base model to fine-tune (provider-prefixed).
    pub base_model: String,
    /// Adapter name for the new version (e.g., "pragmatic-semantics-v2").
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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainDeployRequest {
    /// Adapter ID or skill/expertise name to deploy (e.g., "pragmatic-semantics-v1").
    pub adapter_name: String,
    /// Cloud inference provider for deployment.
    pub provider: DeploymentProvider,
    /// Base model the adapter was trained on. Auto-resolved from AdapterStore if omitted.
    #[serde(default)]
    pub base_model: Option<String>,
    /// GPU type preference (e.g., "A100", "H100", "RTX 4090"). Provider-specific.
    #[serde(default)]
    pub gpu_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainTeardownRequest {
    /// Deployment ID from a previous training_deploy call.
    pub deployment_id: String,
}
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

// ── Pre-flight check ───────────────────────────────────────────────────────

/// Request for `training_preflight_check` — verifies an adapter is safe to
/// deploy before running a full eval or registering it as ready.
///
/// Three checks run sequentially, fail-fast:
/// 1. **load** — adapter_config.json parses and init_lora_weights is valid
/// 2. **weights** — adapter_model.safetensors exists and is non-empty
/// 3. **sanity** — a test prompt produces output > 50 chars (optional, requires inference)
///
/// This tool prevents the class of failures where a PiSSA-trained adapter is
/// loaded with `init_lora_weights: true` without conversion, causing the
/// principal components to be double-counted and the model to regress.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TrainPreflightCheckRequest {
    /// Path to the adapter directory containing adapter_config.json and
    /// adapter_model.safetensors.
    pub adapter_path: String,
    /// Model identifier for the sanity-check inference call (provider-prefixed).
    /// If omitted, only the load and weights checks run (no inference).
    #[serde(default)]
    pub model: Option<String>,
    /// Test prompt for the sanity check. If omitted, a default Rust prompt is used.
    #[serde(default)]
    pub test_prompt: Option<String>,
    /// Minimum acceptable response length for the sanity check (default: 50).
    #[serde(default)]
    pub min_response_chars: Option<usize>,
}
