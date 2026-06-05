//! hKask Types — Foundation types for the hKask agent platform
//!
//! This crate provides:
//! - ID types (WebID, TemplateID, BotID, GoalID, etc.)
//! - ν-event (cybernetic audit trail)
//! - hLexicon (canonical vocabulary)
//! - Visibility types (OCAP-enforced)
//! - Capability types (OCAP tokens)
//! - Template types (high-temperature templates, LLM parameters)
//! - Curation types (Curator, OCAP boundaries, curation decisions)
//! - CNS types (variety counters, algedonic alerts, kill zone detection)
//! - Sovereignty types (user sovereignty, acquisition resistance, kill-zone detection)
//! - Goal types (minimal coordination substrate for multi-agent collaboration)

pub mod agent_def;
pub mod audit;
pub mod bundle;
pub mod capability;
pub mod cns;
pub mod curation;
pub mod error;
pub mod event;
pub mod goal;
pub mod id;
pub mod identity;
pub mod lexicon;
pub mod loops;
pub mod ports;
pub mod r7;
pub mod secret;
pub mod soap_config;
pub mod sovereignty;
pub mod template;
pub mod text;
pub mod visibility;

pub use agent_def::{
    AgentDefinition, AgentKind, Charter, PersonaConstraints, RegisteredAgent, Responsibility, Right,
};
pub use audit::{AuditEntry, AuditLogPort, AuditOutcome};
pub use bundle::{
    BundleComplementarity, BundleConflict, BundleDependencyIndex, BundleManifest,
    BundleManifestStep, BundleSkill, BundleSkillChange, CascadePhase, ComplementarityType,
    CompositionError, ConflictResolution, ConflictType, GasConfig, SkillPolarity, ValidationResult,
    VersionBump,
};
pub use capability::tokens::{ConsolidationToken, IssuerVerification};
pub use capability::{
    AgentDelegation, CapabilityAction, CapabilityChecker, CapabilityParseError, CapabilityResource,
    CapabilitySpec, CapabilityToken, DelegationAction, DelegationResource, DelegationToken,
    DelegationTokenBuilder, SYSTEM_MAX_ATTENUATION, SYSTEM_MAX_RECURSION,
};
pub use cns::{CircuitState, CnsHealth, RetryConfig};
pub use curation::{CurationDecision, CurationThresholdConfig, OCAPBoundary};
pub use error::{GitError, HkaskError, InfrastructureError, McpErrorKind};
pub use event::{NuEvent, NuEventSink, Phase, Span, SpanNamespace};
pub use goal::*;
pub use id::*;
pub use identity::*;
pub use lexicon::{HLexicon, LexiconTerm, TemplateType};
pub use loops::{
    ActionType, CuratorDirective, CuratorHandle, CyberneticsHandle, Deviation, DeviationDirection,
    DispatchTarget, EpisodicReadHandle, EpisodicWriteHandle, ExperienceClassification, HkaskLoop,
    LoopAction, LoopId, LoopMessage, LoopPayload, MessagePriority, Signal, WorkerKind,
};
pub use ports::{
    BackpressureSignal, BundleRegistryIndex, CircuitBreakerPort, CnsObserver, CnsPort,
    ConsolidationOutcome, ConsolidationPort, DepletionSignal, EmbeddingError,
    EmbeddingGenerationError, EmbeddingGenerationPort, EmbeddingPort, GitCASPort, InferenceError,
    InferencePort, InferenceResult, InferenceUsage, MessageRecord, RegistryEntry, RegistryError,
    RegistryIndex, SessionRecord, SessionStoreError, SimilarityResult, Skill, SkillRegistryIndex,
    StandingSessionPort, StoredEmbedding, StructuredToolCall, TokenProb, TokenProbability,
    ToolInfo, ToolPort, ToolPortError,
};
pub use r7::{R7BotIdentity, default_r7_bots};
pub use secret::{SecretRef, derivation_contexts};
pub use soap_config::InferenceConfig;
pub use sovereignty::{
    DataCategory, KillZoneConfig, KillZoneState, KillZoneThresholds, SovereigntyCheckResult,
    SovereigntyOperation, SovereigntyPort, UserSovereigntyState,
};
pub use template::{
    LLMParameters, TemplateCrate, TemplateFile, TemplateId, TemplateInvocation, TemplateOutcome,
};
pub use text::blake3_hash;
pub use visibility::*;
