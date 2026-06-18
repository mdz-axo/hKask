//! Hexagonal port traits — Infrastructure abstractions
//
//! Port traits that enable crates to depend on abstractions
//! rather than concrete implementations. Per the Authority DAG,
//! domain crates depend on these port traits (not on each other).

pub mod git_cas;

pub mod cns;
pub mod embedding;
pub mod inference_port;
pub mod inference_types;
pub mod registry;
pub mod tool;

// --- CNS boundary ports ---
pub use cns::{
    BackpressureSignal, CircuitBreakerPort, CnsObserver, ConsolidationOutcome,
    ConsolidationRequest, DepletionSignal,
};

// --- Embedding ---
pub use embedding::EmbeddingGenerationError;

// --- Inference data types ---
pub use inference_types::{
    InferenceError, InferenceResult, InferenceUsage, StructuredToolCall, TokenProb,
    TokenProbability, compute_confidence,
};

// --- Inference port trait + stream chunk ---
pub use inference_port::{InferencePort, InferenceStreamChunk};

// --- Registry domain ---
pub use registry::{
    BundleRegistryIndex, RegistryEntry, RegistryError, RegistryIndex, Skill, SkillRegistryIndex,
    SkillZone,
};

// --- Tool governance ---
pub use tool::{ToolInfo, ToolPort, ToolPortError};
