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

pub mod adapters;
pub mod audit;
pub mod cascade;
pub mod contracts;
pub mod contract_validator;
pub mod csp;
pub mod dependency;
pub mod error;
pub mod manifest;
pub mod ports;
pub mod provenance;
pub mod registry;
pub mod registry_git;
pub mod registry_sqlite;
pub mod renderer;
pub mod security;
pub mod skill_translation;

pub use adapters::{MockRegistryAdapter, RegistryAdapter, RegistryResult, SkillRegistryPort};
pub use audit::{AuditStats, AuditTrail, ExecutionAudit};
pub use cascade::{Cascade, CascadeBuilder, CascadeContext, CascadeExecutor, MAX_CASCADE_DEPTH};
pub use contracts::{
    InferenceConfig as InferenceConfigParsed, ParsedContract, ParsedInference, TemplateFrontmatter,
    parse_frontmatter, validate_lexicon_terms,
};
pub use contract_validator::{
    ContractValidator, OkapiCapabilities, OkapiRequirements, RegistrationFrontmatter, ValidationError, ValidatorError,
    fetch_okapi_capabilities,
};
pub use csp::{
    CspPipelineExecutor, CspStageConfig, IsolatedStageRunner, StageExecutor, StageMessage,
    StageResult,
};
pub use dependency::{parse_dependencies, DependencyGraph};
pub use error::{CompositionError, RetryConfig};
pub use manifest::SelectorConfig;
pub use ports::{
    Action, CnsPort, CompositionTemplate, DEFAULT_MATROSHKA_LIMIT, FAST_LOCAL_MODEL,
    DependencyProvider, InMemoryDependencyProvider, InferenceConfig, InferencePort, ManifestExecutor,
    ManifestStep, McpPort, ProcessManifest, RegistryEntry, RegistryIndex, Result, TemplateContract,
    TemplateError, TemplateRenderer,
};
pub use provenance::{ProvenanceManager, TemplateProvenance};
pub use registry::{Registry, TemplateEntry};
pub use registry_git::GitRegistry;
pub use registry_sqlite::SqliteRegistry;
pub use security::SecurityAdapter;
pub use skill_translation::{
    GeneratedManifest, GeneratedTemplate, ManifestStep as SkillManifestStep, ParsedPrompt,
    ParsedSkill, PipelineStage, RdfTriple, RegisteredArtifact, SkillFormat,
    SkillTranslationPipeline, StageOutput, TemplateContract as SkillContract, ValidatedArtifact,
};
