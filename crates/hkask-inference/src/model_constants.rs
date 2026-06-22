//! Canonical model name constants.
//!
//! Every model name used in the codebase MUST be defined here, never inlined.
//! This ensures a single point of update when models are superseded.
//!
//! Naming convention:
//! - `CLASSIFIER_*` — fast, cheap, non-thinking models for classification/summarization
//! - `EMBEDDING_*` — embedding models
//! - `DEFAULT_*` — fallback defaults when env vars are unset
//! - `TEST_*` — arbitrary strings for serialization/fixture tests

/// Fast classifier model for summarization, classification, and non-thinking tasks.
/// Uses the `DI/` (DeepInfra) prefix for cloud-first routing.
/// Current: Google Gemma 4 9B — fast, cheap, reliable for structured output.
pub const CLASSIFIER_MODEL: &str = "DI/google/gemma-4-9b-it";

/// Default embedding model.
/// Current: Qwen3 Embedding 0.6B — compact, high-quality embeddings.
pub const EMBEDDING_MODEL: &str = "DI/Qwen/Qwen3-Embedding-0.6B";

/// Fallback model when `HKASK_MODEL` env var is unset (ACP agent).
/// Mirrors `InferenceConfig::default_model` to keep the codebase consistent.
pub const DEFAULT_FALLBACK_MODEL: &str = "deepseek-v4-pro";

// ── Test fixtures (arbitrary identifiers, no network calls) ──────────────

/// Arbitrary model identifier for serialization tests.
/// Any valid model string works; choose a stable name to avoid churn.
pub const TEST_MODEL_SMALL: &str = "DI/google/gemma-4-9b-it";

/// Arbitrary model identifier for serialization tests (variant).
pub const TEST_MODEL_MEDIUM: &str = "DI/meta-llama/Llama-4-Scout-17B-16E-Instruct";
