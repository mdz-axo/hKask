//! HuggingFace infrastructure traits — model registry, adapter registry, dataset registry.
//!
//! These are *infrastructure* traits, not a 4th training host. They enhance the
//! existing hosts (Together, Runpod, Baseten) transparently, providing:
//! - Model resolution (provider-prefix → HF model ID)
//! - Adapter publication/pull (Together publishes to HF, Baseten pulls from HF)
//! - Dataset remote sourcing (hf://datasets/ URLs)
//!
//! MDS categories:
//! - ModelRegistry → Domain entity: ModelSource with hf:// URI scheme
//! - AdapterRegistry → Lifecycle entity: AdapterPublication
//! - DatasetRegistry → Domain entity: DatasetSource

use hkask_rsolidity::contract;

use std::path::{Path, PathBuf};

// ── HuggingFace error ─────────────────────────────────────────────────────
#[derive(Debug, thiserror::Error)]
pub enum HuggingFaceError {
    #[error("HuggingFace API error: {0}")]
    Api(String),
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("Adapter not found: {0}")]
    AdapterNotFound(String),
    #[error("Dataset not found: {0}")]
    DatasetNotFound(String),
    #[error("Download failed: {0}")]
    Download(String),
    #[error("Authentication failed (set HF_TOKEN)")]
    AuthRequired,
}

/// Resolves and downloads base models from HuggingFace Hub.
///
/// Used by: BasetenProvider (hf:// mount for model loading)
///
/// MDS: Domain entity — `ModelSource` with `hf://` URI scheme.
/// Composition: `CAN resolve_model_id|download_weights|list_variants ON ModelSource VIA API`
///
/// pre:  HF_TOKEN set for gated models
/// post: resolved HF model ID or downloaded weight path

#[async_trait::async_trait]
pub trait ModelRegistry: Send + Sync {
    /// Resolve a provider-prefixed base_model to a HuggingFace model ID.
    ///
    /// Strips known prefixes (OM/, DI/, FA/, TG/) from the base_model string.
    /// Returns the raw HF model ID (e.g., "Qwen/Qwen3.5-9B").
    fn resolve_model_id(&self, base_model: &str) -> String;

    /// Download model weights to a local cache directory.
    ///
    /// Uses HF_TOKEN for gated model access.
    /// Returns the path to the downloaded weights.
    async fn download_weights(
        &self,
        hf_model_id: &str,
        cache_dir: &Path,
    ) -> Result<PathBuf, HuggingFaceError>;

    /// List available variants/checkpoints for a model.
    ///
    /// Returns a list of branch/tag names (e.g., ["main", "fp16", "gguf"]).
    async fn list_variants(&self, hf_model_id: &str) -> Result<Vec<String>, HuggingFaceError>;
}

/// Publishes and retrieves LoRA adapters via HuggingFace Hub.
///
/// Used by: TogetherProvider (publishes adapters to mdz-axolotl/* repos)
///          BasetenProvider (pulls adapters from HF for deployment)
///
/// MDS: Lifecycle entity — `AdapterPublication`.
/// Composition: `CAN publish_adapter|pull_adapter ON Adapter VIA API`
///
/// pre:  adapter weights exist (local or remote)
/// post: adapter published to / pulled from HF Hub

#[async_trait::async_trait]
pub trait AdapterRegistry: Send + Sync {
    /// Publish a LoRA adapter to a HuggingFace repository.
    ///
    /// Uploads adapter weights + adapter_config.json to the specified repo.
    /// Creates the repo if it doesn't exist. Requires HF_TOKEN with write access.
    ///
    /// Returns the HF Hub URL of the published adapter.
    async fn publish_adapter(
        &self,
        adapter_id: &str,
        weight_path: &Path,
        hf_repo: &str,
    ) -> Result<String, HuggingFaceError>;

    /// Pull/download a LoRA adapter from HuggingFace to local cache.
    ///
    /// Downloads adapter weights + config from the specified HF repo.
    /// Returns the local path to the downloaded adapter directory.
    async fn pull_adapter(
        &self,
        hf_repo: &str,
        revision: Option<&str>,
        cache_dir: &Path,
    ) -> Result<PathBuf, HuggingFaceError>;
}

/// Resolves and downloads training datasets from HuggingFace Hub.
///
/// Used by: DatasetPipeline (optional remote source for hf://datasets/ URLs)
///
/// MDS: Domain entity — `DatasetSource`.
/// Composition: `CAN resolve_dataset|download_dataset ON DatasetSource VIA API`
///
/// pre:  dataset exists on HF Hub
/// post: resolved dataset URL or downloaded local path

#[async_trait::async_trait]
pub trait DatasetRegistry: Send + Sync {
    /// Resolve a HuggingFace dataset ID to a download URL.
    ///
    /// Accepts hf://datasets/username/dataset-name format or bare HF dataset IDs.
    /// Returns a direct download URL for the dataset.
    fn resolve_dataset(&self, dataset_id: &str) -> Result<String, HuggingFaceError>;

    /// Download a dataset from HuggingFace to a local file.
    ///
    /// Returns the local path to the downloaded dataset.
    async fn download_dataset(
        &self,
        dataset_id: &str,
        cache_dir: &Path,
    ) -> Result<PathBuf, HuggingFaceError>;
}

// ── Default implementation for model ID resolution ─────────────────────────

/// Strip known provider prefixes to extract the raw HuggingFace model ID.
///
/// This is the canonical resolution logic used by BasetenProvider.
/// Provider prefixes: DI/ (DeepInfra), FA/ (fal.ai), TG/ (Together).
    #[contract(id = "P4-trn-hf-dataset-registry", principle = "P4")]
pub fn resolve_model_id(base_model: &str) -> String {
    let known_prefixes = ["DI/", "FA/", "TG/"];
    let mut model = base_model;
    for prefix in &known_prefixes {
        if model.starts_with(prefix) {
            model = &model[prefix.len()..];
            break;
        }
    }
    model.to_string()
}

// ── Reqwest-based ModelRegistry implementation ─────────────────────────────

/// HuggingFace Hub model registry using the HF REST API.
pub struct HfModelRegistry {
    client: reqwest::Client,
    api_key: String,
}

impl HfModelRegistry {
    /// Create a new HuggingFace model registry.
    ///
    /// `api_key` is the HF_TOKEN for gated model access.
    #[contract(id = "P4-trn-hf-adapter-registry", principle = "P4")]
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }
}
#[async_trait::async_trait]
impl ModelRegistry for HfModelRegistry {
    fn resolve_model_id(&self, base_model: &str) -> String {
        resolve_model_id(base_model)
    }

    async fn download_weights(
        &self,
        _hf_model_id: &str,
        _cache_dir: &Path,
    ) -> Result<PathBuf, HuggingFaceError> {
        // Weight download via huggingface_hub Python library or hf_transfer.
        // For now, cloud hosts (Baseten) mount via hf:// directly — no local download needed.
        // Local download would use: huggingface_hub.snapshot_download()
        Err(HuggingFaceError::Download(
            "Direct weight download via REST not implemented — use hf:// mount or huggingface_hub CLI"
                .to_string(),
        ))
    }

    async fn list_variants(&self, hf_model_id: &str) -> Result<Vec<String>, HuggingFaceError> {
        let url = format!("https://huggingface.co/api/models/{}/refs", hf_model_id);
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| HuggingFaceError::Api(format!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(HuggingFaceError::ModelNotFound(hf_model_id.to_string()));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| HuggingFaceError::Api(format!("Parse error: {}", e)))?;

        let branches = json
            .get("branches")
            .and_then(|b| b.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|b| b.get("name").and_then(|n| n.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(branches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: P8-trn-hf-resolve-provider-prefix — resolve_model_id strips known prefixes
    #[test]
    fn resolve_provider_prefix() {
        assert_eq!(resolve_model_id("DI/some-model"), "some-model");
    }

    // REQ: P8-trn-hf-resolve-together-prefix — resolve_model_id strips TG/ prefix
    #[test]
    fn resolve_together_prefix() {
        assert_eq!(resolve_model_id("TG/Qwen/Qwen3.5-9B"), "Qwen/Qwen3.5-9B");
    }

    // REQ: P8-trn-hf-resolve-no-prefix-passthrough — resolve_model_id passes through unprefixed IDs
    #[test]
    fn resolve_no_prefix_passthrough() {
        assert_eq!(resolve_model_id("Qwen/Qwen3.5-9B"), "Qwen/Qwen3.5-9B");
    }

    // REQ: P8-trn-hf-resolve-deepinfra-prefix — resolve_model_id strips DI/ prefix
    #[test]
    fn resolve_deepinfra_prefix() {
        assert_eq!(
            resolve_model_id("DI/meta-llama/Llama-3.3-70B-Instruct"),
            "meta-llama/Llama-3.3-70B-Instruct"
        );
    }

    // REQ: P8-trn-hf-resolve-fireworks-prefix — resolve_model_id strips FA/ prefix
    #[test]
    fn resolve_fireworks_prefix() {
        assert_eq!(
            resolve_model_id("FA/accounts/fireworks/models/my-model"),
            "accounts/fireworks/models/my-model"
        );
    }
}

// ── Model provenance ──────────────────────────────────────────────────────

use serde::Serialize;

/// Resolved model provenance — what we know about a model before training.
#[derive(Debug, Clone, Serialize)]
pub struct ModelProvenance {
    pub model_id: String,
    pub architecture: String,
    pub license: Option<String>,
    pub lora_compatible: bool,
    pub is_gated: bool,
}

/// ModelResolver — resolves model identity and provenance before training.
pub trait ModelResolver: Send + Sync {
    fn resolve(&self, model_id: &str) -> Result<ModelProvenance, HuggingFaceError>;
    fn validate(&self, model_id: &str) -> bool {
        self.resolve(model_id).is_ok()
    }
}

/// Static model resolver using built-in known-model registry.
#[derive(Default)]
pub struct LocalModelResolver;

impl ModelResolver for LocalModelResolver {
    fn resolve(&self, model_id: &str) -> Result<ModelProvenance, HuggingFaceError> {
        if !model_id.contains('/') {
            return Err(HuggingFaceError::ModelNotFound(model_id.to_string()));
        }
        let (org, model) = model_id
            .split_once('/')
            .ok_or_else(|| HuggingFaceError::ModelNotFound(model_id.to_string()))?;
        let model_lower = model.to_lowercase();
        let (arch, license, lora_ok, gated) = if model_lower.contains("llama") {
            ("llama", Some("llama3"), true, org == "meta-llama")
        } else if model_lower.contains("mistral") {
            ("mistral", Some("apache-2.0"), true, false)
        } else if model_lower.contains("qwen") {
            ("qwen", Some("apache-2.0"), true, false)
        } else if model_lower.contains("gemma") {
            ("gemma", Some("gemma"), true, org == "google")
        } else if model_lower.contains("phi") {
            ("phi", Some("mit"), true, false)
        } else if model_lower.contains("deepseek") {
            ("deepseek", Some("mit"), true, false)
        } else if model_lower.contains("yi") {
            ("yi", Some("apache-2.0"), true, false)
        } else {
            ("unknown", None, true, false)
        };
        Ok(ModelProvenance {
            model_id: model_id.to_string(),
            architecture: arch.to_string(),
            license: license.map(|s| s.to_string()),
            lora_compatible: lora_ok,
            is_gated: gated,
        })
    }
}
