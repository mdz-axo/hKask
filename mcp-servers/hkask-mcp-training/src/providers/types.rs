//! Training provider types — enums, params, traits, and error types.
//!
//! This module contains the foundational types for the training provider system:
//! - Host/harness identifiers
//! - Training job representation
//! - Hyperparameter types
//! - The `TrainingHost` trait
//! - Error and metadata types
//! - Cost estimation

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

// ── Training harness identifiers ─────────────────────────────────────────────

/// Training harnesses — the tooling that runs on top of a host.
///
/// This is the *harness* layer (Axolotl/Unsloth tooling), distinct from the
/// *host* layer (where compute runs: Together/Runpod/Baseten) and the
/// *base model* layer (what model is fine-tuned: Qwen/Gemma/Mistral).
///
/// Each variant represents a training framework that produces LoRA adapters.
/// All training runs on cloud hosts — there is no local training path.
///
/// Harness → Host mapping:
///   Axolotl → Together AI, Runpod
///   Unsloth → Baseten
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingHarnessId {
    /// axolotl — YAML-based training framework, dispatched to Together AI or Runpod
    Axolotl,
    /// unsloth — memory-efficient Python training framework, dispatched to Baseten
    Unsloth,
}

impl TrainingHarnessId {
    /// Parse from a config string (case-insensitive).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "axolotl" => Some(Self::Axolotl),
            "unsloth" => Some(Self::Unsloth),
            _ => None,
        }
    }
}

// ── Host identifiers ─────────────────────────────────────────────────────────

/// Training hosts — where the GPU compute runs.
///
/// This is the *host* layer (cloud only — no local training), distinct from the
/// *harness* layer (Axolotl/Unsloth tooling) and the *base model* layer
/// (Qwen/Gemma/Mistral/etc.). Each variant represents a cloud backend that
/// executes training jobs.
///
/// Host → Harness mapping:
///   Together → Axolotl
///   Runpod   → Axolotl
///   Baseten  → Unsloth
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingHostId {
    /// together — Together AI cloud fine-tuning API (Axolotl harness)
    Together,
    /// runpod — Runpod GPU cloud training, pod-based axolotl dispatch
    Runpod,
    /// baseten — Baseten managed training infrastructure, Unsloth harness
    Baseten,
}

impl TrainingHostId {
    /// Parse from a config string (case-insensitive).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "together" => Some(Self::Together),
            "runpod" => Some(Self::Runpod),
            "baseten" => Some(Self::Baseten),
            _ => None,
        }
    }
}

// ── Job/Adapter types ──────────────────────────────────────────────────────

/// The canonical representation of a training job, provider-agnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingJob {
    /// Unique job identifier (UUIDv4).
    pub id: String,
    /// Path to the preprocessed dataset file.
    pub dataset_path: PathBuf,
    /// Base model identifier (provider-prefixed, e.g., "OM/qwen3:8b").
    pub base_model: String,
    /// Training hyperparameters.
    pub params: TrainingParams,
    /// Current job status.
    pub status: TrainingJobStatus,
    /// Job creation timestamp.
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Host executing this job.
    pub host: TrainingHostId,
    /// Harness (training framework) to use.
    pub harness: TrainingHarnessId,
    /// User/owner WebID for provenance.
    #[serde(default)]
    pub owner: Option<String>,
    /// Skill name for retraining — when set, enables A/B comparison on completion.
    /// Semantic/generic fine-tuning leaves this `None`.
    #[serde(default)]
    pub skill_name: Option<String>,
    /// Estimated cost of this training job in micro-rJoules (µrJ).
    /// Computed from host provider pricing + training epochs.
    /// Not optional — always computed at job creation time.
    #[serde(default)]
    pub estimated_cost_urj: u64,
}

impl TrainingJob {
    pub fn new(
        dataset_path: PathBuf,
        base_model: String,
        params: TrainingParams,
        host: TrainingHostId,
        harness: TrainingHarnessId,
    ) -> Self {
        let cost = estimate_training_cost_urj(&host, params.num_epochs, &base_model);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            dataset_path,
            base_model,
            params,
            status: TrainingJobStatus::Queued,
            created_at: chrono::Utc::now(),
            host,
            harness,
            owner: None,
            skill_name: None,
            estimated_cost_urj: cost,
        }
    }
}

/// Estimate training cost in micro-rJoules (µrJ) from host provider pricing.
///
/// Uses host-specific base rates multiplied by epoch count and model size.
pub(crate) fn estimate_training_cost_urj(
    host: &TrainingHostId,
    num_epochs: u32,
    base_model: &str,
) -> u64 {
    let base_per_epoch: u64 = match host {
        TrainingHostId::Together => 1_000_000, // ~$1.00/epoch
        TrainingHostId::Runpod => 500_000,     // ~$0.50/epoch
        TrainingHostId::Baseten => 500_000,    // ~$0.50/epoch
    };
    let size_mult = extract_model_size_multiplier(base_model);
    base_per_epoch * (num_epochs as u64) * size_mult
}

/// Extract size multiplier from model identifier string.
pub(crate) fn extract_model_size_multiplier(base_model: &str) -> u64 {
    let lower = base_model.to_lowercase();
    for (pattern, mult) in &[
        ("8b", 1),
        ("9b", 1),
        ("7b", 1),
        ("3b", 1),
        ("1b", 1),
        ("13b", 2),
        ("14b", 2),
        ("20b", 2),
        ("26b", 2),
        ("30b", 2),
        ("70b", 4),
        ("72b", 4),
        ("120b", 4),
        ("405b", 4),
        ("8x7b", 2),
        ("8x22b", 4),
    ] {
        if lower.contains(pattern) {
            return *mult;
        }
    }
    2 // Default: assume medium size
}

/// LoRA-specific training parameters.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoraParams {
    /// LoRA rank (r value). Typical range: 4–64.
    pub r: u32,
    /// LoRA alpha scaling factor.
    pub alpha: u32,
    /// LoRA dropout rate. 0.0 is optimized by Unsloth.
    #[serde(default)]
    pub dropout: f32,
    /// Target modules for LoRA adaptation.
    pub target_modules: Vec<String>,
    /// Additional modules to save fully (e.g., ["embed_tokens", "lm_head"]).
    #[serde(default)]
    pub modules_to_save: Vec<String>,
    /// Use Rank-Stabilized LoRA (scales by alpha/sqrt(r)).
    #[serde(default)]
    pub use_rslora: bool,
}

impl Default for LoraParams {
    fn default() -> Self {
        Self {
            r: 16,
            alpha: 32,
            dropout: 0.0,
            target_modules: vec![
                "q_proj".to_string(),
                "v_proj".to_string(),
                "k_proj".to_string(),
                "o_proj".to_string(),
            ],
            modules_to_save: vec![],
            use_rslora: false,
        }
    }
}

/// Quantization parameters for QLoRA training.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct QuantizationParams {
    /// Load base model in 4-bit (QLoRA).
    #[serde(default)]
    pub load_in_4bit: bool,
    /// Load base model in 8-bit.
    #[serde(default)]
    pub load_in_8bit: bool,
    /// 4-bit compute dtype ("bf16", "fp16", "fp32").
    #[serde(default)]
    pub bnb_4bit_compute_dtype: Option<String>,
    /// 4-bit quantization type ("nf4", "fp4").
    #[serde(default)]
    pub bnb_4bit_quant_type: Option<String>,
    /// Use double quantization for additional memory savings.
    #[serde(default)]
    pub bnb_4bit_use_double_quant: bool,
}

/// Optimization and scheduler parameters.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OptimizationParams {
    /// Optimizer name ("adamw_torch", "adamw_8bit", "adamw_bnb_8bit", "paged_adamw_32bit").
    #[serde(default)]
    pub optimizer: Option<String>,
    /// Weight decay (excludes bias and LayerNorm).
    #[serde(default)]
    pub weight_decay: f32,
    /// Warmup steps (mutually exclusive with warmup_ratio).
    #[serde(default)]
    pub warmup_steps: Option<u32>,
    /// Warmup ratio (fraction of total steps).
    #[serde(default)]
    pub warmup_ratio: Option<f32>,
    /// LR scheduler type ("cosine", "linear", "constant", "constant_with_warmup").
    #[serde(default)]
    pub lr_scheduler: Option<String>,
    /// Gradient accumulation steps (effective batch = batch_size × grad_accum × n_gpu).
    #[serde(default)]
    pub gradient_accumulation_steps: u32,
    /// Cosine minimum LR ratio (e.g., 0.1 for 10% of peak LR).
    #[serde(default)]
    pub cosine_min_lr_ratio: Option<f32>,
    /// Adam beta1.
    #[serde(default)]
    pub adam_beta1: Option<f32>,
    /// Adam beta2.
    #[serde(default)]
    pub adam_beta2: Option<f32>,
    /// Adam epsilon.
    #[serde(default)]
    pub adam_epsilon: Option<f32>,
    /// Max gradient norm for clipping.
    #[serde(default)]
    pub max_grad_norm: Option<f32>,
}

impl Default for OptimizationParams {
    fn default() -> Self {
        Self {
            optimizer: None,
            weight_decay: 0.0,
            warmup_steps: None,
            warmup_ratio: None,
            lr_scheduler: None,
            gradient_accumulation_steps: 1,
            cosine_min_lr_ratio: None,
            adam_beta1: None,
            adam_beta2: None,
            adam_epsilon: None,
            max_grad_norm: None,
        }
    }
}

/// Sequence and packing parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct SequenceParams {
    /// Maximum sequence length for training.
    #[serde(default)]
    pub sequence_len: Option<u32>,
    /// Enable sample packing (block-diagonal attention for short sequences).
    #[serde(default)]
    pub sample_packing: bool,
    /// Pad to sequence length (reduces memory fragmentation).
    #[serde(default)]
    pub pad_to_sequence_len: bool,
    /// Enable NEFTune noise (paper default: 5.0).
    #[serde(default)]
    pub neftune_noise_alpha: Option<f32>,
}

/// Advanced attention and memory parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct AdvancedParams {
    /// Attention implementation ("flash_attention_2", "sdpa", "eager").
    #[serde(default)]
    pub attn_implementation: Option<String>,
    /// Gradient checkpointing ("unsloth" for Unsloth-optimized, "true" for standard).
    #[serde(default)]
    pub gradient_checkpointing: Option<String>,
    /// Use bf16 mixed precision.
    #[serde(default)]
    pub bf16: bool,
    /// Use fp16 mixed precision.
    #[serde(default)]
    pub fp16: bool,
    /// Evaluation split ratio (fraction of dataset held out for eval).
    #[serde(default)]
    pub eval_split_ratio: Option<f32>,
}

/// Canonical training hyperparameters — the union of capabilities supported by
/// at least two harnesses or required by a specific training mode.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrainingParams {
    /// Number of training epochs.
    pub num_epochs: u32,
    /// Per-device batch size.
    pub batch_size: u32,
    /// Learning rate.
    pub learning_rate: f32,
    /// LoRA-specific parameters.
    #[serde(default)]
    pub lora: LoraParams,
    /// Quantization parameters (QLoRA).
    #[serde(default)]
    pub quantization: QuantizationParams,
    /// Optimizer, scheduler, and gradient parameters.
    #[serde(default)]
    pub optimization: OptimizationParams,
    /// Sequence length and packing parameters.
    #[serde(default)]
    pub sequence: SequenceParams,
    /// Attention, mixed precision, and eval parameters.
    #[serde(default)]
    pub advanced: AdvancedParams,
}

impl Default for TrainingParams {
    fn default() -> Self {
        Self {
            num_epochs: 3,
            batch_size: 4,
            learning_rate: 2e-4,
            lora: LoraParams::default(),
            quantization: QuantizationParams::default(),
            optimization: OptimizationParams::default(),
            sequence: SequenceParams::default(),
            advanced: AdvancedParams::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

// ── Provider error ─────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Provider '{0}' is not available (missing CLI or configuration)")]
    Unavailable(String),
    #[error("Training job failed: {0}")]
    JobFailed(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Dataset error: {0}")]
    DatasetError(String),
    #[error("Provider backend error: {0}")]
    Backend(String),
}

// ── TrainingHost trait ────────────────────────────────────────────────────

/// Pluggable training host — where a training job runs.
///
/// This is the *host* layer (cloud or local GPU), distinct from the *harness* layer
/// (Axolotl/Unsloth tooling) and the *base model* layer (Qwen/Gemma/Mistral).
/// Each implementation talks to a specific compute backend.
///
/// Implementations translate canonical `TrainingJob` representations into
/// host-specific API calls (CLI execution, remote HTTP dispatch, etc.).
/// The trait is async to accommodate both local subprocess management and
/// cloud provider HTTP calls via `hkask-inference` routing.
#[async_trait::async_trait]
pub trait TrainingHost: Send + Sync {
    /// Submit a training job for execution.
    /// Returns a provider-specific job ID for status tracking.
    async fn submit(&self, job: &TrainingJob) -> Result<String, ProviderError>;

    /// Query the status of a previously submitted job.
    async fn status(&self, job_id: &str) -> Result<TrainingJobStatus, ProviderError>;

    /// Cancel a running or queued job.
    async fn cancel(&self, job_id: &str) -> Result<(), ProviderError>;

    /// List all LoRA adapters produced by this provider.
    async fn list_adapters(&self) -> Result<Vec<String>, ProviderError>;

    /// Delete a LoRA adapter and its associated artifacts.
    async fn delete_adapter(&self, adapter_id: &str) -> Result<(), ProviderError>;

    /// Return completion metadata for a finished job (base model, metrics, output path).
    /// Returns `None` if the job is not completed or the provider doesn't support metadata.
    /// Default implementation returns `None`.
    async fn completion_metadata(
        &self,
        _job_id: &str,
    ) -> Result<Option<CompletionMetadata>, ProviderError> {
        Ok(None)
    }

    /// Return the filesystem path to adapter weights, if stored locally.
    /// Returns `None` for cloud hosts (Together AI) where weights are server-side.
    /// Default implementation returns `None`.
    async fn adapter_weight_path(
        &self,
        _adapter_id: &str,
    ) -> Result<Option<PathBuf>, ProviderError> {
        Ok(None)
    }

    /// Download adapter weights from cloud host to a local cache path.
    ///
    /// Returns the local path after download, or `None` if the host doesn't
    /// support weight download (e.g., weights are on HuggingFace, not the host's storage).
    ///
    /// Default implementation returns `None` — local hosts already have weights
    /// on disk, and cloud hosts return `Some(local_path)` after download.
    /// The default is `None` because most cloud hosts store weights on
    /// third-party services (HuggingFace, S3), not the host's own storage.
    async fn download_adapter(
        &self,
        _adapter_id: &str,
        _cache_dir: &std::path::Path,
    ) -> Result<Option<PathBuf>, ProviderError> {
        Ok(None)
    }

    /// Estimate the cost of a training job before execution.
    ///
    /// Used by `training_recommend_model` to surface cost before committing to a job.
    /// Cloud hosts return GPU-hour pricing; local hosts return 0.0
    /// (cost is the user's own hardware).
    ///
    /// Default implementation returns a zero estimate.
    async fn estimate_cost(&self, _job: &TrainingJob) -> CostEstimate {
        CostEstimate::default()
    }
}

/// Metadata returned by a provider when a training job completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionMetadata {
    /// Base model used for training.
    pub base_model: String,
    /// Fine-tuned model name / output identifier.
    pub output_name: Option<String>,
    /// Final training loss.
    pub loss: Option<f32>,
    /// Training duration in seconds.
    pub training_duration_secs: Option<u64>,
    /// Number of tokens processed.
    pub tokens_processed: Option<u64>,
}

// ── Cost estimation ────────────────────────────────────────────────────────

/// Estimated training cost broken down by resource.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CostEstimate {
    /// GPU hours consumed (cloud = billed, local = 0.0).
    pub gpu_hours: f32,
    /// Tokens processed during training.
    pub total_tokens: u64,
    /// Estimated cost in USD (cloud hosts) or 0.0 (local).
    pub estimated_dollars: f32,
}

impl Default for CostEstimate {
    fn default() -> Self {
        Self {
            gpu_hours: 0.0,
            total_tokens: 0,
            estimated_dollars: 0.0,
        }
    }
}
