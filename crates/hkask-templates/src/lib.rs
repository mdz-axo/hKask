//! hKask Templates — Registry and template execution
//!
//! Unified registry with template_type discriminator per architecture v0.21.0.
//! Rust is the loom. YAML/Jinja2 is the thread.
//!
//! **Template Types:**
//! - Prompt (WordAct) — What to say
//! - Process (FlowDef) — What to do
//! - Cognition (KnowAct) — How to think

pub mod cascade;
pub mod manifest;
pub mod ports;
pub mod registry;
pub mod renderer;

pub use ports::{
    Action, CnsPort, CompositionTemplate, DEFAULT_MATROSHKA_LIMIT, FAST_LOCAL_MODEL, InferencePort,
    ManifestExecutor, ManifestStep, McpPort, ProcessManifest, RegistryEntry, RegistryIndex, Result,
    TemplateContract, TemplateError, TemplateRenderer,
};
pub use registry::{Registry, TemplateEntry};
