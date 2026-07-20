//! Training provider abstraction — pluggable backend adapter for model fine-tuning.
//!
//! Each provider wraps a different training framework behind a common
//! `TrainingHost` trait. The MCP server maps its tool surface (`submit`,
//! `status`, `cancel`) to provider methods, isolating the MCP surface from
//! host-specific API differences.
//!
//! Architecture (cloud-only — no local training):
//!   TrainingHostConfig × HarnessAdapter → cloud Host → TrainingJob
//!
//! Provider selection is driven by `training.harness` in settings.json,
//! routed through `hkask-services` shared config init. The host is fixed
//! to Runpod (cloud-only, single host).

pub mod harness;
pub mod runpod;
pub mod types;

// ── Re-exports for lib.rs compatibility ──────────────────────────────────

pub use harness::{AxolotlHarness, HarnessAdapter};
pub use runpod::RunpodHost;
pub use types::{
    AdvancedParams, CompletionMetadata, LoraParams, OptimizationParams, ProviderError,
    QuantizationParams, SequenceParams, TrainingHarnessId, TrainingHost, TrainingHostId,
    TrainingJob, TrainingJobStatus, TrainingParams,
};

// ── Host factory ───────────────────────────────────────────────────────────

/// Create a training host from configuration and a pre-built harness.
///
/// The harness selects the tooling (Axolotl). The host is fixed to
/// Runpod — the only cloud host. The harness is injected into the host at
/// construction.
pub fn create_host(
    config: &TrainingHostConfig,
    harness: Box<dyn HarnessAdapter>,
) -> Result<Box<dyn TrainingHost>, ProviderError> {
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

// ── Training host config ──────────────────────────────────────────────────

/// Training host configuration resolved from hKask settings.
///
/// Selects the Runpod cloud host with API credentials. The harness is
/// selected separately and injected into the host at construction time —
/// this config only describes *where* compute runs.
#[derive(Debug, Clone)]
pub struct TrainingHostConfig {
    /// Selected training host (always Runpod — kept for future extensibility).
    pub host: TrainingHostId,
    /// Runpod API key.
    pub runpod_api_key: String,
    /// Runpod GPU pod template ID with axolotl pre-installed.
    pub runpod_template_id: String,
}

impl Default for TrainingHostConfig {
    fn default() -> Self {
        Self {
            host: TrainingHostId::Runpod,
            runpod_api_key: String::new(),
            runpod_template_id: String::new(),
        }
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
        assert_eq!(TrainingHarnessId::from_str("unknown"), None);
    }

    #[test]
    fn host_id_from_str() {
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
        let cost = types::estimate_training_cost_urj(&TrainingHostId::Runpod, 3, "Qwen3:8b");
        assert_eq!(cost, 1_500_000);
        let cost = types::estimate_training_cost_urj(&TrainingHostId::Runpod, 2, "Llama-3.3-70B");
        assert_eq!(cost, 4_000_000);
    }

    #[test]
    fn training_job_new_has_valid_defaults() {
        let params = TrainingParams::default();
        let job = TrainingJob::new(
            std::path::PathBuf::from("/tmp/test.jsonl"),
            "Qwen3:8b".into(),
            params,
            TrainingHostId::Runpod,
            TrainingHarnessId::Axolotl,
        );
        assert!(!job.id.is_empty());
        assert_eq!(job.base_model, "Qwen3:8b");
        assert_eq!(job.host, TrainingHostId::Runpod);
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
    fn host_config_default() {
        let config = TrainingHostConfig::default();
        assert_eq!(config.host, TrainingHostId::Runpod);
    }
}
