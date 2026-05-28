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
//!
//! **Curator Pipeline:**
//! - Evaluates template outputs (Merge, Discard, Revise, Defer)
//! - OCAP boundary enforcement
//! - Variety counter tracking with algedonic alerts

pub mod adapters;
pub mod audit;
pub mod capability_validator;
pub mod cascade;
pub mod config;
pub mod context_assembly;
pub mod contract_validator;
pub mod contracts;
pub mod csp;
pub mod curator_pipeline;
pub mod dependency;
pub mod engine;
pub mod error;
pub mod inference_port;
pub mod manifest;
pub mod multi_okapi;
pub mod okapi_config;
pub mod ports;
pub mod prompt_cache;
pub mod provenance;
pub mod registry;
pub mod registry_sqlite;
pub mod renderer;
pub mod resilience;
pub mod skill_translation;

pub use audit::{AuditStats, AuditTrail, ExecutionAudit};
pub use capability_validator::CapabilityAwareValidator;
pub use context_assembly::{
    AddResult, AssemblyStats, ContextAssembler, ContextFragment, FragmentSource,
};
pub use contract_validator::{OkapiRequirements, RegistrationFrontmatter};
pub use contracts::{
    InferenceConfig as InferenceConfigParsed, ParsedContract, ParsedInference, TemplateFrontmatter,
    parse_frontmatter, validate_lexicon_terms,
};
pub use curator_pipeline::{CuratorPipeline, EvaluationResult, merge_outputs};
pub use dependency::{DependencyGraph, parse_dependencies};
pub use engine::{TemplateEngine, TemplateRegistry};
pub use hkask_types::cns::RetryConfig;
pub use inference_port::{
    InferenceError, InferencePort, InferenceResult, OkapiInference, Usage, create_shared_client,
    invoke_template_with_okapi_generic as invoke_template_with_okapi,
    invoke_template_with_selection_generic as invoke_template_with_selection,
};
pub use manifest::{
    CspEnforcer, EnergyAccount, ManifestExecutorImpl, ModelRequirements, NoopCsp, SelectorConfig,
    SimpleExecutor,
};
pub use okapi_config::OkapiConfig;
pub use ports::{
    Action, CnsPort, CompositionTemplate, DEFAULT_MATROSHKA_LIMIT, FAST_LOCAL_MODEL,
    InferenceConfig, ManifestExecutor, ManifestStep, McpPort, MemoryFragment, MemoryPort,
    ProcessManifest, RegistryEntry, RegistryIndex, Result, SyncInferencePort, TemplateContract,
    TemplateError, TemplateRenderer,
};
pub use provenance::{ProvenanceManager, TemplateProvenance};
pub use registry::{Registry, TemplateEntry};
pub use registry_sqlite::SqliteRegistry;
pub use resilience::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
