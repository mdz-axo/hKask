#![forbid(unsafe_code)]
//! hKask Ports — Hexagonal port traits for infrastructure abstractions
//!
//! Port traits that enable crates to depend on abstractions
//! rather than concrete implementations. Per the Authority DAG,
//! domain crates depend on these port traits (not on each other).

pub mod regulation;
pub mod consent_port;
pub mod embedding;
pub mod embedding_port;
pub mod escalation;
pub mod federation;
pub mod flowdef_validation;
pub mod pipeline_manifest;
pub mod pipeline_runner;
pub mod pipeline_state;
pub use federation::ReplicaId;
pub mod git_cas;
pub mod inference_port;
pub mod inference_types;
pub mod registry;
pub mod tool;
pub mod wallet_budget_port;

pub use cns::{
    BackpressureSignal, CircuitBreakerPort, LedgerObserver, LedgerStoragePort, ConsolidationOutcome,
    ConsolidationRequest, DecayConfig, DepletionSignal, WeightedEvent,
};
pub use embedding::EmbeddingGenerationError;
pub use flowdef_validation::{
    FlowDefValidationFinding, FlowDefValidationReport, validate_convergence_field,
    validate_step_input_mapping,
};
pub use inference_port::{InferencePort, InferenceStreamChunk};
pub use inference_types::{
    ChatToolDefinition, ChatToolFunction, InferenceError, InferenceResult, InferenceUsage,
    StructuredToolCall, TokenProb, TokenProbability, compute_confidence,
};
pub use registry::{
    RegistryEntry, RegistryError, RegistryIndex, Skill, SkillRegistryIndex, SkillZone,
};
pub use tool::{ToolFuture, ToolInfo, ToolPort, ToolPortError};
pub use wallet_budget_port::{WalletBudgetError, WalletBudgetPort};
