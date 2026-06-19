//! Training provider abstraction — pluggable backend adapter for model fine-tuning.
//!
//! Each provider wraps a different training framework (axolotl, unsloth) behind
//! a common `TrainingProvider` trait. The server maps its tool surface (`submit`,
//! `status`, `cancel`, `list_adapters`, `delete_adapter`) to provider methods,
//! isolating the MCP surface from framework-specific API differences.
//!
//! Provider selection is driven by `training.provider` in settings.json, routed
//! through `hkask-services` shared config init.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use thiserror::Error;

// ── Training harness identifiers ─────────────────────────────────────────────

/// Training harnesses — the tooling that runs on top of a host.
///
/// This is the *harness* layer (Axolotl/Unsloth tooling), distinct from the
/// *host* layer (where compute runs: Together/Runpod/Baseten) and the
/// *base model* layer (what model is fine-tuned: Qwen/Gemma/Mistral).
///
/// Each variant represents a training framework that produces LoRA adapters.
/// A harness can run on any host (local GPU or cloud), depending on configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingHarnessId {
    /// axolotl — YAML-based training framework, runs locally via CLI or dispatched to Runpod
    Axolotl,
    /// unsloth — memory-efficient Python training framework, runs locally via CLI or dispatched to Baseten
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
/// This is the *host* layer (cloud or local), distinct from the *harness* layer
/// (Axolotl/Unsloth tooling) and the *base model* layer (Qwen/Gemma/Mistral/etc.).
/// Each variant represents a backend that can execute training jobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingHostId {
    /// together — Together AI cloud fine-tuning API
    Together,
    /// runpod — Runpod GPU cloud training (pod-based axolotl dispatch)
    Runpod,
    /// baseten — Baseten managed training infrastructure (bring your own train.py)
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
}

impl TrainingJob {
    pub fn new(
        dataset_path: PathBuf,
        base_model: String,
        params: TrainingParams,
        host: TrainingHostId,
        harness: TrainingHarnessId,
    ) -> Self {
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
        }
    }
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

// ── Axolotl provider ───────────────────────────────────────────────────────

/// Axolotl training provider — wraps the axolotl CLI for local/remote training.
///
/// Axolotl uses YAML configuration files with explicit model/dataset/lora
/// sections. This provider translates canonical `TrainingJob` into an axolotl
/// config via the bound harness, writes it to a temp file, and dispatches
/// execution via local subprocess.
pub struct AxolotlProvider {
    /// Path to axolotl CLI binary or `accelerate launch` wrapper.
    cli_path: Option<PathBuf>,
    /// Harness for rendering axolotl YAML config.
    harness: Box<dyn HarnessAdapter>,
    /// Running job PIDs for cancellation support.
    jobs: Arc<Mutex<HashMap<String, u32>>>,
}

impl AxolotlProvider {
    /// Create a new axolotl provider with the given harness.
    ///
    /// If `cli_path` is `None`, the provider will attempt to find `axolotl`
    /// on PATH.
    pub fn new(cli_path: Option<PathBuf>, harness: Box<dyn HarnessAdapter>) -> Self {
        Self {
            cli_path,
            harness,
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check whether axolotl is available locally.
    fn available(&self) -> bool {
        if let Some(ref path) = self.cli_path {
            return path.exists();
        }
        std::process::Command::new("axolotl")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

// ── Harness capability enumeration ───────────────────────────────────

/// Harness capabilities — features that a training harness supports.
/// Used for capability filtering when generating provider-specific config
/// from canonical `TrainingParams`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarnessCapability {
    Qlora4bit,
    Qlora8bit,
    DoubleQuant,
    RsLora,
    SequencePacking,
    Neftune,
    FlashAttention2,
    FlashAttention3,
    Sdpa,
    GradientCheckpointing,
    Fp8Mixed,
    DeepSpeed,
    Fsdp,
    SampleGeneration,
    LoraPlus,
}

impl HarnessCapability {
    pub fn cns_span(&self) -> &'static str {
        match self {
            HarnessCapability::Qlora4bit => "cns.training.harness.qlora_4bit",
            HarnessCapability::Qlora8bit => "cns.training.harness.qlora_8bit",
            HarnessCapability::DoubleQuant => "cns.training.harness.double_quant",
            HarnessCapability::RsLora => "cns.training.harness.rslora",
            HarnessCapability::SequencePacking => "cns.training.harness.sequence_packing",
            HarnessCapability::Neftune => "cns.training.harness.neftune",
            HarnessCapability::FlashAttention2 => "cns.training.harness.flash_attn2",
            HarnessCapability::FlashAttention3 => "cns.training.harness.flash_attn3",
            HarnessCapability::Sdpa => "cns.training.harness.sdpa",
            HarnessCapability::GradientCheckpointing => "cns.training.harness.grad_ckpt",
            HarnessCapability::Fp8Mixed => "cns.training.harness.fp8_mixed",
            HarnessCapability::DeepSpeed => "cns.training.harness.deepspeed",
            HarnessCapability::Fsdp => "cns.training.harness.fsdp",
            HarnessCapability::SampleGeneration => "cns.training.harness.sample_gen",
            HarnessCapability::LoraPlus => "cns.training.harness.loraplus",
        }
    }
}

// ── HarnessAdapter trait ───────────────────────────────────────────────────

/// Renders training configuration in a harness-specific format.
///
/// The *harness* is the tooling that orchestrates training (axolotl CLI,
/// unsloth Python, TRL SFTTrainer). The *host* is where compute runs.
/// Each host binds exactly one harness; the harness generates the config
/// or script that the host dispatches.
///
/// pre:  job.params carries full expanded TrainingParams
/// post: returns harness-native config string (YAML, Python script, etc.)
///
/// MDS: Composition — CAN render_config ON TrainingJob VIA HarnessAdapter
pub trait HarnessAdapter: Send + Sync {
    /// Render the training configuration in the harness's native format.
    fn render_config(&self, job: &TrainingJob) -> Result<String, ProviderError>;

    /// Directory where the harness outputs adapter weights.
    fn output_dir(&self, job_id: &str) -> PathBuf;

    /// File whose existence signals training completion.
    fn completion_marker(&self, job_id: &str) -> PathBuf;

    /// The harness identifier for CNS spans.
    fn harness_id(&self) -> TrainingHarnessId;
}

// ── Axolotl harness ────────────────────────────────────────────────────────

/// Renders axolotl YAML configuration from canonical TrainingParams.
pub struct AxolotlHarness;

impl HarnessAdapter for AxolotlHarness {
    fn render_config(&self, job: &TrainingJob) -> Result<String, ProviderError> {
        let p = &job.params;
        let lo = &p.lora;
        let q = &p.quantization;
        let opt = &p.optimization;
        let seq = &p.sequence;
        let adv = &p.advanced;

        let mut yaml = String::new();
        yaml.push_str("# Auto-generated by hKask Training Server (harness: axolotl)\n");
        yaml.push_str(&format!("base_model: {}\n", job.base_model));
        yaml.push_str("datasets:\n");
        yaml.push_str(&format!("  - path: {}\n", job.dataset_path.display()));
        yaml.push_str("    type: chatml\n");
        yaml.push_str(&format!(
            "output_dir: {}\n",
            self.output_dir(&job.id).display()
        ));
        yaml.push_str(&format!("num_epochs: {}\n", p.num_epochs));
        yaml.push_str(&format!("micro_batch_size: {}\n", p.batch_size));
        yaml.push_str(&format!("learning_rate: {:.6}\n", p.learning_rate));
        yaml.push_str("adapter: lora\n");
        yaml.push_str(&format!("lora_r: {}\n", lo.r));
        yaml.push_str(&format!("lora_alpha: {}\n", lo.alpha));
        yaml.push_str(&format!("lora_dropout: {}\n", lo.dropout));
        yaml.push_str("lora_target_modules:\n");
        for m in &lo.target_modules {
            yaml.push_str(&format!("  - {}\n", m));
        }
        if !lo.modules_to_save.is_empty() {
            yaml.push_str("lora_modules_to_save:\n");
            for m in &lo.modules_to_save {
                yaml.push_str(&format!("  - {}\n", m));
            }
        }
        if lo.use_rslora {
            yaml.push_str("use_rslora: true\n");
        }

        // Quantization
        if q.load_in_4bit {
            yaml.push_str("load_in_4bit: true\n");
            if let Some(ref dt) = q.bnb_4bit_compute_dtype {
                yaml.push_str(&format!("bnb_4bit_compute_dtype: {}\n", dt));
            }
            if let Some(ref qt) = q.bnb_4bit_quant_type {
                yaml.push_str(&format!("bnb_4bit_quant_type: {}\n", qt));
            }
            if q.bnb_4bit_use_double_quant {
                yaml.push_str("bnb_4bit_use_double_quant: true\n");
            }
        } else if q.load_in_8bit {
            yaml.push_str("load_in_8bit: true\n");
        }

        // Optimizer & scheduler
        if let Some(ref optimizer) = opt.optimizer {
            yaml.push_str(&format!("optimizer: {}\n", optimizer));
        }
        if opt.weight_decay > 0.0 {
            yaml.push_str(&format!("weight_decay: {}\n", opt.weight_decay));
        }
        if opt.gradient_accumulation_steps > 1 {
            yaml.push_str(&format!(
                "gradient_accumulation_steps: {}\n",
                opt.gradient_accumulation_steps
            ));
        }
        if let Some(ref sched) = opt.lr_scheduler {
            yaml.push_str(&format!("lr_scheduler: {}\n", sched));
        }
        if let Some(ws) = opt.warmup_steps {
            yaml.push_str(&format!("warmup_steps: {}\n", ws));
        } else if let Some(wr) = opt.warmup_ratio {
            yaml.push_str(&format!("warmup_ratio: {}\n", wr));
        }
        if let Some(mgn) = opt.max_grad_norm {
            yaml.push_str(&format!("max_grad_norm: {}\n", mgn));
        }

        // Sequence
        if let Some(sl) = seq.sequence_len {
            yaml.push_str(&format!("sequence_len: {}\n", sl));
        }
        if seq.sample_packing {
            yaml.push_str("sample_packing: true\n");
        }
        if seq.pad_to_sequence_len {
            yaml.push_str("pad_to_sequence_len: true\n");
        }
        if let Some(na) = seq.neftune_noise_alpha {
            yaml.push_str(&format!("neftune_noise_alpha: {}\n", na));
        }

        // Advanced
        if let Some(ref attn) = adv.attn_implementation {
            yaml.push_str(&format!("attn_implementation: {}\n", attn));
        }
        if let Some(ref gc) = adv.gradient_checkpointing {
            yaml.push_str(&format!("gradient_checkpointing: {}\n", gc));
        }
        if adv.bf16 {
            yaml.push_str("bf16: true\n");
        } else if adv.fp16 {
            yaml.push_str("fp16: true\n");
        }
        if let Some(esr) = adv.eval_split_ratio {
            yaml.push_str(&format!("eval_split_ratio: {}\n", esr));
        }

        Ok(yaml)
    }

    fn output_dir(&self, job_id: &str) -> PathBuf {
        PathBuf::from(format!("./axolotl-output/{}", job_id))
    }

    fn completion_marker(&self, job_id: &str) -> PathBuf {
        self.output_dir(job_id).join("adapter_model.safetensors")
    }

    fn harness_id(&self) -> TrainingHarnessId {
        TrainingHarnessId::Axolotl
    }
}

#[async_trait::async_trait]
impl TrainingHost for AxolotlProvider {
    async fn submit(&self, job: &TrainingJob) -> Result<String, ProviderError> {
        if !self.available() {
            return Err(ProviderError::Unavailable(
                "axolotl CLI not found. Install with: pip install axolotl".to_string(),
            ));
        }

        let config_yaml = self.harness.render_config(job)?;
        let config_path = std::env::temp_dir().join(format!("hkask-training-{}.yaml", job.id));
        std::fs::write(&config_path, &config_yaml).map_err(|e| {
            ProviderError::Backend(format!("Failed to write axolotl config: {}", e))
        })?;

        let cli = self
            .cli_path
            .as_deref()
            .unwrap_or(std::path::Path::new("axolotl"));
        let mut child = tokio::process::Command::new(cli)
            .arg("train")
            .arg(&config_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ProviderError::Backend(format!("Failed to spawn axolotl: {}", e)))?;

        // Store PID for cancellation support
        if let Some(pid) = child.id() {
            self.jobs
                .lock()
                .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?
                .insert(job.id.clone(), pid);
        }

        // Spawn a background task to clean up the PID entry when the process exits
        let job_id = job.id.clone();
        let jobs = Arc::clone(&self.jobs);
        tokio::spawn(async move {
            let _ = child.wait().await;
            if let Ok(mut map) = jobs.lock() {
                map.remove(&job_id);
            }
        });

        tracing::info!(
            target: "cns.training.job.submit",
            job_id = %job.id,
            host = "axolotl",
            "Training job submitted"
        );

        Ok(job.id.clone())
    }

    async fn status(&self, job_id: &str) -> Result<TrainingJobStatus, ProviderError> {
        let completion = self.harness.completion_marker(job_id);
        if completion.exists() {
            return Ok(TrainingJobStatus::Completed);
        }
        let output_dir = self.harness.output_dir(job_id);
        if output_dir.join("checkpoint").exists() {
            return Ok(TrainingJobStatus::Running);
        }
        if output_dir.exists() {
            return Ok(TrainingJobStatus::Queued);
        }
        Err(ProviderError::JobFailed(format!(
            "Job {} not found or no output produced",
            job_id
        )))
    }

    async fn cancel(&self, job_id: &str) -> Result<(), ProviderError> {
        let pid = {
            let map = self
                .jobs
                .lock()
                .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
            map.get(job_id).copied()
        };

        if let Some(pid) = pid {
            // Send SIGTERM to the training process
            let _ = std::process::Command::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();

            if let Ok(mut map) = self.jobs.lock() {
                map.remove(job_id);
            }

            tracing::info!(
                target: "cns.training.job.cancel",
                job_id = %job_id,
                pid = %pid,
                "Axolotl training job cancelled (SIGTERM)"
            );
        } else {
            tracing::warn!(
                target: "cns.training.job.cancel",
                job_id = %job_id,
                "No PID found for job — process may have already exited"
            );
        }
        Ok(())
    }

    async fn list_adapters(&self) -> Result<Vec<String>, ProviderError> {
        let output_root = PathBuf::from("./axolotl-output");
        if !output_root.exists() {
            return Ok(vec![]);
        }
        let mut adapters = Vec::new();
        for entry in std::fs::read_dir(&output_root).map_err(|e| {
            ProviderError::Backend(format!("Failed to read axolotl output dir: {}", e))
        })? {
            let entry = entry
                .map_err(|e| ProviderError::Backend(format!("Failed to read dir entry: {}", e)))?;
            if entry.path().join("adapter_model.safetensors").exists() {
                adapters.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        Ok(adapters)
    }

    async fn delete_adapter(&self, adapter_id: &str) -> Result<(), ProviderError> {
        let adapter_dir = PathBuf::from("./axolotl-output").join(adapter_id);
        if adapter_dir.exists() {
            tokio::fs::remove_dir_all(&adapter_dir).await.map_err(|e| {
                ProviderError::Backend(format!("Failed to delete adapter {}: {}", adapter_id, e))
            })?;
            tracing::info!(
                target: "cns.training.adapter.deleted",
                adapter_id = %adapter_id,
                "LoRA adapter deleted"
            );
        }
        Ok(())
    }

    async fn adapter_weight_path(
        &self,
        adapter_id: &str,
    ) -> Result<Option<PathBuf>, ProviderError> {
        let path = PathBuf::from("./axolotl-output")
            .join(adapter_id)
            .join("adapter_model.safetensors");
        if path.exists() {
            Ok(Some(path))
        } else {
            Ok(None)
        }
    }
}

// ── Unsloth harness ────────────────────────────────────────────────────────

/// Renders unsloth Python training script from canonical TrainingParams.
pub struct UnslothHarness;

impl HarnessAdapter for UnslothHarness {
    fn render_config(&self, job: &TrainingJob) -> Result<String, ProviderError> {
        let p = &job.params;
        let lo = &p.lora;
        let q = &p.quantization;
        let opt = &p.optimization;
        let seq = &p.sequence;
        let adv = &p.advanced;

        let target_modules_str = lo
            .target_modules
            .iter()
            .map(|m| format!("\"{}\"", m))
            .collect::<Vec<_>>()
            .join(", ");

        let mut script = String::new();
        script.push_str("# Auto-generated by hKask Training Server (harness: unsloth)\n");
        script.push_str("import torch\n");
        script.push_str("from unsloth import FastLanguageModel\n");
        script.push_str("from datasets import load_dataset\n\n");

        // Model loading with quantization
        let load_kwargs = if q.load_in_4bit {
            ", load_in_4bit=True"
        } else {
            ""
        };
        let max_seq = seq.sequence_len.unwrap_or(2048);
        script.push_str(&format!(
            "model, tokenizer = FastLanguageModel.from_pretrained(\n    model_name=\"{}\",\n    max_seq_length={}{},\n)\n\n",
            job.base_model, max_seq, load_kwargs
        ));

        // PEFT config
        let grad_ckpt = adv.gradient_checkpointing.as_deref().unwrap_or("unsloth");
        script.push_str("model = FastLanguageModel.get_peft_model(\n    model,\n");
        script.push_str(&format!("    r={},\n", lo.r));
        script.push_str(&format!("    lora_alpha={},\n", lo.alpha));
        script.push_str(&format!("    target_modules=[{}],\n", target_modules_str));
        script.push_str(&format!("    lora_dropout={},\n", lo.dropout));
        script.push_str("    bias=\"none\",\n");
        script.push_str(&format!(
            "    use_gradient_checkpointing=\"{}\",\n",
            grad_ckpt
        ));
        if lo.use_rslora {
            script.push_str("    use_rslora=True,\n");
        }
        script.push_str(")\n\n");

        // Dataset
        script.push_str(&format!(
            "dataset = load_dataset(\"json\", data_files=\"{}\", split=\"train\")\n\n",
            job.dataset_path.display()
        ));

        // TrainingArguments
        script.push_str("from transformers import TrainingArguments\n");
        script.push_str("from trl import SFTTrainer\n\n");
        script.push_str("trainer = SFTTrainer(\n    model=model,\n    tokenizer=tokenizer,\n    train_dataset=dataset,\n    dataset_text_field=\"text\",\n");
        script.push_str(&format!("    max_seq_length={},\n", max_seq));
        script.push_str("    args=TrainingArguments(\n");
        script.push_str(&format!(
            "        per_device_train_batch_size={},\n",
            p.batch_size
        ));
        script.push_str(&format!("        num_train_epochs={},\n", p.num_epochs));
        script.push_str(&format!("        learning_rate={},\n", p.learning_rate));
        if let Some(ref sched) = opt.lr_scheduler {
            script.push_str(&format!("        lr_scheduler_type=\"{}\",\n", sched));
        }
        if let Some(wr) = opt.warmup_ratio {
            script.push_str(&format!("        warmup_ratio={},\n", wr));
        }
        if opt.weight_decay > 0.0 {
            script.push_str(&format!("        weight_decay={},\n", opt.weight_decay));
        }
        if opt.gradient_accumulation_steps > 1 {
            script.push_str(&format!(
                "        gradient_accumulation_steps={},\n",
                opt.gradient_accumulation_steps
            ));
        }
        if adv.bf16 {
            script.push_str("        bf16=True,\n");
        } else if adv.fp16 {
            script.push_str("        fp16=True,\n");
        }
        if let Some(esr) = adv.eval_split_ratio {
            script.push_str("        eval_strategy=\"steps\",\n");
            script.push_str(&format!("        eval_steps={},\n", (100.0 / esr) as u32));
        }
        script.push_str(&format!(
            "        output_dir=\"{}\",\n",
            self.output_dir(&job.id).display()
        ));
        script.push_str("    ),\n");
        if seq.sample_packing {
            script.push_str("    packing=True,\n");
        }
        if let Some(na) = seq.neftune_noise_alpha {
            script.push_str(&format!("    neftune_noise_alpha={},\n", na));
        }
        script.push_str(")\n\n");

        script.push_str("trainer.train()\n");
        script.push_str(&format!(
            "model.save_pretrained(\"{}\")\n",
            self.completion_marker(&job.id)
                .parent()
                .expect("completion marker always has a parent directory")
                .display()
        ));

        Ok(script)
    }

    fn output_dir(&self, job_id: &str) -> PathBuf {
        PathBuf::from(format!("./unsloth-output/{}", job_id))
    }

    fn completion_marker(&self, job_id: &str) -> PathBuf {
        self.output_dir(job_id)
            .join("adapter")
            .join("adapter_model.safetensors")
    }

    fn harness_id(&self) -> TrainingHarnessId {
        TrainingHarnessId::Unsloth
    }
}

// ── Unsloth provider ───────────────────────────────────────────────────────

/// Unsloth training provider — wraps unsloth for memory-efficient fine-tuning.
///
/// Unsloth uses Python scripts rather than YAML configs. This provider
/// generates a training script from the canonical TrainingJob and executes it.
pub struct UnslothProvider {
    /// Path to Python interpreter (default: python3).
    python_path: Option<PathBuf>,
    /// Harness for rendering unsloth Python training script.
    harness: Box<dyn HarnessAdapter>,
    /// Running job PIDs for cancellation support.
    jobs: Arc<Mutex<HashMap<String, u32>>>,
}

impl UnslothProvider {
    /// Create a new unsloth provider with the given harness.
    pub fn new(python_path: Option<PathBuf>, harness: Box<dyn HarnessAdapter>) -> Self {
        Self {
            python_path,
            harness,
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check whether unsloth is available locally.
    fn available(&self) -> bool {
        let py = self
            .python_path
            .as_deref()
            .unwrap_or(std::path::Path::new("python3"));
        std::process::Command::new(py)
            .args(["-c", "import unsloth"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

#[async_trait::async_trait]
impl TrainingHost for UnslothProvider {
    async fn submit(&self, job: &TrainingJob) -> Result<String, ProviderError> {
        if !self.available() {
            return Err(ProviderError::Unavailable(
                "unsloth not found. Install with: pip install unsloth".to_string(),
            ));
        }

        let script = self.harness.render_config(job)?;
        let script_path = std::env::temp_dir().join(format!("hkask-training-{}.py", job.id));
        std::fs::write(&script_path, &script).map_err(|e| {
            ProviderError::Backend(format!("Failed to write unsloth script: {}", e))
        })?;

        let py = self
            .python_path
            .as_deref()
            .unwrap_or(std::path::Path::new("python3"));
        let mut child = tokio::process::Command::new(py)
            .arg(&script_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ProviderError::Backend(format!("Failed to spawn unsloth: {}", e)))?;

        // Store PID for cancellation support
        if let Some(pid) = child.id() {
            self.jobs
                .lock()
                .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?
                .insert(job.id.clone(), pid);
        }

        let job_id = job.id.clone();
        let jobs = Arc::clone(&self.jobs);
        tokio::spawn(async move {
            let _ = child.wait().await;
            if let Ok(mut map) = jobs.lock() {
                map.remove(&job_id);
            }
        });

        tracing::info!(
            target: "cns.training.job.submit",
            job_id = %job.id,
            host = "unsloth",
            "Training job submitted"
        );
        Ok(job.id.clone())
    }

    async fn status(&self, job_id: &str) -> Result<TrainingJobStatus, ProviderError> {
        let completion = self.harness.completion_marker(job_id);
        if completion.exists() {
            return Ok(TrainingJobStatus::Completed);
        }
        let output_dir = self.harness.output_dir(job_id);
        if output_dir.exists() {
            return Ok(TrainingJobStatus::Running);
        }
        Err(ProviderError::JobFailed(format!(
            "Job {} not found or no output produced",
            job_id
        )))
    }

    async fn cancel(&self, job_id: &str) -> Result<(), ProviderError> {
        let pid = {
            let map = self
                .jobs
                .lock()
                .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
            map.get(job_id).copied()
        };

        if let Some(pid) = pid {
            let _ = std::process::Command::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();

            if let Ok(mut map) = self.jobs.lock() {
                map.remove(job_id);
            }

            tracing::info!(
                target: "cns.training.job.cancel",
                job_id = %job_id,
                pid = %pid,
                "Unsloth training job cancelled (SIGTERM)"
            );
        } else {
            tracing::warn!(
                target: "cns.training.job.cancel",
                job_id = %job_id,
                "No PID found for job — process may have already exited"
            );
        }
        Ok(())
    }

    async fn list_adapters(&self) -> Result<Vec<String>, ProviderError> {
        let output_root = PathBuf::from("./unsloth-output");
        if !output_root.exists() {
            return Ok(vec![]);
        }
        let mut adapters = Vec::new();
        for entry in std::fs::read_dir(&output_root).map_err(|e| {
            ProviderError::Backend(format!("Failed to read unsloth output dir: {}", e))
        })? {
            let entry = entry
                .map_err(|e| ProviderError::Backend(format!("Failed to read dir entry: {}", e)))?;
            if entry.path().join("adapter").exists() {
                adapters.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        Ok(adapters)
    }

    async fn delete_adapter(&self, adapter_id: &str) -> Result<(), ProviderError> {
        let adapter_dir = PathBuf::from("./unsloth-output").join(adapter_id);
        if adapter_dir.exists() {
            tokio::fs::remove_dir_all(&adapter_dir).await.map_err(|e| {
                ProviderError::Backend(format!("Failed to delete adapter {}: {}", adapter_id, e))
            })?;
            tracing::info!(
                target: "cns.training.adapter.deleted",
                adapter_id = %adapter_id,
                "LoRA adapter deleted"
            );
        }
        Ok(())
    }

    async fn adapter_weight_path(
        &self,
        adapter_id: &str,
    ) -> Result<Option<PathBuf>, ProviderError> {
        let path = PathBuf::from("./unsloth-output")
            .join(adapter_id)
            .join("adapter");
        if path.exists() {
            Ok(Some(path))
        } else {
            Ok(None)
        }
    }
}

// ── Together AI cloud provider ──────────────────────────────────────────────

/// Together AI training provider — submits fine-tuning jobs via REST API.
///
/// Uses the Together AI fine-tuning API (<https://api.together.xyz/v1/fine-tunes>)
/// to submit LoRA fine-tuning jobs, poll status, and manage adapters.
/// No local GPU or CLI required.
pub struct TogetherProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl TogetherProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.together.xyz".to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl TrainingHost for TogetherProvider {
    async fn submit(&self, job: &TrainingJob) -> Result<String, ProviderError> {
        // Step 1: Upload the dataset file
        let file_name = job
            .dataset_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("dataset.jsonl");

        let file_bytes = tokio::fs::read(&job.dataset_path)
            .await
            .map_err(|e| ProviderError::Backend(format!("Failed to read dataset file: {}", e)))?;

        let file_part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name(file_name.to_string())
            .mime_str("application/jsonl")
            .map_err(|e| ProviderError::Backend(format!("Invalid MIME type: {}", e)))?;

        let form = reqwest::multipart::Form::new()
            .text("purpose", "fine-tune")
            .text("file_name", file_name.to_string())
            .part("file", file_part);

        let upload_response = self
            .client
            .post(format!("{}/v1/files/upload", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| ProviderError::Backend(format!("Together AI upload failed: {}", e)))?;

        let upload_status = upload_response.status();
        let upload_json: serde_json::Value = upload_response.json().await.map_err(|e| {
            ProviderError::Backend(format!("Together AI upload parse error: {}", e))
        })?;

        if !upload_status.is_success() {
            return Err(ProviderError::Backend(format!(
                "Together AI upload error {}: {}",
                upload_status,
                serde_json::to_string_pretty(&upload_json).unwrap_or_default()
            )));
        }

        let file_id = upload_json["id"]
            .as_str()
            .ok_or_else(|| ProviderError::Backend("No file ID in upload response".to_string()))?
            .to_string();

        tracing::info!(
            target: "cns.training.file.upload",
            file_id = %file_id,
            "Dataset uploaded to Together AI"
        );

        // Step 2: Submit the fine-tuning job
        let body = serde_json::json!({
            "model": job.base_model,
            "training_file": file_id,
            "n_epochs": job.params.num_epochs,
            "n_checkpoints": 1,
            "learning_rate": job.params.learning_rate,
            "lora": true,
            "lora_r": job.params.lora.r,
            "lora_alpha": job.params.lora.alpha,
            "batch_size": job.params.batch_size.max(8),
            "suffix": format!("hkask-{}", &job.id[..8]),
        });

        let response = self
            .client
            .post(format!("{}/v1/fine-tunes", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Backend(format!("Together AI request failed: {}", e)))?;

        let status = response.status();
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Backend(format!("Together AI parse error: {}", e)))?;

        if !status.is_success() {
            return Err(ProviderError::Backend(format!(
                "Together AI error {}: {}",
                status,
                serde_json::to_string_pretty(&json).unwrap_or_default()
            )));
        }

        let job_id = json["id"].as_str().unwrap_or("unknown").to_string();

        tracing::info!(
            target: "cns.training.job.submit",
            job_id = %job_id,
            host = "together",
            harness = ?job.harness,
            "Training job submitted to Together AI"
        );

        Ok(job_id)
    }

    async fn status(&self, job_id: &str) -> Result<TrainingJobStatus, ProviderError> {
        let response = self
            .client
            .get(format!("{}/v1/fine-tunes/{}", self.base_url, job_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                ProviderError::Backend(format!("Together AI status request failed: {}", e))
            })?;

        let status_code = response.status();
        let json: serde_json::Value = response.json().await.map_err(|e| {
            ProviderError::Backend(format!("Together AI status parse error: {}", e))
        })?;

        if !status_code.is_success() {
            return Err(ProviderError::Backend(format!(
                "Together AI status error {}: {}",
                status_code,
                serde_json::to_string_pretty(&json).unwrap_or_default()
            )));
        }

        let status_str = json["status"].as_str().unwrap_or("unknown");
        match status_str {
            "pending" | "queued" => Ok(TrainingJobStatus::Queued),
            "running" => Ok(TrainingJobStatus::Running),
            "completed" | "succeeded" => Ok(TrainingJobStatus::Completed),
            "failed" | "error" => Ok(TrainingJobStatus::Failed),
            "cancelled" => Ok(TrainingJobStatus::Cancelled),
            _ => Ok(TrainingJobStatus::Queued),
        }
    }

    async fn cancel(&self, job_id: &str) -> Result<(), ProviderError> {
        let response = self
            .client
            .post(format!("{}/v1/fine-tunes/{}/cancel", self.base_url, job_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                ProviderError::Backend(format!("Together AI cancel request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::Backend(format!(
                "Together AI cancel error: {}",
                body
            )));
        }

        tracing::info!(
            target: "cns.training.job.cancel",
            job_id = %job_id,
            host = "together",
            "Training job cancelled"
        );
        Ok(())
    }

    async fn list_adapters(&self) -> Result<Vec<String>, ProviderError> {
        let response = self
            .client
            .get(format!("{}/v1/fine-tunes", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                ProviderError::Backend(format!("Together AI list request failed: {}", e))
            })?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Backend(format!("Together AI list parse error: {}", e)))?;

        let adapters: Vec<String> = json["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter(|j| j["status"] == "completed" || j["status"] == "succeeded")
            .filter_map(|j| j["id"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(adapters)
    }

    async fn delete_adapter(&self, adapter_id: &str) -> Result<(), ProviderError> {
        let response = self
            .client
            .delete(format!("{}/v1/fine-tunes/{}", self.base_url, adapter_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                ProviderError::Backend(format!("Together AI delete request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::Backend(format!(
                "Together AI delete error: {}",
                body
            )));
        }

        tracing::info!(
            target: "cns.training.adapter.deleted",
            adapter_id = %adapter_id,
            host = "together",
            "LoRA adapter deleted from Together AI"
        );
        Ok(())
    }

    async fn completion_metadata(
        &self,
        job_id: &str,
    ) -> Result<Option<CompletionMetadata>, ProviderError> {
        let response = self
            .client
            .get(format!("{}/v1/fine-tunes/{}", self.base_url, job_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| {
                ProviderError::Backend(format!("Together AI metadata request failed: {}", e))
            })?;

        let json: serde_json::Value = response.json().await.map_err(|e| {
            ProviderError::Backend(format!("Together AI metadata parse error: {}", e))
        })?;

        let status_str = json["status"].as_str().unwrap_or("");
        if status_str != "completed" && status_str != "succeeded" {
            return Ok(None);
        }

        let base_model = json["model"].as_str().unwrap_or("unknown").to_string();
        let output_name = json["output_name"].as_str().map(|s| s.to_string());

        // Extract loss from the last training event
        let loss = json["events"].as_array().and_then(|events| {
            events.iter().rev().find_map(|e| {
                e.get("type")
                    .and_then(|t| t.as_str())
                    .filter(|t| *t == "training_loss" || *t == "checkpoint")
                    .and_then(|_| e.get("data").and_then(|d| d.get("loss")))
                    .and_then(|l| l.as_f64())
                    .map(|l| l as f32)
            })
        });

        let tokens_processed = json["events"].as_array().and_then(|events| {
            events.iter().rev().find_map(|e| {
                e.get("type")
                    .and_then(|t| t.as_str())
                    .filter(|t| *t == "training_loss" || *t == "checkpoint")
                    .and_then(|_| e.get("data").and_then(|d| d.get("tokens")))
                    .and_then(|t| t.as_u64())
            })
        });

        let training_duration_secs = json["events"].as_array().and_then(|events| {
            let created = events.first()?.get("created_at")?.as_i64()?;
            let finished = events.last()?.get("created_at")?.as_i64()?;
            Some((finished - created) as u64)
        });

        Ok(Some(CompletionMetadata {
            base_model,
            output_name,
            loss,
            training_duration_secs,
            tokens_processed,
        }))
    }
}

// ── Runpod provider ─────────────────────────────────────────────────────

/// Runpod GPU cloud training provider — dispatches training to GPU pods.
///
/// Uses the Runpod GraphQL API to create GPU pods from a pre-built template
/// (with axolotl installed), execute training, and retrieve LoRA adapters.
/// This is the "cloud dispatch" path for Axolotl — instead of running locally,
/// training runs on Runpod's GPU infrastructure.
///
/// **Template requirements:** The pod template must include a startup script
/// that reads `HKASK_*` environment variables, downloads the dataset from
/// `HKASK_DATASET_URL`, runs axolotl training, and uploads the resulting
/// adapter weights to a storage location.
///
/// Environment variables:
/// - `RUNPOD_API_KEY` — Runpod API key
/// - `RUNPOD_TEMPLATE_ID` — GPU pod template ID with axolotl pre-installed
/// - `RUNPOD_GPU_TYPE_ID` — GPU type ID (default: "NVIDIA RTX 4090")
/// - `RUNPOD_CONTAINER_DISK_GB` — Container disk in GB (default: 50)
/// - `RUNPOD_MIN_MEMORY_GB` — Minimum memory in GB (default: 24)
/// - `RUNPOD_MIN_VCPU_COUNT` — Minimum vCPU count (default: 8)
/// - `HKASK_DATASET_URL` — Public URL where the pod can download the dataset
///   (set this before calling training_submit with runpod provider)
pub struct RunpodProvider {
    api_key: String,
    template_id: String,
    graphql_url: String,
    client: reqwest::Client,
    /// job_id → pod_id mapping for status/cancel
    jobs: Arc<Mutex<HashMap<String, String>>>,
}

impl RunpodProvider {
    pub fn new(api_key: String, template_id: String) -> Self {
        Self {
            api_key,
            template_id,
            graphql_url: "https://api.runpod.io/graphql".to_string(),
            client: reqwest::Client::new(),
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn graphql_query(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError> {
        let body = json!({
            "query": query,
            "variables": variables,
        });

        let response = self
            .client
            .post(&self.graphql_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Backend(format!("Runpod API request failed: {}", e)))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Backend(format!("Runpod API parse error: {}", e)))?;

        if let Some(errors) = json.get("errors") {
            return Err(ProviderError::Backend(format!(
                "Runpod GraphQL errors: {}",
                serde_json::to_string_pretty(errors).unwrap_or_default()
            )));
        }

        Ok(json)
    }
}

#[async_trait::async_trait]
impl TrainingHost for RunpodProvider {
    async fn submit(&self, job: &TrainingJob) -> Result<String, ProviderError> {
        // Create a GPU pod from the template
        let mutation = r#"
            mutation CreatePod($input: PodCreateAndDeployInput!) {
                podCreateAndDeploy(input: $input) {
                    id
                    name
                    desiredStatus
                    runtime { uptimeInSeconds }
                }
            }
        "#;

        let gpu_type_id =
            std::env::var("RUNPOD_GPU_TYPE_ID").unwrap_or_else(|_| "NVIDIA RTX 4090".to_string());
        let container_disk_gb: u32 = std::env::var("RUNPOD_CONTAINER_DISK_GB")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(50);
        let min_memory_gb: u32 = std::env::var("RUNPOD_MIN_MEMORY_GB")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(24);
        let min_vcpu: u32 = std::env::var("RUNPOD_MIN_VCPU_COUNT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8);
        let dataset_url = std::env::var("HKASK_DATASET_URL").unwrap_or_default();

        let variables = json!({
            "input": {
                "name": format!("hkask-training-{}", &job.id[..8]),
                "templateId": self.template_id,
                "gpuTypeId": gpu_type_id,
                "containerDiskInGb": container_disk_gb,
                "minMemoryInGb": min_memory_gb,
                "minVcpuCount": min_vcpu,
                "env": [
                    { "key": "HKASK_JOB_ID", "value": job.id },
                    { "key": "HKASK_BASE_MODEL", "value": job.base_model },
                    { "key": "HKASK_DATASET_URL", "value": dataset_url },
                    { "key": "HKASK_HARNESS", "value": format!("{:?}", job.harness).to_lowercase() },
                    { "key": "HKASK_NUM_EPOCHS", "value": job.params.num_epochs.to_string() },
                    { "key": "HKASK_LORA_R", "value": job.params.lora.r.to_string() },
                    { "key": "HKASK_LORA_ALPHA", "value": job.params.lora.alpha.to_string() },
                    { "key": "HKASK_LEARNING_RATE", "value": job.params.learning_rate.to_string() },
                    { "key": "HKASK_BATCH_SIZE", "value": job.params.batch_size.to_string() },
                ],
            }
        });

        let result = self.graphql_query(mutation, variables).await?;

        let pod_id = result["data"]["podCreateAndDeploy"]["id"]
            .as_str()
            .ok_or_else(|| ProviderError::Backend("No pod ID in Runpod response".to_string()))?
            .to_string();

        // Store pod_id for status/cancel
        if let Ok(mut map) = self.jobs.lock() {
            map.insert(job.id.clone(), pod_id.clone());
        }

        tracing::info!(
            target: "cns.training.job.submit",
            job_id = %job.id,
            pod_id = %pod_id,
            host = "runpod",
            harness = ?job.harness,
            "Training pod created on Runpod"
        );

        Ok(job.id.clone())
    }

    async fn status(&self, job_id: &str) -> Result<TrainingJobStatus, ProviderError> {
        let pod_id = {
            let map = self
                .jobs
                .lock()
                .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
            map.get(job_id).cloned()
        };

        let pod_id = match pod_id {
            Some(id) => id,
            None => {
                return Err(ProviderError::JobFailed(format!(
                    "No pod found for job {}",
                    job_id
                )));
            }
        };

        let query = r#"
            query GetPod($id: String!) {
                pod(input: { podId: $id }) {
                    id
                    desiredStatus
                    runtime { uptimeInSeconds }
                    machine { gpuType }
                }
            }
        "#;

        let result = self.graphql_query(query, json!({ "id": pod_id })).await?;

        let status_str = result["data"]["pod"]["desiredStatus"]
            .as_str()
            .unwrap_or("UNKNOWN");

        match status_str {
            "CREATING" | "PENDING" => Ok(TrainingJobStatus::Queued),
            "RUNNING" => Ok(TrainingJobStatus::Running),
            "STOPPED" | "TERMINATED" => {
                // Check if training output exists — if pod stopped with output, it completed
                // For now, treat STOPPED as potentially completed
                Ok(TrainingJobStatus::Completed)
            }
            "FAILED" | "ERROR" => Ok(TrainingJobStatus::Failed),
            _ => Ok(TrainingJobStatus::Queued),
        }
    }

    async fn cancel(&self, job_id: &str) -> Result<(), ProviderError> {
        let pod_id = {
            let map = self
                .jobs
                .lock()
                .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
            map.get(job_id).cloned()
        };

        let pod_id = match pod_id {
            Some(id) => id,
            None => {
                tracing::warn!(
                    target: "cns.training.job.cancel",
                    job_id = %job_id,
                    "No pod found for job"
                );
                return Ok(());
            }
        };

        let mutation = r#"
            mutation TerminatePod($id: String!) {
                podTerminate(input: { podId: $id })
            }
        "#;

        self.graphql_query(mutation, json!({ "id": pod_id }))
            .await?;

        if let Ok(mut map) = self.jobs.lock() {
            map.remove(job_id);
        }

        tracing::info!(
            target: "cns.training.job.cancel",
            job_id = %job_id,
            pod_id = %pod_id,
            host = "runpod",
            "Training pod terminated"
        );
        Ok(())
    }

    async fn list_adapters(&self) -> Result<Vec<String>, ProviderError> {
        // List completed pods — adapters are identified by job_id
        let map = self
            .jobs
            .lock()
            .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
        Ok(map.keys().cloned().collect())
    }

    async fn delete_adapter(&self, adapter_id: &str) -> Result<(), ProviderError> {
        // Terminate the pod if still running
        let _ = self.cancel(adapter_id).await;
        Ok(())
    }

    async fn completion_metadata(
        &self,
        _job_id: &str,
    ) -> Result<Option<CompletionMetadata>, ProviderError> {
        // Runpod doesn't provide structured training metrics via API.
        // Metrics would need to be extracted from the pod's output logs.
        Ok(None)
    }

    async fn adapter_weight_path(
        &self,
        _adapter_id: &str,
    ) -> Result<Option<PathBuf>, ProviderError> {
        // Weights are on the Runpod pod — need to be downloaded separately.
        Ok(None)
    }
}

// ── Trainer harness (TRL) ──────────────────────────────────────────────────

/// Renders TRL SFTTrainer Python script for Baseten and generic TRL hosts.
pub struct TrainerHarness;

impl HarnessAdapter for TrainerHarness {
    fn render_config(&self, _job: &TrainingJob) -> Result<String, ProviderError> {
        Err(ProviderError::InvalidConfig(
            "TrainerHarness requires model_id from host — use render_with_model".to_string(),
        ))
    }

    fn output_dir(&self, job_id: &str) -> PathBuf {
        PathBuf::from(format!("./trl-output/{}", job_id))
    }

    fn completion_marker(&self, job_id: &str) -> PathBuf {
        self.output_dir(job_id).join("adapter_model.safetensors")
    }

    fn harness_id(&self) -> TrainingHarnessId {
        TrainingHarnessId::Unsloth
    }
}

impl TrainerHarness {
    /// Render with an explicit HuggingFace model ID (used by Baseten).
    pub fn render_with_model(&self, job: &TrainingJob, hf_model_id: &str) -> String {
        let p = &job.params;
        let lo = &p.lora;
        let opt = &p.optimization;

        let target_modules_str = lo
            .target_modules
            .iter()
            .map(|m| format!("\"{}\"", m))
            .collect::<Vec<_>>()
            .join(", ");

        let max_seq = p.sequence.sequence_len.unwrap_or(2048);
        let grad_accum = opt.gradient_accumulation_steps.max(1);
        let warmup = opt.warmup_ratio.unwrap_or(0.1);
        let lr_scheduler = opt.lr_scheduler.as_deref().unwrap_or("cosine");
        let grad_ckpt = p
            .advanced
            .gradient_checkpointing
            .as_deref()
            .unwrap_or("true");
        let dtype = if p.advanced.bf16 {
            ", torch_dtype=torch.bfloat16"
        } else if p.advanced.fp16 {
            ", torch_dtype=torch.float16"
        } else {
            ""
        };

        format!(
            r#"# Auto-generated by hKask Training Server (harness: trl)
import os
import torch
from datasets import load_dataset
from peft import LoraConfig
from transformers import AutoModelForCausalLM, AutoTokenizer
from trl import SFTConfig, SFTTrainer

# Base model loaded from HuggingFace (mounted by host)
model_id = "{hf_model_id}"
print(f"Loading base model: {{model_id}}")

tokenizer = AutoTokenizer.from_pretrained(model_id, trust_remote_code=True)
if tokenizer.pad_token is None:
    tokenizer.pad_token = tokenizer.eos_token

model = AutoModelForCausalLM.from_pretrained(
    model_id,
    device_map="auto"{dtype},
    trust_remote_code=True,
)

# LoRA configuration
peft_config = LoraConfig(
    r={lora_r},
    lora_alpha={lora_alpha},
    target_modules=[{target_modules}],
    lora_dropout={lora_dropout},
    task_type="CAUSAL_LM",
)

# Load dataset (from URL or local path)
import urllib.request
dataset_url = os.getenv("HKASK_DATASET_URL", "")
if dataset_url:
    print(f"Downloading dataset: {{dataset_url}}")
    urllib.request.urlretrieve(dataset_url, "dataset.jsonl")
    dataset_path = "dataset.jsonl"
else:
    dataset_path = os.getenv("HKASK_DATASET_PATH", "dataset.jsonl")
print(f"Loading dataset: {{dataset_path}}")
dataset = load_dataset("json", data_files=dataset_path, split="train")

def format_chatml(examples):
    texts = []
    for messages in examples["messages"]:
        text = ""
        for msg in messages:
            role = msg.get("role", "user")
            content = msg.get("content", "")
            text += f"<|{{role}}|>\n{{content}}\n"
        texts.append(text)
    return {{"text": texts}}

dataset = dataset.map(format_chatml, batched=True, remove_columns=dataset.column_names)

# Training arguments
training_args = SFTConfig(
    learning_rate={learning_rate},
    num_train_epochs={num_epochs},
    per_device_train_batch_size={batch_size},
    gradient_accumulation_steps={grad_accum},
    gradient_checkpointing={grad_ckpt},
    max_seq_length={max_seq},
    warmup_ratio={warmup},
    lr_scheduler_type="{lr_scheduler}",
    save_steps=50,
    bf16=True,
    output_dir=os.getenv("BT_CHECKPOINT_DIR", "./checkpoints"),
    logging_steps=10,
)

trainer = SFTTrainer(
    model=model,
    args=training_args,
    train_dataset=dataset,
    processing_class=tokenizer,
    peft_config=peft_config,
)

print("Starting training...")
trainer.train()
print("Training complete. Checkpoints saved to BT_CHECKPOINT_DIR.")
"#,
            hf_model_id = hf_model_id,
            lora_r = lo.r,
            lora_alpha = lo.alpha,
            lora_dropout = lo.dropout,
            target_modules = target_modules_str,
            learning_rate = p.learning_rate,
            num_epochs = p.num_epochs,
            batch_size = p.batch_size,
            max_seq = max_seq,
            grad_accum = grad_accum,
            warmup = warmup,
            lr_scheduler = lr_scheduler,
            grad_ckpt = grad_ckpt,
            dtype = dtype,
        )
    }
}

// ── Baseten provider ────────────────────────────────────────────────────

/// Baseten managed training provider — runs your training code on their GPU infra.
///
/// Uses the Baseten REST API to submit training jobs with a generated `train.py`
/// script that loads models from HuggingFace, applies LoRA via TRL/SFTTrainer,
/// and saves checkpoints for automatic deployment.
///
/// **Model loading:** Base models are loaded from HuggingFace via Baseten's
/// weights mount system (`hf://` source). Requires `HF_TOKEN` in Baseten Secrets
/// or passed as an environment variable.
///
/// Environment variables:
/// - `BASETEN_API_KEY` — Baseten API key
/// - `BASETEN_PROJECT_ID` — Baseten training project ID
/// - `HF_TOKEN` — HuggingFace access token (for gated model loading)
/// - `BASETEN_GPU` — GPU accelerator type (default: "H100")
/// - `BASETEN_GPU_COUNT` — Number of GPUs (default: 1)
pub struct BasetenProvider {
    api_key: String,
    project_id: String,
    base_url: String,
    client: reqwest::Client,
    /// Harness for rendering TRL training script.
    harness: TrainerHarness,
    /// job_id tracking for status/cancel
    jobs: Arc<Mutex<HashMap<String, String>>>,
}

impl BasetenProvider {
    pub fn new(api_key: String, project_id: String) -> Self {
        Self {
            api_key,
            project_id,
            base_url: "https://api.baseten.co".to_string(),
            client: reqwest::Client::new(),
            harness: TrainerHarness,
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl TrainingHost for BasetenProvider {
    async fn submit(&self, job: &TrainingJob) -> Result<String, ProviderError> {
        // Resolve HuggingFace model ID via canonical resolver.
        let hf_model_id = crate::huggingface::resolve_model_id(&job.base_model);

        // Generate train.py via harness and encode as base64
        let train_script = self.harness.render_with_model(job, &hf_model_id);
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(train_script.as_bytes());

        let gpu = std::env::var("BASETEN_GPU").unwrap_or_else(|_| "H100".to_string());
        let gpu_count: u32 = std::env::var("BASETEN_GPU_COUNT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let hf_token = std::env::var("HF_TOKEN").unwrap_or_default();
        let dataset_url = std::env::var("HKASK_DATASET_URL").unwrap_or_default();

        let body = json!({
            "training_job": {
                "name": format!("hkask-training-{}", &job.id[..8]),
                "image": {
                    "base_image": "baseten/trt-llm-train:latest",
                },
                "compute": {
                    "node_count": 1,
                    "cpu_count": 8,
                    "memory": "32Gi",
                    "accelerator": {
                        "accelerator": gpu,
                        "count": gpu_count,
                    },
                },
                "runtime": {
                    "start_commands": [
                        "pip install peft trl datasets accelerate",
                        format!("python -c \"import base64; open('train.py','w').write(base64.b64decode('{}').decode())\"", encoded),
                        "python train.py",
                    ],
                    "environment_variables": {
                        "HKASK_JOB_ID": job.id,
                        "HKASK_BASE_MODEL": job.base_model,
                        "HKASK_DATASET_URL": dataset_url,
                        "HKASK_NUM_EPOCHS": job.params.num_epochs.to_string(),
                        "HKASK_LORA_R": job.params.lora.r.to_string(),
                        "HF_TOKEN": hf_token,
                    },
                    "checkpointing_config": {
                        "enabled": true,
                        "checkpoint_path": "/mnt/ckpts",
                        "volume_size_gib": 20,
                    },
                },
                "weights": [
                    {
                        "source": format!("hf://{}", hf_model_id),
                        "mount_location": format!("/app/models/{}", hf_model_id),
                    }
                ],
            }
        });

        let url = format!(
            "{}/v1/training_projects/{}/jobs",
            self.base_url, self.project_id
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Backend(format!("Baseten API request failed: {}", e)))?;

        let status_code = response.status();
        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Backend(format!("Baseten API parse error: {}", e)))?;

        if !status_code.is_success() {
            return Err(ProviderError::Backend(format!(
                "Baseten error {}: {}",
                status_code,
                serde_json::to_string_pretty(&json).unwrap_or_default()
            )));
        }

        let baseten_job_id = json["training_job"]["id"]
            .as_str()
            .or_else(|| json["id"].as_str())
            .unwrap_or("unknown")
            .to_string();

        // Store mapping for status/cancel
        if let Ok(mut map) = self.jobs.lock() {
            map.insert(job.id.clone(), baseten_job_id.clone());
        }

        tracing::info!(
            target: "cns.training.job.submit",
            job_id = %job.id,
            baseten_job_id = %baseten_job_id,
            host = "baseten",
            "Training job submitted to Baseten"
        );

        Ok(job.id.clone())
    }

    async fn status(&self, job_id: &str) -> Result<TrainingJobStatus, ProviderError> {
        let baseten_job_id = {
            let map = self
                .jobs
                .lock()
                .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
            map.get(job_id).cloned()
        };

        let baseten_job_id = match baseten_job_id {
            Some(id) => id,
            None => {
                return Err(ProviderError::JobFailed(format!(
                    "No Baseten job found for {}",
                    job_id
                )));
            }
        };

        let url = format!(
            "{}/v1/training_projects/{}/jobs/{}",
            self.base_url, self.project_id, baseten_job_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| ProviderError::Backend(format!("Baseten status request failed: {}", e)))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Backend(format!("Baseten status parse error: {}", e)))?;

        let status_str = json["training_job"]["status"]
            .as_str()
            .or_else(|| json["status"].as_str())
            .unwrap_or("unknown");

        match status_str {
            "PENDING" | "QUEUED" | "CREATING" => Ok(TrainingJobStatus::Queued),
            "RUNNING" | "TRAINING" => Ok(TrainingJobStatus::Running),
            "COMPLETED" | "SUCCEEDED" | "DONE" => Ok(TrainingJobStatus::Completed),
            "FAILED" | "ERROR" | "CANCELLED" => Ok(TrainingJobStatus::Failed),
            _ => Ok(TrainingJobStatus::Queued),
        }
    }

    async fn cancel(&self, job_id: &str) -> Result<(), ProviderError> {
        let baseten_job_id = {
            let map = self
                .jobs
                .lock()
                .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
            map.get(job_id).cloned()
        };

        let baseten_job_id = match baseten_job_id {
            Some(id) => id,
            None => {
                tracing::warn!(
                    target: "cns.training.job.cancel",
                    job_id = %job_id,
                    "No Baseten job found"
                );
                return Ok(());
            }
        };

        let url = format!(
            "{}/v1/training_projects/{}/jobs/{}",
            self.base_url, self.project_id, baseten_job_id
        );

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| ProviderError::Backend(format!("Baseten cancel request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::Backend(format!(
                "Baseten cancel error: {}",
                body
            )));
        }

        if let Ok(mut map) = self.jobs.lock() {
            map.remove(job_id);
        }

        tracing::info!(
            target: "cns.training.job.cancel",
            job_id = %job_id,
            host = "baseten",
            "Training job cancelled on Baseten"
        );
        Ok(())
    }

    async fn list_adapters(&self) -> Result<Vec<String>, ProviderError> {
        let map = self
            .jobs
            .lock()
            .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
        Ok(map.keys().cloned().collect())
    }

    async fn delete_adapter(&self, adapter_id: &str) -> Result<(), ProviderError> {
        let _ = self.cancel(adapter_id).await;
        Ok(())
    }

    async fn completion_metadata(
        &self,
        _job_id: &str,
    ) -> Result<Option<CompletionMetadata>, ProviderError> {
        // Baseten checkpoints contain metrics; extraction requires checkpoint API.
        Ok(None)
    }

    async fn adapter_weight_path(
        &self,
        _adapter_id: &str,
    ) -> Result<Option<PathBuf>, ProviderError> {
        // Weights are on Baseten — download via checkpoint archive URL.
        Ok(None)
    }
}

// ── Host factory ───────────────────────────────────────────────────────────

/// Create a training host from configuration.
///
/// Reads `training.host` and `training.harness` from hKask settings
/// (via hkask-services config). The harness selects the tooling (Axolotl/Unsloth),
/// the host selects where the compute runs (Together/Runpod/Baseten or local).
/// Default: Axolotl harness on Together host.
pub fn create_host(config: &TrainingHostConfig) -> Result<Box<dyn TrainingHost>, ProviderError> {
    match (&config.harness, &config.host) {
        (TrainingHarnessId::Axolotl, TrainingHostId::Together) => {
            if config.together_api_key.is_empty() {
                return Err(ProviderError::Unavailable(
                    "Together AI API key not configured (set TOGETHER_API_KEY)".to_string(),
                ));
            }
            Ok(Box::new(TogetherProvider::new(
                config.together_api_key.clone(),
            )))
        }
        (TrainingHarnessId::Axolotl, TrainingHostId::Runpod) => {
            if config.runpod_api_key.is_empty() {
                return Err(ProviderError::Unavailable(
                    "Runpod API key not configured (set RUNPOD_API_KEY)".to_string(),
                ));
            }
            if config.runpod_template_id.is_empty() {
                return Err(ProviderError::Unavailable(
                    "Runpod template ID not configured (set RUNPOD_TEMPLATE_ID)".to_string(),
                ));
            }
            Ok(Box::new(RunpodProvider::new(
                config.runpod_api_key.clone(),
                config.runpod_template_id.clone(),
            )))
        }
        (TrainingHarnessId::Unsloth, TrainingHostId::Together) => {
            if config.together_api_key.is_empty() {
                return Err(ProviderError::Unavailable(
                    "Together AI API key not configured (set TOGETHER_API_KEY)".to_string(),
                ));
            }
            Ok(Box::new(TogetherProvider::new(
                config.together_api_key.clone(),
            )))
        }
        (TrainingHarnessId::Unsloth, TrainingHostId::Runpod) => {
            if config.runpod_api_key.is_empty() {
                return Err(ProviderError::Unavailable(
                    "Runpod API key not configured (set RUNPOD_API_KEY)".to_string(),
                ));
            }
            if config.runpod_template_id.is_empty() {
                return Err(ProviderError::Unavailable(
                    "Runpod template ID not configured (set RUNPOD_TEMPLATE_ID)".to_string(),
                ));
            }
            Ok(Box::new(RunpodProvider::new(
                config.runpod_api_key.clone(),
                config.runpod_template_id.clone(),
            )))
        }
        (TrainingHarnessId::Axolotl, TrainingHostId::Baseten) => {
            if config.baseten_api_key.is_empty() {
                return Err(ProviderError::Unavailable(
                    "Baseten API key not configured (set BASETEN_API_KEY)".to_string(),
                ));
            }
            if config.baseten_project_id.is_empty() {
                return Err(ProviderError::Unavailable(
                    "Baseten project ID not configured (set BASETEN_PROJECT_ID)".to_string(),
                ));
            }
            Ok(Box::new(BasetenProvider::new(
                config.baseten_api_key.clone(),
                config.baseten_project_id.clone(),
            )))
        }
        (TrainingHarnessId::Unsloth, TrainingHostId::Baseten) => {
            if config.baseten_api_key.is_empty() {
                return Err(ProviderError::Unavailable(
                    "Baseten API key not configured (set BASETEN_API_KEY)".to_string(),
                ));
            }
            if config.baseten_project_id.is_empty() {
                return Err(ProviderError::Unavailable(
                    "Baseten project ID not configured (set BASETEN_PROJECT_ID)".to_string(),
                ));
            }
            Ok(Box::new(BasetenProvider::new(
                config.baseten_api_key.clone(),
                config.baseten_project_id.clone(),
            )))
        }
    }
}

/// Training host configuration resolved from hKask settings.
///
/// Combines a harness (tooling choice: Axolotl or Unsloth) with a host
/// (compute location: Together/Runpod/Baseten or local). The harness runs
/// on the host — this is the *harness* layer distinct from the *host* layer.
#[derive(Debug, Clone)]
pub struct TrainingHostConfig {
    /// Selected training harness (Axolotl or Unsloth).
    pub harness: TrainingHarnessId,
    /// Selected training host (Together, Runpod, Baseten, or local).
    pub host: TrainingHostId,
    /// Path to axolotl CLI binary (for Axolotl harness).
    pub axolotl_path: Option<PathBuf>,
    /// Path to python3 interpreter (for Unsloth harness).
    pub python_path: Option<PathBuf>,
    /// Together AI API key (for Together host).
    pub together_api_key: String,
    /// Runpod API key (for Runpod host).
    pub runpod_api_key: String,
    /// Runpod GPU pod template ID with axolotl pre-installed (for Runpod host).
    pub runpod_template_id: String,
    /// Baseten API key (for Baseten host).
    pub baseten_api_key: String,
    /// Baseten training project ID (for Baseten host).
    pub baseten_project_id: String,
}

impl Default for TrainingHostConfig {
    fn default() -> Self {
        Self {
            harness: TrainingHarnessId::Axolotl,
            host: TrainingHostId::Together,
            axolotl_path: None,
            python_path: None,
            together_api_key: String::new(),
            runpod_api_key: String::new(),
            runpod_template_id: String::new(),
            baseten_api_key: String::new(),
            baseten_project_id: String::new(),
        }
    }
}

// ── Training host router ──────────────────────────────────────────────────

/// Host multiplexer with fallback semantics.
///
/// Dispatches training jobs across cloud hosts (Runpod, Baseten, Together)
/// and local hosts (Axolotl, Unsloth) in sequence.
/// When the primary host is unavailable, the router cascades to a local
/// provider as fallback. This is the *host* layer — distinct from the
/// *harness* layer (Axolotl/Unsloth tooling) and the *base model* layer.
pub struct TrainingHostRouter {
    hosts: Vec<Box<dyn TrainingHost>>,
}

impl TrainingHostRouter {
    /// Build a fallback chain from a host config.
    ///
    /// Constructs the primary host then adds a local provider as fallback.
    /// The primary is always constructed first; the local fallback is added
    /// unconditionally.
    pub fn from_config(config: &TrainingHostConfig) -> Result<Self, ProviderError> {
        let mut hosts: Vec<Box<dyn TrainingHost>> = Vec::new();

        let primary = create_host(config)?;
        hosts.push(primary);

        // Add local fallback: if primary is cloud (Together/Runpod/Baseten),
        // also construct a local provider as backup via AxolotlProvider or UnslothProvider.
        match config.harness {
            TrainingHarnessId::Axolotl => {
                let harness: Box<dyn HarnessAdapter> = Box::new(AxolotlHarness);
                let local = AxolotlProvider::new(config.axolotl_path.clone(), harness);
                hosts.push(Box::new(local));
            }
            TrainingHarnessId::Unsloth => {
                let harness: Box<dyn HarnessAdapter> = Box::new(UnslothHarness);
                let local = UnslothProvider::new(config.python_path.clone(), harness);
                hosts.push(Box::new(local));
            }
        }

        Ok(Self { hosts })
    }
}

#[async_trait::async_trait]
impl TrainingHost for TrainingHostRouter {
    async fn submit(&self, job: &TrainingJob) -> Result<String, ProviderError> {
        let mut last_err = ProviderError::Unavailable("no hosts configured".to_string());
        for host in &self.hosts {
            match host.submit(job).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!("host cascade failed: {}", e);
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    async fn status(&self, job_id: &str) -> Result<TrainingJobStatus, ProviderError> {
        let mut last_err = ProviderError::Unavailable("no hosts configured".to_string());
        for host in &self.hosts {
            match host.status(job_id).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!("host cascade failed: {}", e);
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    async fn cancel(&self, job_id: &str) -> Result<(), ProviderError> {
        let mut last_err = ProviderError::Unavailable("no hosts configured".to_string());
        for host in &self.hosts {
            match host.cancel(job_id).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!("host cascade failed: {}", e);
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    async fn list_adapters(&self) -> Result<Vec<String>, ProviderError> {
        let mut last_err = ProviderError::Unavailable("no hosts configured".to_string());
        for host in &self.hosts {
            match host.list_adapters().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!("host cascade failed: {}", e);
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    async fn delete_adapter(&self, adapter_id: &str) -> Result<(), ProviderError> {
        let mut last_err = ProviderError::Unavailable("no hosts configured".to_string());
        for host in &self.hosts {
            match host.delete_adapter(adapter_id).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!("host cascade failed: {}", e);
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    async fn completion_metadata(
        &self,
        job_id: &str,
    ) -> Result<Option<CompletionMetadata>, ProviderError> {
        let mut last_err = ProviderError::Unavailable("no hosts configured".to_string());
        for host in &self.hosts {
            match host.completion_metadata(job_id).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!("host cascade failed: {}", e);
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    async fn adapter_weight_path(
        &self,
        adapter_id: &str,
    ) -> Result<Option<PathBuf>, ProviderError> {
        let mut last_err = ProviderError::Unavailable("no hosts configured".to_string());
        for host in &self.hosts {
            match host.adapter_weight_path(adapter_id).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!("host cascade failed: {}", e);
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    async fn download_adapter(
        &self,
        adapter_id: &str,
        cache_dir: &std::path::Path,
    ) -> Result<Option<PathBuf>, ProviderError> {
        let mut last_err = ProviderError::Unavailable("no hosts configured".to_string());
        for host in &self.hosts {
            match host.download_adapter(adapter_id, cache_dir).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!("host cascade failed: {}", e);
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    async fn estimate_cost(&self, job: &TrainingJob) -> CostEstimate {
        for host in &self.hosts {
            let estimate = host.estimate_cost(job).await;
            if estimate.estimated_dollars > 0.0 {
                return estimate;
            }
        }
        CostEstimate::default()
    }
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
