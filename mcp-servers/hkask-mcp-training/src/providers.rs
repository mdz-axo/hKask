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
use std::path::PathBuf;
use thiserror::Error;

// ── Provider identifiers ───────────────────────────────────────────────────

/// Supported training provider backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrainingProviderId {
    /// axolotl — configuration-driven fine-tuning framework
    Axolotl,
    /// unsloth — optimized memory-efficient training
    Unsloth,
    /// together — Together AI cloud fine-tuning API
    Together,
}

impl TrainingProviderId {
    /// Parse from a config string (case-insensitive).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "axolotl" => Some(Self::Axolotl),
            "unsloth" => Some(Self::Unsloth),
            "together" => Some(Self::Together),
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
    /// Provider executing this job.
    pub provider: TrainingProviderId,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrainingParams {
    /// Number of training epochs.
    pub num_epochs: u32,
    /// Batch size.
    pub batch_size: u32,
    /// Learning rate.
    pub learning_rate: f32,
    /// LoRA rank (r value). Typical range: 4–64.
    pub lora_r: u32,
    /// LoRA alpha scaling factor.
    pub lora_alpha: u32,
    /// Target modules for LoRA adaptation (e.g., ["q_proj", "v_proj"]).
    pub target_modules: Vec<String>,
}

impl Default for TrainingParams {
    fn default() -> Self {
        Self {
            num_epochs: 3,
            batch_size: 4,
            learning_rate: 2e-4,
            lora_r: 16,
            lora_alpha: 32,
            target_modules: vec![
                "q_proj".to_string(),
                "v_proj".to_string(),
                "k_proj".to_string(),
                "o_proj".to_string(),
            ],
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

// ── TrainingProvider trait ─────────────────────────────────────────────────

/// Pluggable training backend interface.
///
/// Implementations translate canonical `TrainingJob` representations into
/// provider-specific API calls (CLI execution, remote HTTP dispatch, etc.).
/// The trait is async to accommodate both local subprocess management and
/// cloud provider HTTP calls via `hkask-inference` routing.
#[async_trait::async_trait]
pub trait TrainingProvider: Send + Sync {
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
/// config, writes it to a temp file, and dispatches execution via subprocess
/// or `hkask-inference` cloud routing.
pub struct AxolotlProvider {
    /// Path to axolotl CLI binary or `accelerate launch` wrapper.
    cli_path: Option<PathBuf>,
    /// Whether to use `hkask-inference` cloud dispatch (Fireworks/Baseten)
    /// instead of local subprocess.
    cloud_dispatch: bool,
}

impl AxolotlProvider {
    /// Create a new axolotl provider.
    ///
    /// If `cli_path` is `None`, the provider will attempt to find `axolotl`
    /// on PATH. If not found, falls through to cloud dispatch if configured.
    pub fn new(cli_path: Option<PathBuf>, cloud_dispatch: bool) -> Self {
        Self {
            cli_path,
            cloud_dispatch,
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

    /// Build axolotl YAML config from a canonical TrainingJob.
    fn build_config_yaml(&self, job: &TrainingJob) -> String {
        format!(
            r#"# Auto-generated by hKask Training Server
base_model: {}
datasets:
  - path: {}
    type: chatml
output_dir: ./axolotl-output/{}
num_epochs: {}
batch_size: {}
learning_rate: {}
lora_r: {}
lora_alpha: {}
lora_target_modules:
{}
"#,
            job.base_model,
            job.dataset_path.display(),
            job.id,
            job.params.num_epochs,
            job.params.batch_size,
            job.params.learning_rate,
            job.params.lora_r,
            job.params.lora_alpha,
            job.params
                .target_modules
                .iter()
                .map(|m| format!("  - {}", m))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

#[async_trait::async_trait]
impl TrainingProvider for AxolotlProvider {
    async fn submit(&self, job: &TrainingJob) -> Result<String, ProviderError> {
        if self.cloud_dispatch {
            // Dispatch to cloud (Fireworks/Baseten) via hkask-inference routing.
            // Cloud dispatch sends the canonical job as JSON and receives a
            // provider-specific job ID for status polling.
            return Err(ProviderError::Unavailable(
                "Cloud dispatch not yet implemented for axolotl".to_string(),
            ));
        }

        if !self.available() {
            return Err(ProviderError::Unavailable(
                "axolotl CLI not found. Install with: pip install axolotl".to_string(),
            ));
        }

        let config_yaml = self.build_config_yaml(job);
        let config_path = std::env::temp_dir().join(format!("hkask-training-{}.yaml", job.id));
        std::fs::write(&config_path, &config_yaml).map_err(|e| {
            ProviderError::Backend(format!("Failed to write axolotl config: {}", e))
        })?;

        let cli = self
            .cli_path
            .as_deref()
            .unwrap_or(std::path::Path::new("axolotl"));
        let _status = tokio::process::Command::new(cli)
            .arg("train")
            .arg(&config_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ProviderError::Backend(format!("Failed to spawn axolotl: {}", e)))?;

        tracing::info!(
            target: "cns.training.job.submit",
            job_id = %job.id,
            provider = "axolotl",
            "Training job submitted"
        );

        // Job ID is the hKask job ID — axolotl runs synchronously;
        // async status tracking will poll process exit.
        Ok(job.id.clone())
    }

    async fn status(&self, job_id: &str) -> Result<TrainingJobStatus, ProviderError> {
        // Check if output directory exists and contains a completed checkpoint.
        let output_dir = PathBuf::from(format!("./axolotl-output/{}", job_id));
        if output_dir.join("adapter_model.safetensors").exists() {
            return Ok(TrainingJobStatus::Completed);
        }
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
        // Cancellation is process-level — kill the subprocess if still running.
        // For now, this is a no-op stub; full implementation requires PID tracking.
        tracing::warn!(
            target: "cns.training.job.cancel",
            job_id = %job_id,
            "Axolotl job cancellation is a best-effort stub"
        );
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
}

// ── Unsloth provider ───────────────────────────────────────────────────────

/// Unsloth training provider — wraps unsloth for memory-efficient fine-tuning.
///
/// Unsloth uses Python scripts rather than YAML configs. This provider
/// generates a training script from the canonical TrainingJob and executes it.
pub struct UnslothProvider {
    /// Path to Python interpreter (default: python3).
    python_path: Option<PathBuf>,
    /// Whether to use cloud dispatch.
    cloud_dispatch: bool,
}

impl UnslothProvider {
    /// Create a new unsloth provider.
    pub fn new(python_path: Option<PathBuf>, cloud_dispatch: bool) -> Self {
        Self {
            python_path,
            cloud_dispatch,
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

    /// Generate unsloth training script from canonical TrainingJob.
    fn build_training_script(&self, job: &TrainingJob) -> String {
        let target_modules_str = job
            .params
            .target_modules
            .iter()
            .map(|m| format!("\"{}\"", m))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            r#"# Auto-generated by hKask Training Server
import torch
from unsloth import FastLanguageModel
from datasets import load_dataset

model, tokenizer = FastLanguageModel.from_pretrained(
    model_name="{}",
    max_seq_length=2048,
    load_in_4bit=True,
)

model = FastLanguageModel.get_peft_model(
    model,
    r={},
    lora_alpha={},
    target_modules=[{}],
    lora_dropout=0,
    bias="none",
    use_gradient_checkpointing="unsloth",
)

dataset = load_dataset("json", data_files="{}", split="train")

from transformers import TrainingArguments
from trl import SFTTrainer

trainer = SFTTrainer(
    model=model,
    tokenizer=tokenizer,
    train_dataset=dataset,
    dataset_text_field="text",
    max_seq_length=2048,
    args=TrainingArguments(
        per_device_train_batch_size={},
        num_train_epochs={},
        learning_rate={},
        output_dir="./unsloth-output/{}",
    ),
)
trainer.train()
model.save_pretrained("./unsloth-output/{}/adapter")
"#,
            job.base_model,
            job.params.lora_r,
            job.params.lora_alpha,
            target_modules_str,
            job.dataset_path.display(),
            job.params.batch_size,
            job.params.num_epochs,
            job.params.learning_rate,
            job.id,
            job.id,
        )
    }
}

#[async_trait::async_trait]
impl TrainingProvider for UnslothProvider {
    async fn submit(&self, job: &TrainingJob) -> Result<String, ProviderError> {
        if self.cloud_dispatch {
            return Err(ProviderError::Unavailable(
                "Cloud dispatch not yet implemented for unsloth".to_string(),
            ));
        }

        if !self.available() {
            return Err(ProviderError::Unavailable(
                "unsloth not found. Install with: pip install unsloth".to_string(),
            ));
        }

        let script = self.build_training_script(job);
        let script_path = std::env::temp_dir().join(format!("hkask-training-{}.py", job.id));
        std::fs::write(&script_path, &script).map_err(|e| {
            ProviderError::Backend(format!("Failed to write unsloth script: {}", e))
        })?;

        let py = self
            .python_path
            .as_deref()
            .unwrap_or(std::path::Path::new("python3"));
        tokio::process::Command::new(py)
            .arg(&script_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ProviderError::Backend(format!("Failed to spawn unsloth: {}", e)))?;

        tracing::info!(
            target: "cns.training.job.submit",
            job_id = %job.id,
            provider = "unsloth",
            "Training job submitted"
        );
        Ok(job.id.clone())
    }

    async fn status(&self, job_id: &str) -> Result<TrainingJobStatus, ProviderError> {
        let output_dir = PathBuf::from(format!("./unsloth-output/{}", job_id));
        if output_dir.join("adapter").exists() {
            return Ok(TrainingJobStatus::Completed);
        }
        if output_dir.exists() {
            return Ok(TrainingJobStatus::Running);
        }
        Err(ProviderError::JobFailed(format!(
            "Job {} not found or no output produced",
            job_id
        )))
    }

    async fn cancel(&self, job_id: &str) -> Result<(), ProviderError> {
        tracing::warn!(
            target: "cns.training.job.cancel",
            job_id = %job_id,
            "Unsloth job cancellation is a best-effort stub"
        );
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
}

// ── Together AI cloud provider ──────────────────────────────────────────────

/// Together AI training provider — submits fine-tuning jobs via REST API.
///
/// Uses the Together AI fine-tuning API (https://api.together.xyz/v1/fine-tunes)
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
impl TrainingProvider for TogetherProvider {
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
            "lora_r": job.params.lora_r,
            "lora_alpha": job.params.lora_alpha,
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
            provider = "together",
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
            provider = "together",
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
            provider = "together",
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

// ── Provider factory ───────────────────────────────────────────────────────

/// Create a provider from configuration.
///
/// Reads `training.provider` from hKask settings (via hkask-services config),
/// defaulting to `Axolotl` if unset.
pub fn create_provider(
    config: &ProviderConfig,
) -> Result<Box<dyn TrainingProvider>, ProviderError> {
    match config.provider {
        TrainingProviderId::Axolotl => Ok(Box::new(AxolotlProvider::new(
            config.axolotl_path.clone(),
            config.cloud_dispatch,
        ))),
        TrainingProviderId::Unsloth => Ok(Box::new(UnslothProvider::new(
            config.python_path.clone(),
            config.cloud_dispatch,
        ))),
        TrainingProviderId::Together => {
            if config.together_api_key.is_empty() {
                return Err(ProviderError::Unavailable(
                    "Together AI API key not configured (set TOGETHER_API_KEY)".to_string(),
                ));
            }
            Ok(Box::new(TogetherProvider::new(
                config.together_api_key.clone(),
            )))
        }
    }
}

/// Provider configuration resolved from hKask settings.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Selected training provider.
    pub provider: TrainingProviderId,
    /// Path to axolotl CLI binary (for Axolotl).
    pub axolotl_path: Option<PathBuf>,
    /// Path to python3 interpreter (for Unsloth).
    pub python_path: Option<PathBuf>,
    /// Whether to dispatch training to cloud (Fireworks/Baseten).
    pub cloud_dispatch: bool,
    /// Together AI API key (for Together).
    pub together_api_key: String,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider: TrainingProviderId::Axolotl,
            axolotl_path: None,
            python_path: None,
            cloud_dispatch: false,
            together_api_key: String::new(),
        }
    }
}
