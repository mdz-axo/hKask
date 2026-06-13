//! hKask Inference — multi-provider inference router
//!
//! Routes LLM requests to Ollama (local), Fireworks.ai (cloud), or DeepInfra (cloud)
//! based on a 2-letter provider prefix in the model name.
//!
//! # Architecture
//!
//! ```text
//! InferenceRouter (implements InferencePort)
//!   ├── OllamaBackend    — OM/ prefix → localhost:11434
//!   ├── FireworksBackend — FW/ prefix → api.fireworks.ai
//!   └── DeepInfraBackend — DI/ prefix → api.deepinfra.com
//!
//! EmbeddingRouter
//!   ├── OllamaEmbedding    — OM/ prefix → /api/embed
//!   ├── FireworksEmbedding — FW/ prefix → /v1/embeddings
//!   └── DeepInfraEmbedding — DI/ prefix → /v1/embeddings
//! ```
//!
//! # Model Naming
//!
//! - `OM/qwen3:8b` → Ollama
//! - `FW/llama-v3p1-70b-instruct` → Fireworks
//! - `DI/meta-llama/Llama-3.3-70B-Instruct` → DeepInfra
//! - No prefix → default provider (configurable, default: Ollama)

pub mod chat_protocol;
pub mod config;
pub mod deepinfra_backend;
pub mod embedding_router;
pub mod fireworks_backend;
pub mod inference_router;
pub mod ollama_backend;

// Re-exports — public API
pub use config::{InferenceConfig, ProviderId};
pub use embedding_router::EmbeddingRouter;
pub use inference_router::InferenceRouter;

// Model listing types
pub use deepinfra_backend::DeepInfraModelEntry;
pub use fireworks_backend::FireworksModelEntry;
pub use ollama_backend::OllamaModelEntry;

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
    /// Heuristic: known vision-capable model families.
    ///
    /// Checks model name and family against a static allowlist.
    /// Does not perform runtime probing — fast but incomplete.
    /// `None` means unknown; `Some(true)` means likely vision-capable.
    ///
    /// Families: llava, bakllava, minicpm-v, gemma3, llama3.2-vision,
    /// cogvlm, moondream, pixtral, florence, paligemma, qwen2-vl,
    /// internvl, phi-3-vision, lighton
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
