//! hKask Templates — Registry and template execution
//!
//! Unified registry with template_type discriminator per architecture v0.21.0.
//! Rust is the loom. YAML/Jinja2 is the thread.
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
pub mod contracts;
pub mod inference_port;
pub mod manifest;
pub mod okapi_config;
pub mod ports;
pub mod prompt_cache;
pub mod provenance;
pub mod registry;
pub mod registry_sqlite;
pub mod renderer;

pub use contracts::{
    ParsedContract, ParsedInference, TemplateFrontmatter, TemplateInferenceConfig,
    parse_frontmatter, validate_lexicon_terms,
};
pub use hkask_types::ports::{InferenceError, InferencePort, InferenceResult, InferenceUsage};
pub use inference_port::OkapiInference;
pub use manifest::ModelRequirements;
pub use okapi_config::OkapiConfig;
pub use okapi_config::{
    OkapiModelDetails, OkapiModelEntry, OkapiTagsResponse, list_okapi_models, search_okapi_models,
};
pub use ports::{
    Action, CompositionTemplate, DEFAULT_MATROSHKA_LIMIT, InferenceConfig, ManifestStep, McpPort,
    MemoryFragment, ProcessManifest, RegistryEntry, RegistryIndex, Result, TemplateContract,
    TemplateError, YamlManifestFile,
};
pub use provenance::{ProvenanceManager, TemplateProvenance};
pub use registry::{Registry, TemplateEntry};
pub use registry_sqlite::SqliteRegistry;
