//! hKask Templates — Registry and template execution
//!
//! Unified registry with template_type discriminator per architecture v0.21.0.
//! Rust is the loom. YAML/Jinja2 is the thread.
//!
//! **Template Types:**
//! - Prompt (WordAct) — What to say
//! - Process (FlowDef) — What to do
//! - Cognition (KnowAct) — How to think

pub mod audit;
pub mod cascade;
pub mod contracts;
pub mod dependency;
pub mod manifest;
pub mod ports;
pub mod provenance;
pub mod registry;
pub mod renderer;

pub use audit::{AuditStats, AuditTrail, ExecutionAudit};
pub use contracts::{parse_frontmatter, validate_lexicon_terms, InferenceConfig as FrontmatterInferenceConfig, ParsedContract, ParsedInference, TemplateFrontmatter};
pub use dependency::{parse_dependencies, DependencyGraph};
pub use manifest::SelectorConfig;
pub use ports::{
    Action, CnsPort, CompositionTemplate, DEFAULT_MATROSHKA_LIMIT, FAST_LOCAL_MODEL, InferenceConfig,
    InferencePort, ManifestExecutor, ManifestStep, McpPort, ProcessManifest, RegistryEntry,
    RegistryIndex, Result, TemplateContract, TemplateError, TemplateRenderer,
};
pub use provenance::{ProvenanceManager, TemplateProvenance};
pub use registry::{Registry, TemplateEntry};
