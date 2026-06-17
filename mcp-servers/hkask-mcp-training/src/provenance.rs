//! HuggingFace model provenance — resolves model identity, fetches model cards,
//! validates license compatibility, and verifies tokenizer configs.
//!
//! This is a *model provenance provider*, not a training host. It doesn't run
//! training — it resolves model identity before training begins.
//!
//! HF's role is **model provenance + dataset provenance**:
//! - Model cards (architecture, license, known limitations)
//! - Dataset cards (source, distribution, bias warnings)
//! - License compatibility checks
//! - Tokenizer configuration verification
//!
//! CNS span: `cns.training.provenance.resolved`

use serde::{Deserialize, Serialize};
use std::fmt;

/// Resolved model provenance — what we know about a model before training.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProvenance {
    /// HuggingFace model ID (org/model).
    pub model_id: String,
    /// Model architecture family (e.g., "llama", "qwen", "gemma", "mistral").
    pub architecture: String,
    /// License identifier (e.g., "apache-2.0", "llama3", "gemma").
    pub license: Option<String>,
    /// Whether the model supports LoRA fine-tuning on this architecture.
    pub lora_compatible: bool,
    /// Tokenizer class name (e.g., "LlamaTokenizerFast").
    pub tokenizer_type: Option<String>,
    /// Maximum sequence length from model config.
    pub max_sequence_length: Option<u32>,
    /// Model parameter count in billions (approximate).
    pub param_count_b: Option<f32>,
    /// Whether the model is gated (requires approval).
    pub is_gated: bool,
}

impl ModelProvenance {
    /// Construct a resolved provenance entry for a known model.
    pub fn known(model_id: &str) -> Option<Self> {
        let (org, model) = model_id.split_once('/')?;
        let model_lower = model.to_lowercase();

        // Known model registry — architecture, license, LoRA compatibility.
        // This is a static fallback; the preferred path is API resolution.
        let (arch, license, lora_ok, tokenizer, max_seq, params, gated) =
            if model_lower.contains("llama") {
                (
                    "llama",
                    Some("llama3"),
                    true,
                    Some("LlamaTokenizerFast"),
                    Some(8192),
                    Some(8.0),
                    org == "meta-llama",
                )
            } else if model_lower.contains("mistral") {
                (
                    "mistral",
                    Some("apache-2.0"),
                    true,
                    Some("LlamaTokenizerFast"),
                    Some(32768),
                    Some(7.0),
                    false,
                )
            } else if model_lower.contains("qwen") {
                (
                    "qwen",
                    Some("apache-2.0"),
                    true,
                    Some("Qwen2TokenizerFast"),
                    Some(32768),
                    Some(7.0),
                    false,
                )
            } else if model_lower.contains("gemma") {
                (
                    "gemma",
                    Some("gemma"),
                    true,
                    Some("GemmaTokenizerFast"),
                    Some(8192),
                    Some(9.0),
                    org == "google",
                )
            } else if model_lower.contains("phi") {
                (
                    "phi",
                    Some("mit"),
                    true,
                    Some("AutoTokenizer"),
                    Some(4096),
                    Some(3.8),
                    false,
                )
            } else if model_lower.contains("deepseek") {
                (
                    "deepseek",
                    Some("mit"),
                    true,
                    Some("LlamaTokenizerFast"),
                    Some(16384),
                    Some(16.0),
                    false,
                )
            } else if model_lower.contains("yi") {
                (
                    "yi",
                    Some("apache-2.0"),
                    true,
                    Some("LlamaTokenizerFast"),
                    Some(4096),
                    Some(6.0),
                    false,
                )
            } else if model_lower.contains("falcon") {
                (
                    "falcon",
                    Some("apache-2.0"),
                    true,
                    Some("AutoTokenizer"),
                    Some(2048),
                    Some(7.0),
                    false,
                )
            } else {
                // Unknown model — conservative defaults
                ("unknown", None, true, None, None, None, false)
            };

        Some(Self {
            model_id: model_id.to_string(),
            architecture: arch.to_string(),
            license: license.map(|s| s.to_string()),
            lora_compatible: lora_ok,
            tokenizer_type: tokenizer.map(|s| s.to_string()),
            max_sequence_length: max_seq,
            param_count_b: params,
            is_gated: gated,
        })
    }
}

impl fmt::Display for ModelProvenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}, {})",
            self.model_id,
            self.architecture,
            self.license.as_deref().unwrap_or("unknown license")
        )?;
        if self.is_gated {
            write!(f, " [GATED]")?;
        }
        if let Some(params) = self.param_count_b {
            write!(f, " ~{:.1}B params", params)?;
        }
        if let Some(max_seq) = self.max_sequence_length {
            write!(f, " ctx={}", max_seq)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceError {
    /// Model ID is not a valid HF identifier.
    InvalidModelId(String),
    /// Model is gated and requires authentication.
    GatedModel(String),
    /// Model is not found on HuggingFace Hub.
    NotFound(String),
    /// License is incompatible with training.
    IncompatibleLicense(String),
}

impl fmt::Display for ProvenanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProvenanceError::InvalidModelId(id) => write!(f, "Invalid model ID: {}", id),
            ProvenanceError::GatedModel(id) => write!(f, "Model is gated: {}", id),
            ProvenanceError::NotFound(id) => write!(f, "Model not found: {}", id),
            ProvenanceError::IncompatibleLicense(msg) => write!(f, "License incompatible: {}", msg),
        }
    }
}

/// ModelResolver — resolves model identity and provenance before training.
///
/// Implementations query the HuggingFace Hub (or a local registry) to verify
/// that a model exists, is accessible, and has a compatible license for fine-tuning.
pub trait ModelResolver: Send + Sync {
    /// Resolve a model ID to its provenance.
    fn resolve(&self, model_id: &str) -> Result<ModelProvenance, ProvenanceError>;

    /// Check whether a model ID is valid (format + existence).
    fn validate(&self, model_id: &str) -> bool {
        self.resolve(model_id).is_ok()
    }
}

/// Static model resolver — uses the built-in known-model registry.
///
/// This is a local-only resolver. For full HF Hub resolution (model cards,
/// gating checks, live license verification), use `HuggingFaceResolver`
/// which calls the HF Hub API.
pub struct LocalModelResolver;

impl ModelResolver for LocalModelResolver {
    fn resolve(&self, model_id: &str) -> Result<ModelProvenance, ProvenanceError> {
        if !model_id.contains('/') {
            return Err(ProvenanceError::InvalidModelId(model_id.to_string()));
        }

        ModelProvenance::known(model_id)
            .ok_or_else(|| ProvenanceError::NotFound(model_id.to_string()))
    }
}
