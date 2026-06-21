//! hKask Ports — Hexagonal port traits for infrastructure abstractions
//!
//! Port traits that enable crates to depend on abstractions
//! rather than concrete implementations. Per the Authority DAG,
//! domain crates depend on these port traits (not on each other).

pub mod cns;
pub mod embedding;
pub mod git_cas;
pub mod inference_port;
pub mod inference_types;
pub mod registry;
pub mod tool;

pub use cns::{
    BackpressureSignal, CircuitBreakerPort, CnsObserver, ConsolidationOutcome,
    ConsolidationRequest, DepletionSignal,
};
pub use embedding::EmbeddingGenerationError;
pub use inference_port::{InferencePort, InferenceStreamChunk};
pub use inference_types::{
    InferenceError, InferenceResult, InferenceUsage, StructuredToolCall, TokenProb,
    TokenProbability, compute_confidence,
};
pub use registry::{
    RegistryEntry, RegistryError, RegistryIndex, Skill, SkillRegistryIndex, SkillZone,
};
pub use tool::{ToolInfo, ToolPort, ToolPortError};
