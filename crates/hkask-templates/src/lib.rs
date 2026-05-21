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
//! - `GitRegistry` — Git CAS-backed registry (production)
//! - `SqliteRegistry` — SQLite-backed registry (production with search)

pub mod audit;
pub mod capability_validator;
pub mod cascade;
pub mod contract_validator;
pub mod contracts;
pub mod dependency;
pub mod manifest;
pub mod ports;
pub mod provenance;
pub mod registry;
pub mod registry_git;
pub mod registry_sqlite;
pub mod renderer;
pub mod russell_mapper;

pub use audit::{AuditStats, AuditTrail, ExecutionAudit};
pub use capability_validator::CapabilityAwareValidator;
pub use contract_validator::{OkapiRequirements, RegistrationFrontmatter};
pub use contracts::{
    parse_frontmatter, validate_lexicon_terms, InferenceConfig as InferenceConfigParsed,
    ParsedContract, ParsedInference, TemplateFrontmatter,
};
pub use dependency::{parse_dependencies, DependencyGraph};
pub use manifest::{ManifestExecutorImpl, SelectorConfig, SimpleExecutor};
pub use ports::{
    Action, CnsPort, CompositionTemplate, InferenceConfig, InferencePort, ManifestExecutor,
    ManifestStep, McpPort, ProcessManifest, RegistryEntry, RegistryIndex, Result, TemplateContract,
    TemplateError, TemplateRenderer, DEFAULT_MATROSHKA_LIMIT, FAST_LOCAL_MODEL,
};
pub use provenance::{ProvenanceManager, TemplateProvenance};
pub use registry::{Registry, TemplateEntry};
pub use registry_git::GitRegistry;
pub use registry_sqlite::SqliteRegistry;
pub use russell_mapper::{
    MappedAsset, MappedAssetType, MigrationConfig, OutputFormat, RussellMapper,
    RussellSkillManifest,
};
