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
/// Model selection, Few-Shot strategy, and rationale are documented in
/// `registry/classify/triple-extractor.yaml`.
pub const CLASSIFIER_MODEL: &str = "KC/qwen/qwen3-235b-a22b-2507";

/// Default embedding model.
/// Current: Qwen3 Embedding 0.6B — compact, high-quality embeddings.
pub const EMBEDDING_MODEL: &str = "DI/Qwen/Qwen3-Embedding-0.6B";

/// Default OCR model for scanned PDF fallback.
/// Current: LightOnOCR-2 1B — fast, specialized document OCR model.
pub const OCR_MODEL: &str = "maternion/LightOnOCR-2:1b";

/// Fallback model when `HKASK_MODEL` env var is unset (ACP agent).
/// Mirrors `InferenceConfig::default_model` to keep the codebase consistent.
/// Updated June 2026: DeepSeek V4 Pro remains the strongest all-round open-weight model.
pub const DEFAULT_FALLBACK_MODEL: &str = "deepseek-v4-pro";

// ── Test fixtures (arbitrary identifiers, no network calls) ──────────────

/// Arbitrary model identifier for serialization tests.
/// Any valid model string works; choose a stable name to avoid churn.
pub const TEST_MODEL_SMALL: &str = "DI/google/gemma-4-9b-it";

/// Arbitrary model identifier for serialization tests (variant).
pub const TEST_MODEL_MEDIUM: &str = "DI/meta-llama/Llama-4-Scout-17B-16E-Instruct";
