//! Model name resolution — env-configurable with compile-time defaults.
//!
//! Every model used in the system has a corresponding env var for override.
//! The constants here are DEFAULT values; env vars take precedence.
//! This eliminates the need to recompile when models are superseded.
//!
//! Naming convention:
//! - `HKASK_CLASSIFIER_MODEL` — primary classifier model
//! - `HKASK_EMBEDDING_MODEL` — default embedding model
//! - `HKASK_OCR_MODEL` — OCR model for scanned PDF fallback
//! - `HKASK_MODEL_DEFAULT` — fallback when provider-specific not set

/// Approved classifier model for corpus pipeline classification.
/// Qwen3-235B-A22B: 235B total, 22B active MoE. Hosted on KiloCode (China)
/// to satisfy the cross-jurisdiction requirement — the default judge/panel
/// peers default to US-hosted providers (e.g., DeepInfra), so the classifier
/// operates from a different jurisdiction. Fusion orchestration (algo or
/// LLM judge) merges panel outputs; see `fusion_orchestrator`.
pub const DEFAULT_CLASSIFIER_MODEL: &str = "KC/qwen/qwen3-235b-a22b-2507";

/// Default embedding model.
pub const DEFAULT_EMBEDDING_MODEL: &str = "DI/Qwen/Qwen3-Embedding-0.6B";

/// Default OCR model for scanned PDF fallback.
/// Uses kask-ocr on RunPod (OLMOCR-2).
pub const DEFAULT_OCR_MODEL: &str = "RP/kask-ocr";

/// Fallback model when no other model is configured.
/// Prefixed with `KC/` so it routes to KiloCode (which hosts this exact id);
/// an unprefixed value would fall through to the default provider, where the
/// id differs (e.g. DeepInfra exposes it as `deepseek-ai/DeepSeek-V4-Pro`).
pub const DEFAULT_FALLBACK_MODEL: &str = "KC/deepseek-v4-pro";

// ── Test fixtures (arbitrary identifiers, no network calls) ──────────────

pub const TEST_MODEL_SMALL: &str = "DI/google/gemma-4-9b-it";
pub const TEST_MODEL_MEDIUM: &str = "DI/meta-llama/Llama-4-Scout-17B-16E-Instruct";

// ── Resolved model accessors (env var → default) ──────────────────────────

/// Resolve the primary classifier: `HKASK_CLASSIFIER_MODEL` → default.
pub fn classifier_model() -> String {
    std::env::var("HKASK_CLASSIFIER_MODEL").unwrap_or_else(|_| DEFAULT_CLASSIFIER_MODEL.to_string())
}

/// Resolve the embedding model: `HKASK_EMBEDDING_MODEL` → default.
pub fn embedding_model() -> String {
    std::env::var("HKASK_EMBEDDING_MODEL").unwrap_or_else(|_| DEFAULT_EMBEDDING_MODEL.to_string())
}

/// Resolve the OCR model: `HKASK_OCR_MODEL` → default.
pub fn ocr_model() -> String {
    std::env::var("HKASK_OCR_MODEL").unwrap_or_else(|_| DEFAULT_OCR_MODEL.to_string())
}
