//! hKask Inference — multi-provider inference router
//!
//! Routes LLM requests to DeepInfra (cloud), fal.ai (cloud),
//! or Together AI (cloud) based on a 2-letter provider prefix in the model name.
//!
//! # Architecture
//!
//! ```text
//! InferenceRouter (implements InferencePort)
//!   ├── DeepInfraBackend — DI/ prefix → api.deepinfra.com
//!   ├── FalBackend       — FA/ prefix → api.fal.ai
//!   └── TogetherBackend  — TG/ prefix → api.together.xyz
//!
//! EmbeddingRouter
//!   └── DeepInfraEmbedding — DI/ prefix → /v1/embeddings
//! ```
//!
//! # Model Naming
//!
//! - `DI/meta-llama/Llama-3.3-70B-Instruct` → DeepInfra
//! - `TG/Qwen/Qwen2.5-7B-Instruct-Turbo` → Together AI
//! - `FA/paddleocr` → fal.ai
//! - No prefix → default provider (configurable, default: DeepInfra)


pub mod chat_protocol;
pub mod config;
pub mod deepinfra_backend;
pub mod embedding_router;
pub mod fal_backend;
pub mod inference_router;
pub mod together_backend;

// Re-exports — public API
pub use config::{InferenceConfig, ProviderId};
pub use embedding_router::EmbeddingRouter;
pub use inference_router::InferenceRouter;

// Model listing types
pub use deepinfra_backend::DeepInfraModelEntry;
pub use fal_backend::FalModelEntry;
pub use together_backend::TogetherModel;

/// Unified model entry from any provider, with provider prefix applied.
#[derive(Debug, Clone)]
pub struct RouterModelEntry {
    /// Full model name with provider prefix (e.g., "OM/qwen3:8b")
    pub prefixed_name: String,
    /// Provider this model belongs to
    pub provider: ProviderId,
    /// Raw model name without prefix
    pub model: String,
    /// Model family (e.g., "llama", "qwen2")
    pub family: Option<String>,
    /// Parameter count (e.g., "8B", "70B")
    pub parameter_size: Option<String>,
    /// Quantization level (e.g., "Q4_0")
    pub quantization_level: Option<String>,
    /// Model size in bytes (if available)
    pub size_bytes: Option<u64>,
    /// Whether the model supports vision/multimodal input.
    /// Populated via heuristic on model family name (not runtime probing).
    pub supports_vision: Option<bool>,
}

impl RouterModelEntry {
    /// Construct a RouterModelEntry from a provider and model id.
    ///
    /// expect: "The system heuristically routes multimodal models"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — canonical model entry construction
    /// pre:  model_id is non-empty
    /// post: returns RouterModelEntry with prefixed name, provider, and inferred vision support
    fn from_model_entry(provider: ProviderId, model_id: &str) -> Self {
        Self {
            prefixed_name: provider.prefix_model(model_id),
            provider,
            model: model_id.to_string(),
            supports_vision: Self::infer_vision_support(model_id, None),
            family: None,
            parameter_size: None,
            quantization_level: None,
            size_bytes: None,
        }
    }

    /// Heuristic: known vision-capable model families.
    ///
    /// Checks model name and family against a static allowlist.
    /// Does not perform runtime probing — fast but incomplete.
    /// `None` means unknown; `Some(true)` means likely vision-capable.
    ///
    /// expect: "The system heuristically routes multimodal models"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — heuristic routing for multimodal models
    /// pre:  model is non-empty
    /// post: returns Some(true) if model/family matches known vision families
    /// post: returns None if unknown
    pub fn infer_vision_support(model: &str, family: Option<&str>) -> Option<bool> {
        const VISION_FAMILIES: &[&str] = &[
            "llava",
            "bakllava",
            "minicpm-v",
            "gemma3",
            "llama3.2-vision",
            "cogvlm",
            "moondream",
            "pixtral",
            "florence",
            "paligemma",
            "qwen2-vl",
            "internvl",
            "phi-3-vision",
            "lighton",
            "paddleocr",
            "nemotron-parse",
        ];

        let model_lower = model.to_lowercase();
        let family_lower = family.map(|f| f.to_lowercase());

        for vf in VISION_FAMILIES {
            if model_lower.contains(vf) {
                return Some(true);
            }
            if let Some(ref fam) = family_lower
                && fam.contains(vf)
            {
                return Some(true);
            }
        }

        // No match — unknown (not confirmed false, just unknown)
        None
    }
}
