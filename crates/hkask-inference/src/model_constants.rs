//! Model name resolution — env-configurable with compile-time defaults.
//!
//! Every model used in the system has a corresponding env var for override.
//! The constants here are DEFAULT values; env vars take precedence.
//! This eliminates the need to recompile when models are superseded.
//!
//! Naming convention:
//! - `HKASK_CLASSIFIER_MODEL_A` — first peer classifier (dual-model A)
//! - `HKASK_CLASSIFIER_MODEL_B` — second peer classifier (dual-model B)
//! - `HKASK_EMBEDDING_MODEL` — default embedding model
//! - `HKASK_OCR_MODEL` — OCR model for scanned PDF fallback
//! - `HKASK_MODEL_DEFAULT` — fallback when provider-specific not set

/// Fast classifier model for summarization, classification, and non-thinking tasks.
/// DeepInfra hosts the same Qwen3-235B-A22B model and handles high concurrency.
pub const DEFAULT_CLASSIFIER_MODEL: &str = "DI/Qwen/Qwen3-235B-A22B-Instruct-2507";

/// Recommended secondary classifier for dual-model epistemic integrity.
pub const DEFAULT_CLASSIFIER_MODEL_SECONDARY: &str = "DI/google/gemma-4-26B-A4B-it";

/// Default embedding model.
pub const DEFAULT_EMBEDDING_MODEL: &str = "DI/Qwen/Qwen3-Embedding-0.6B";

/// Default OCR model for scanned PDF fallback.
/// Uses kask-ocr on RunPod (OLMOCR-2).
pub const DEFAULT_OCR_MODEL: &str = "RP/kask-ocr";

/// Fallback model when no other model is configured.
pub const DEFAULT_FALLBACK_MODEL: &str = "deepseek-v4-pro";

// ── Test fixtures (arbitrary identifiers, no network calls) ──────────────

pub const TEST_MODEL_SMALL: &str = "DI/google/gemma-4-9b-it";
pub const TEST_MODEL_MEDIUM: &str = "DI/meta-llama/Llama-4-Scout-17B-16E-Instruct";

// ── Resolved model accessors (env var → default) ──────────────────────────

/// Resolve the first peer classifier: `HKASK_CLASSIFIER_MODEL_A` → default.
pub fn classifier_model() -> String {
    std::env::var("HKASK_CLASSIFIER_MODEL_A")
        .unwrap_or_else(|_| DEFAULT_CLASSIFIER_MODEL.to_string())
}

/// Resolve the second peer classifier: `HKASK_CLASSIFIER_MODEL_B` → default.
pub fn classifier_model_secondary() -> String {
    std::env::var("HKASK_CLASSIFIER_MODEL_B")
        .unwrap_or_else(|_| DEFAULT_CLASSIFIER_MODEL_SECONDARY.to_string())
}

/// Resolve the embedding model: `HKASK_EMBEDDING_MODEL` → default.
pub fn embedding_model() -> String {
    std::env::var("HKASK_EMBEDDING_MODEL").unwrap_or_else(|_| DEFAULT_EMBEDDING_MODEL.to_string())
}

/// Resolve the OCR model: `HKASK_OCR_MODEL` → default.
pub fn ocr_model() -> String {
    std::env::var("HKASK_OCR_MODEL").unwrap_or_else(|_| DEFAULT_OCR_MODEL.to_string())
}
