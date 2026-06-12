//! hKask Templates — registry and template execution
//!
//! Unified registry with template_type discriminator per architecture v0.22.0.
//! Rust is the loom. YAML/Jinja2 is the thread.
//!
//! Inference (L1) has been extracted to `hkask-inference`. Deprecation
//! re-exports below provide backward compatibility for one release cycle.
//! Template types: Prompt (WordAct), Process (FlowDef), Cognition (KnowAct).
//! Registry adapters: `Registry` (in-memory), `SqliteRegistry` (SQLite).

pub mod contract_validator;
pub mod executor;
pub mod lexicon;
pub mod manifest_loader;
pub mod ports;
pub mod prompt_strategy;
pub mod registry;
pub mod registry_sqlite;
pub mod skill_loader;

// ── Deprecation re-exports (inference moved to hkask-inference) ──────────────

pub mod embedding_port;
pub mod inference_config;
pub mod inference_port;
pub mod okapi_config;

pub use executor::ManifestExecutor;
pub use hkask_types::ports::BundleRegistryIndex;
pub use hkask_types::ports::InferencePort;
pub use hkask_types::ports::Skill;
pub use hkask_types::ports::SkillZone;
pub use hkask_types::{BundleManifest, SkillPolarity};

pub use manifest_loader::resolve_manifest;
pub use ports::{McpPort, RegistryEntry, RegistryIndex, Result, TemplateError};
pub use prompt_strategy::PromptStrategy;

pub use registry::Registry;
pub use registry_sqlite::SqliteRegistry;
pub use skill_loader::{SkillFrontMatter, SkillLoadResult, SkillLoader};

// ── Inference re-exports (from hkask-inference) ─────────────────────────────

pub use hkask_inference::EmbeddingRouter;
pub use hkask_inference::InferenceConfig;
pub use hkask_inference::InferenceRouter;
pub use hkask_inference::ProviderId;
pub use hkask_inference::RouterModelEntry;

// ── Deprecated aliases (migrate to hkask-inference types) ────────────────────

#[deprecated(
    since = "0.28.0",
    note = "Use hkask_inference::InferenceRouter instead"
)]
pub use inference_port::OkapiInference;

#[deprecated(
    since = "0.28.0",
    note = "Use hkask_inference::InferenceConfig instead"
)]
pub use okapi_config::OkapiConfig;

#[deprecated(
    since = "0.28.0",
    note = "Use hkask_inference::EmbeddingRouter instead"
)]
pub use embedding_port::OkapiEmbedding;

#[deprecated(
    since = "0.28.0",
    note = "Use hkask_inference::RouterModelEntry or provider-specific types"
)]
pub use okapi_config::OkapiModelEntry;

#[deprecated(
    since = "0.28.0",
    note = "Use hkask_inference::InferenceRouter::list_models()"
)]
pub use okapi_config::list_okapi_models;

#[deprecated(
    since = "0.28.0",
    note = "Use hkask_inference::InferenceRouter::search_models()"
)]
pub use okapi_config::search_okapi_models;

#[deprecated(
    since = "0.28.0",
    note = "Model show not yet ported to hkask-inference"
)]
pub use okapi_config::{OkapiModelShow, fetch_model_show};
