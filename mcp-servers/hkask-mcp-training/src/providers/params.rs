//! Training provider parameters and types.
//!
//! Canonical representation of training jobs, hyperparameters,
//! and provider-agnostic types used across all host/harness backends.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

// ── Training harness identifiers ─────────────────────────────────────────────

/// Training harnesses — the tooling that runs on top of a host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingHarnessId {
    Axolotl,
    Unsloth,
}

impl TrainingHarnessId {
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

/// Training hosts — where the GPU compute runs (cloud only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingHostId {
    Together,
    Runpod,
    Baseten,
}

impl TrainingHostId {
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

// ── Job types ────────────────────────────────────────────────────────────────

/// The canonical representation of a training job, provider-agnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingJob {
    pub id: String,
    pub dataset_path: PathBuf,
    pub base_model: String,
    pub params: TrainingParams,
    pub status: TrainingJobStatus,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub host: TrainingHostId,
    pub harness: TrainingHarnessId,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub skill_name: Option<String>,
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
pub(crate) fn estimate_training_cost_urj(
    host: &TrainingHostId,
    num_epochs: u32,
    base_model: &str,
) -> u64 {
    let base_per_epoch: u64 = match host {
        TrainingHostId::Together => 1_000_000,
        TrainingHostId::Runpod => 500_000,
        TrainingHostId::Baseten => 500_000,
    };
    let size_mult = extract_model_size_multiplier(base_model);
    base_per_epoch * (num_epochs as u64) * size_mult
}

/// Extract size multiplier from model identifier string.
fn extract_model_size_multiplier(base_model: &str) -> u64 {
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
    2
}

// ── LoRA parameters ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoraParams {
    pub r: u32,
    pub alpha: u32,
    #[serde(default)]
    pub dropout: f32,
    pub target_modules: Vec<String>,
    #[serde(default)]
    pub modules_to_save: Vec<String>,
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

// ── Quantization parameters ──────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct QuantizationParams {
    #[serde(default)]
    pub load_in_4bit: bool,
    #[serde(default)]
    pub load_in_8bit: bool,
    #[serde(default)]
    pub bnb_4bit_compute_dtype: Option<String>,
    #[serde(default)]
    pub bnb_4bit_quant_type: Option<String>,
    #[serde(default)]
    pub use_double_quant: bool,
}

// ── Optimization parameters ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OptimizationParams {
    #[serde(default)]
    pub flash_attention: bool,
    #[serde(default)]
    pub gradient_checkpointing: bool,
    #[serde(default)]
    pub neftune_noise_alpha: Option<f32>,
    #[serde(default)]
    pub sample_packing: bool,
    #[serde(default)]
    pub deepspeed_stage: Option<u8>,
    #[serde(default)]
    pub fsdp: bool,
    #[serde(default)]
    pub fp8_mixed: bool,
}

impl Default for OptimizationParams {
    fn default() -> Self {
        Self {
            flash_attention: true,
            gradient_checkpointing: true,
            neftune_noise_alpha: None,
            sample_packing: false,
            deepspeed_stage: None,
            fsdp: false,
            fp8_mixed: false,
        }
    }
}

// ── Sequence parameters ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SequenceParams {
    pub max_seq_length: u32,
    #[serde(default)]
    pub packing: bool,
}

impl Default for SequenceParams {
    fn default() -> Self {
        Self {
            max_seq_length: 2048,
            packing: false,
        }
    }
}

// ── Advanced parameters ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct AdvancedParams {
    #[serde(default)]
    pub loraplus_lr_ratio: Option<f32>,
    #[serde(default)]
    pub use_galore: bool,
    #[serde(default)]
    pub adapter_save_steps: Option<u32>,
}

// ── Training parameters (composite) ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrainingParams {
    pub num_epochs: u32,
    pub batch_size: u32,
    #[serde(default)]
    pub learning_rate: f32,
    pub lora: LoraParams,
    #[serde(default)]
    pub quantization: QuantizationParams,
    #[serde(default)]
    pub optimization: OptimizationParams,
    #[serde(default)]
    pub sequence: SequenceParams,
    #[serde(default)]
    pub advanced: AdvancedParams,
    #[serde(default)]
    pub warmup_steps: u32,
    #[serde(default)]
    pub weight_decay: f32,
    #[serde(default)]
    pub max_steps: Option<u32>,
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
            warmup_steps: 10,
            weight_decay: 0.0,
            max_steps: None,
        }
    }
}

// ── Training job status ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrainingJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

// ── Provider error ───────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider unavailable: {0}")]
    Unavailable(String),
    #[error("backend error: {0}")]
    Backend(String),
    #[error("invalid config: {0}")]
    InvalidConfig(String),
    #[error("job not found: {0}")]
    JobNotFound(String),
    #[error("cancellation failed: {0}")]
    CancelFailed(String),
}

// ── TrainingHost trait ───────────────────────────────────────────────────────

/// A pluggable training backend (cloud or local).
#[async_trait::async_trait]
pub trait TrainingHost: Send + Sync {
    async fn submit(&self, job: &TrainingJob) -> Result<String, ProviderError>;
    async fn status(&self, job_id: &str) -> Result<TrainingJobStatus, ProviderError>;
    async fn cancel(&self, job_id: &str) -> Result<(), ProviderError>;
}

// ── Completion metadata ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompletionMetadata {
    pub job_id: String,
    pub adapter_name: Option<String>,
    pub output_dir: PathBuf,
    pub training_loss: Option<f32>,
    pub tokens_processed: Option<u64>,
    pub elapsed_seconds: Option<f64>,
}
