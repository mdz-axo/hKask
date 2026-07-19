//! Training provider abstraction — pluggable backend adapter for model fine-tuning.
//!
//! Each provider wraps a different training framework behind a common
//! `TrainingHost` trait. The MCP server maps its tool surface (`submit`,
//! `status`, `cancel`, `list_adapters`, `delete_adapter`) to provider methods,
//! isolating the MCP surface from host-specific API differences.
//!
//! Architecture (cloud-only — no local training):
//!   TrainingHostConfig × HarnessAdapter → cloud Host → TrainingJob
//!
//! Provider selection is driven by `training.host` and `training.harness`
//! in settings.json, routed through `hkask-services` shared config init.

pub mod harness;
pub mod runpod;
pub mod tinker;
pub mod together;
pub mod types;

// ── Re-exports for lib.rs compatibility ──────────────────────────────────

pub use harness::{
    AxolotlHarness, HarnessAdapter, HarnessCapability, TinkerHarness, UnslothHarness,
};
pub use runpod::RunpodHost;
pub use tinker::TinkerHost;
pub use together::TogetherHost;
pub use types::{
    AdvancedParams, CompletionMetadata, CostEstimate, LoraParams, OptimizationParams,
    ProviderError, QuantizationParams, SequenceParams, TrainingHarnessId, TrainingHost,
    TrainingHostId, TrainingJob, TrainingJobStatus, TrainingParams,
};

use std::path::PathBuf;

// ── Host factory ───────────────────────────────────────────────────────────

/// Create a training host from configuration and a pre-built harness.
///
/// The harness selects the tooling (Axolotl/Unsloth). The host selects where
/// compute runs (Together/Runpod). The harness is injected into the
/// host at construction — the caller (lib.rs) selects the harness based on
/// `harness_id` from config.
///
/// Reads `training.host` from hKask settings (via hkask-services config).
/// Default: Axolotl harness on Together host.
pub fn create_host(
    config: &TrainingHostConfig,
    harness: Box<dyn HarnessAdapter>,
) -> Result<Box<dyn TrainingHost>, ProviderError> {
    match config.host {
        TrainingHostId::Together => {
            if config.together_api_key.is_empty() {
                return Err(ProviderError::Unavailable(
                    "Together AI API key not configured (set TG_API_KEY)".to_string(),
                ));
            }
            Ok(Box::new(TogetherHost::new(
                config.together_api_key.clone(),
                harness,
            )))
        }
        TrainingHostId::Runpod => {
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
            Ok(Box::new(RunpodHost::new(
                config.runpod_api_key.clone(),
                config.runpod_template_id.clone(),
                harness,
            )))
        }
        TrainingHostId::Tinker => {
            if config.tinker_api_key.is_empty() {
                return Err(ProviderError::Unavailable(
                    "Tinker API key not configured (set TINKER_API_KEY)".to_string(),
                ));
            }
            Ok(Box::new(TinkerHost::new(
                config.tinker_python_path.clone(),
                harness,
            )))
        }
    }
}

// ── Training host config ──────────────────────────────────────────────────

/// Training host configuration resolved from hKask settings.
///
/// Selects a cloud host (Together/Runpod) or a Tinker subprocess host with
/// API credentials. The harness is selected separately and injected into the
/// host at construction time — this config only describes *where* compute runs.
#[derive(Debug, Clone)]
pub struct TrainingHostConfig {
    /// Selected training host (Together, Runpod, Tinker).
    pub host: TrainingHostId,
    /// Together AI API key (for Together host).
    pub together_api_key: String,
    /// Runpod API key (for Runpod host).
    pub runpod_api_key: String,
    /// Runpod GPU pod template ID with axolotl pre-installed (for Runpod host).
    pub runpod_template_id: String,
    /// Thinking Machines Tinker API key (for Tinker host). Read by the Python
    /// SDK inside the subprocess; stored here only for fail-fast validation.
    pub tinker_api_key: String,
    /// Path to the python3 interpreter with the tinker package installed
    /// (for Tinker host). Empty string falls back to python3 from PATH.
    pub tinker_python_path: String,
}

impl Default for TrainingHostConfig {
    fn default() -> Self {
        Self {
            host: TrainingHostId::Together,
            together_api_key: String::new(),
            runpod_api_key: String::new(),
            runpod_template_id: String::new(),
            tinker_api_key: String::new(),
            tinker_python_path: String::new(),
        }
    }
}

// ── Training host router ──────────────────────────────────────────────────

/// Host wrapper — dispatches training jobs to a single cloud host.
///
/// Cloud-only deployment with no fallback chain. If the host is unavailable,
/// the operation fails gracefully.
pub struct TrainingHostRouter {
    host: Box<dyn TrainingHost>,
}

impl TrainingHostRouter {
    /// Build a router from host config and a pre-built harness.
    ///
    /// Constructs exactly one cloud host — no local fallback.
    pub fn from_config(
        config: &TrainingHostConfig,
        harness: Box<dyn HarnessAdapter>,
    ) -> Result<Self, ProviderError> {
        let host = create_host(config, harness)?;
        Ok(Self { host })
    }
}

#[async_trait::async_trait]
impl TrainingHost for TrainingHostRouter {
    async fn submit(&self, job: &TrainingJob) -> Result<String, ProviderError> {
        self.host.submit(job).await
    }

    async fn status(&self, job_id: &str) -> Result<TrainingJobStatus, ProviderError> {
        self.host.status(job_id).await
    }

    async fn cancel(&self, job_id: &str) -> Result<(), ProviderError> {
        self.host.cancel(job_id).await
    }

    async fn list_adapters(&self) -> Result<Vec<String>, ProviderError> {
        self.host.list_adapters().await
    }

    async fn delete_adapter(&self, adapter_id: &str) -> Result<(), ProviderError> {
        self.host.delete_adapter(adapter_id).await
    }

    async fn completion_metadata(
        &self,
        job_id: &str,
    ) -> Result<Option<CompletionMetadata>, ProviderError> {
        self.host.completion_metadata(job_id).await
    }

    async fn adapter_weight_path(
        &self,
        adapter_id: &str,
    ) -> Result<Option<PathBuf>, ProviderError> {
        self.host.adapter_weight_path(adapter_id).await
    }

    async fn download_adapter(
        &self,
        adapter_id: &str,
        cache_dir: &std::path::Path,
    ) -> Result<Option<PathBuf>, ProviderError> {
        self.host.download_adapter(adapter_id, cache_dir).await
    }

    async fn estimate_cost(&self, job: &TrainingJob) -> CostEstimate {
        self.host.estimate_cost(job).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_id_from_str() {
        assert_eq!(
            TrainingHarnessId::from_str("axolotl"),
            Some(TrainingHarnessId::Axolotl)
        );
        assert_eq!(
            TrainingHarnessId::from_str("AXOLOTL"),
            Some(TrainingHarnessId::Axolotl)
        );
        assert_eq!(
            TrainingHarnessId::from_str("unsloth"),
            Some(TrainingHarnessId::Unsloth)
        );
        assert_eq!(TrainingHarnessId::from_str("unknown"), None);
    }

    #[test]
    fn host_id_from_str() {
        assert_eq!(
            TrainingHostId::from_str("together"),
            Some(TrainingHostId::Together)
        );
        assert_eq!(
            TrainingHostId::from_str("runpod"),
            Some(TrainingHostId::Runpod)
        );
        assert_eq!(TrainingHostId::from_str("unknown"), None);
    }

    #[test]
    fn model_size_multiplier() {
        assert_eq!(types::extract_model_size_multiplier("Qwen3:8b"), 1);
        assert_eq!(types::extract_model_size_multiplier("Llama-3.3-70B"), 4);
        assert_eq!(types::extract_model_size_multiplier("Mixtral-8x7b"), 1);
        assert_eq!(types::extract_model_size_multiplier("unknown-model"), 2);
    }

    #[test]
    fn estimate_cost_is_positive() {
        let cost = types::estimate_training_cost_urj(&TrainingHostId::Together, 3, "Qwen3:8b");
        assert_eq!(cost, 3_000_000);
        let cost = types::estimate_training_cost_urj(&TrainingHostId::Runpod, 2, "Llama-3.3-70B");
        assert_eq!(cost, 4_000_000);
    }

    #[test]
    fn training_job_new_has_valid_defaults() {
        let params = TrainingParams::default();
        let job = TrainingJob::new(
            PathBuf::from("/tmp/test.jsonl"),
            "Qwen3:8b".into(),
            params,
            TrainingHostId::Together,
            TrainingHarnessId::Axolotl,
        );
        assert!(!job.id.is_empty());
        assert_eq!(job.base_model, "Qwen3:8b");
        assert_eq!(job.host, TrainingHostId::Together);
        assert_eq!(job.harness, TrainingHarnessId::Axolotl);
        assert_eq!(job.status, TrainingJobStatus::Queued);
        assert!(job.estimated_cost_urj > 0);
    }

    #[test]
    fn lora_params_default() {
        let params = LoraParams::default();
        assert_eq!(params.r, 16);
        assert_eq!(params.alpha, 32);
        assert_eq!(params.dropout, 0.0);
        assert_eq!(params.target_modules.len(), 7);
        assert!(!params.use_rslora);
    }

    #[test]
    fn training_params_default() {
        let params = TrainingParams::default();
        assert_eq!(params.num_epochs, 3);
        assert!(params.batch_size > 0);
    }

    #[test]
    fn harness_capability_cns_spans() {
        use HarnessCapability::*;
        for cap in [
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
        ] {
            let span = cap.cns_span();
            assert!(!span.is_empty(), "{cap:?} has empty CNS span");
            assert!(
                span.starts_with("cns."),
                "{cap:?} span '{span}' doesn't start with cns."
            );
        }
    }

    #[test]
    fn training_job_status_is_serializable() {
        let status = TrainingJobStatus::Queued;
        let json = serde_json::to_string(&status).expect("serialize");
        assert!(json.contains("queued"));
    }

    #[test]
    fn axolotl_harness_output_dir() {
        let harness = AxolotlHarness;
        let path = harness.output_dir("job-123");
        assert!(path.to_string_lossy().contains("job-123"));
    }

    #[test]
    fn unsloth_harness_output_dir() {
        let harness = UnslothHarness;
        let path = harness.output_dir("job-456");
        assert!(path.to_string_lossy().contains("job-456"));
    }

    #[test]
    fn host_config_default() {
        let config = TrainingHostConfig::default();
        assert_eq!(config.host, TrainingHostId::Together);
    }
}
