//! Training provider abstraction — cloud host + harness + base model triple.
//!
//! Host (where) × Harness (how) × BaseModel (what) = Training capability.
//!
//! Host → Harness compatibility:
//!   Together AI → Axolotl
//!   Runpod      → Axolotl
//!   Baseten     → Axolotl or Unsloth

mod baseten;
pub mod harness;
mod runpod;
mod together;
pub mod types;

use baseten::BasetenProvider;
use runpod::RunpodProvider;
use together::TogetherProvider;
use types::*;

pub use harness::{AxolotlHarness, HarnessAdapter, HarnessCapability, UnslothHarness};
pub(crate) use types::estimate_training_cost_urj;
pub use types::{
    AdvancedParams, CompletionMetadata, CostEstimate, LoraParams, OptimizationParams,
    ProviderError, QuantizationParams, SequenceParams, TrainingHarnessId, TrainingHost,
    TrainingHostId, TrainingJob, TrainingJobStatus, TrainingParams,
};

// ── Host factory (host + harness → TrainingHost) ──────────────────────────

pub fn create_host(
    config: &TrainingHostConfig,
    harness: Box<dyn HarnessAdapter>,
) -> Result<Box<dyn TrainingHost>, ProviderError> {
    match config.host {
        TrainingHostId::Together => {
            if config.together_api_key.is_empty() {
                return Err(ProviderError::Unavailable(
                    "Together AI API key not configured (set TOGETHER_API_KEY)".into(),
                ));
            }
            Ok(Box::new(TogetherProvider::new(
                config.together_api_key.clone(),
                harness,
            )))
        }
        TrainingHostId::Runpod => {
            if config.runpod_api_key.is_empty() || config.runpod_template_id.is_empty() {
                return Err(ProviderError::Unavailable(
                    "Runpod API key or template ID not configured".into(),
                ));
            }
            Ok(Box::new(RunpodProvider::new(
                config.runpod_api_key.clone(),
                config.runpod_template_id.clone(),
                harness,
            )))
        }
        TrainingHostId::Baseten => {
            if config.baseten_api_key.is_empty() || config.baseten_project_id.is_empty() {
                return Err(ProviderError::Unavailable(
                    "Baseten API key or project ID not configured".into(),
                ));
            }
            Ok(Box::new(BasetenProvider::new(
                config.baseten_api_key.clone(),
                config.baseten_project_id.clone(),
                harness,
            )))
        }
    }
}

// ── Host config ────────────────────────────────────────────────────────────

pub struct TrainingHostConfig {
    pub harness: TrainingHarnessId,
    pub host: TrainingHostId,
    pub together_api_key: String,
    pub runpod_api_key: String,
    pub runpod_template_id: String,
    pub baseten_api_key: String,
    pub baseten_project_id: String,
}

impl Default for TrainingHostConfig {
    fn default() -> Self {
        Self {
            harness: TrainingHarnessId::Axolotl,
            host: TrainingHostId::Together,
            together_api_key: String::new(),
            runpod_api_key: String::new(),
            runpod_template_id: String::new(),
            baseten_api_key: String::new(),
            baseten_project_id: String::new(),
        }
    }
}

// ── Training host router (single host, no cascade) ────────────────────────

/// Wraps a single TrainingHost selected by config.
/// Delegates all trait methods to the inner host.
pub struct TrainingHostRouter {
    host: Box<dyn TrainingHost>,
}

impl TrainingHostRouter {
    pub fn from_config(config: &TrainingHostConfig) -> Result<Self, ProviderError> {
        let harness: Box<dyn HarnessAdapter> = match config.harness {
            TrainingHarnessId::Axolotl => Box::new(AxolotlHarness),
            TrainingHarnessId::Unsloth => Box::new(UnslothHarness),
        };
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
    ) -> Result<Option<std::path::PathBuf>, ProviderError> {
        self.host.adapter_weight_path(adapter_id).await
    }
    async fn download_adapter(
        &self,
        adapter_id: &str,
        cache_dir: &std::path::Path,
    ) -> Result<Option<std::path::PathBuf>, ProviderError> {
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
    fn host_config_default() {
        let c = TrainingHostConfig::default();
        assert_eq!(c.harness, TrainingHarnessId::Axolotl);
        assert_eq!(c.host, TrainingHostId::Together);
    }

    #[test]
    fn harness_id_from_str() {
        assert_eq!(
            TrainingHarnessId::from_str("axolotl"),
            Some(TrainingHarnessId::Axolotl)
        );
        assert_eq!(TrainingHarnessId::from_str("unknown"), None);
    }

    #[test]
    fn host_id_from_str() {
        assert_eq!(
            TrainingHostId::from_str("together"),
            Some(TrainingHostId::Together)
        );
        assert_eq!(TrainingHostId::from_str("unknown"), None);
    }

    #[test]
    fn model_size_multiplier() {
        assert_eq!(extract_model_size_multiplier("Qwen3:8b"), 1);
        assert_eq!(extract_model_size_multiplier("Llama-3.3-70B"), 4);
        assert_eq!(extract_model_size_multiplier("unknown-model"), 2);
    }

    #[test]
    fn estimate_cost_is_positive() {
        assert_eq!(
            estimate_training_cost_urj(&TrainingHostId::Together, 3, "Qwen3:8b"),
            3_000_000
        );
    }

    #[test]
    fn training_params_default() {
        let p = TrainingParams::default();
        assert_eq!(p.num_epochs, 3);
        assert!(p.batch_size > 0);
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
            assert!(!span.is_empty());
            assert!(span.starts_with("cns."));
        }
    }
}
