//! hKask Templates — Registry and template execution
//!
//! Unified registry with template_type discriminator per architecture v0.21.0.
//! Rust is the loom. YAML/Jinja2 is the thread.
//!
//! **Intended Architecture (deferred until second consumer):**
//!
//! This crate straddles two loops:
//! - Inference (L1): rendering, registry for discovery, prompt assembly, OkapiInference
//! - Curation (L5): lexicon validation, provenance
//!
//! When a second consumer requires it, split into:
//! - `hkask-templates-inference` (depends on `hkask-cns` for GovernedTool)
//! - `hkask-templates-curation` (depends on `hkask-agents` for CurationLoop)
//!
//! Until then, this crate serves both concerns. `hkask-ensemble` intentionally
//! avoids depending on `hkask-templates` to prevent pulling in both inference
//! and curation dependencies transitively.
//!
//! **Template Types:**
//! - Prompt (WordAct) — What to say
//! - Process (FlowDef) — What to do
//! - Cognition (KnowAct) — How to think
//!
//! **Registry Adapters:**
//! - `Registry` — In-memory filesystem-based registry (MVP default)
//! - `SqliteRegistry` — SQLite-backed registry (production with search)
//!
//! **High-Temperature Templates:**
//! - Anti-normative generation via temperature-controlled LLM parameters
//! - Prevents convergence to homogeneous, predictable behavior

pub mod adapters;

pub mod embedding_port;
pub mod inference_port;
pub mod lexicon;
pub mod okapi_config;
pub mod ports;
pub mod prompt_cache;
pub mod prompt_strategy;
pub mod provenance;
pub mod registry;
pub mod registry_sqlite;

pub use embedding_port::OkapiEmbedding;
pub use hkask_types::ports::BundleRegistryIndex;
pub use hkask_types::ports::EmbeddingGenerationPort;
pub use hkask_types::ports::InferencePort;
pub use hkask_types::ports::Skill;
pub use hkask_types::{
    BundleDependencyIndex, BundleManifest, BundleSkillChange, SkillPolarity, VersionBump,
};
pub use inference_port::OkapiInference;
pub use lexicon::{load_hlexicon_default, load_hlexicon_from_file, load_hlexicon_from_yaml};
pub use okapi_config::OkapiConfig;
pub use okapi_config::OkapiModelDetails;
pub use okapi_config::OkapiModelEntry;
pub use okapi_config::{list_okapi_models, search_okapi_models};
pub use ports::{McpPort, RegistryEntry, RegistryError, RegistryIndex, Result, TemplateError};
pub use prompt_strategy::PromptStrategy;
pub use provenance::TemplateProvenance;
pub use registry::Registry;
pub use registry_sqlite::SqliteRegistry;
