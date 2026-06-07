//! hKask Templates — registry and template execution
//
//! Unified registry with template_type discriminator per architecture v0.22.0.
//! Rust is the loom. YAML/Jinja2 is the thread.
//
//! Straddles Inference (L1) and Curation (L5). Split deferred until second consumer.
//! Template types: Prompt (WordAct), Process (FlowDef), Cognition (KnowAct).
//! Registry adapters: `Registry` (in-memory), `SqliteRegistry` (SQLite).

pub mod embedding_port;
pub mod executor;
pub mod inference_port;
pub mod lexicon;
pub mod manifest_loader;
pub mod okapi_config;
pub mod ports;
pub mod prompt_cache;
pub mod prompt_strategy;

pub mod registry;
pub mod registry_sqlite;
pub mod skill_loader;

pub use embedding_port::OkapiEmbedding;
pub use executor::ManifestExecutor;
pub use hkask_types::ports::BundleRegistryIndex;
pub use hkask_types::ports::InferencePort;
pub use hkask_types::ports::Skill;
pub use hkask_types::ports::SkillZone;
pub use hkask_types::{BundleManifest, SkillPolarity};
pub use inference_port::OkapiInference;

pub use manifest_loader::resolve_manifest;
pub use okapi_config::OkapiConfig;

pub use okapi_config::OkapiModelEntry;
pub use okapi_config::{list_okapi_models, search_okapi_models};
pub use ports::{McpPort, RegistryEntry, RegistryIndex, Result, TemplateError};
// ManifestLoadError kept pub(crate) — only used within crate for YAML loading errors
pub use prompt_strategy::PromptStrategy;

pub use registry::Registry;
pub use registry_sqlite::SqliteRegistry;
pub use skill_loader::{SkillFrontMatter, SkillLoadResult, SkillLoader};
