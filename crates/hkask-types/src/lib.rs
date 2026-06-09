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
//! - CNS types (variety counters, algedonic alerts)
//! - Sovereignty types (user sovereignty, affirmative consent)
//! - Goal types (minimal coordination substrate for multi-agent collaboration)

pub mod agent_def;
pub mod allosteric;
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
pub mod inference_config;
pub mod lexicon;
pub mod loops;
pub mod ports;
pub mod r7;
pub mod secret;
pub mod sovereignty;
pub mod template;
pub mod text;
pub mod time;
pub mod visibility;

#[cfg(feature = "sql")]
pub mod sql_impls;

pub use agent_def::{
    AgentDefinition, AgentKind, Charter, PersonaConstraints, RegisteredAgent, Responsibility, Right,
};
pub use allosteric::{AllostericError, AllostericGate, AllostericGateConfig, mwc_state_function};
pub use audit::{AuditEntry, AuditOutcome};
pub use bundle::{
    BundleComplementarity, BundleConflict, BundleManifest, BundleManifestStep, BundleSkill,
    CascadePhase, ComplementarityType, ConflictResolution, ConflictType, GasConfig, SkillPolarity,
    ValidationResult,
};
pub use capability::tokens::ConsolidationToken;
pub use capability::{
    AuthContext, CapabilityChecker, CapabilitySpec, DelegationAction, DelegationResource,
    DelegationToken, DelegationTokenBuilder, SYSTEM_MAX_ATTENUATION, SYSTEM_MAX_RECURSION,
    TOKEN_ERR_EXPIRED, TOKEN_ERR_INVALID_SIGNATURE, TOKEN_ERR_NO_CHECKER, VerificationOutcome,
    capabilities_match, capability_from_server_id, require_read_access, require_write_access,
    token_err_insufficient_access, token_err_tool_access_denied, verify_delegation_token,
    verify_delegation_token_now,
};
pub use cns::{CircuitState, CnsHealth, QueueDepth, RBarThreshold};
pub use curation::{CurationDecision, OCAPBoundary, OcapCapability, OcapTokenKind};
pub use error::{GitError, HkaskError, InfrastructureError, McpErrorKind};
pub use event::{NuEvent, NuEventSink};
pub use goal::{Goal, GoalArtifact, GoalCriterion, GoalState};
pub use id::{
    BotID, EmbeddingID, EventID, GoalID, Id, IdKind, PodID, TemplateID, TripleID, UserID, WebID,
};
pub use identity::{
    HumanUser, RegistrationError, RegistrationRequest, ReplicantIdentity, UserSession,
};
pub use inference_config::InferenceConfig;
pub use lexicon::{HLexicon, LexiconTerm, TemplateType};
pub use loops::{
    ActionType, CuratorDirective, CuratorHandle, Deviation, DeviationDirection, DispatchTarget,
    ExperienceClassification, HkaskLoop, LoopAction, LoopId, LoopMessage, LoopPayload,
    MessagePriority, Signal, SignalMetric, WorkerKind,
};
pub use ports::git_cas::{
    CommitHash, ContentHash, DiffKind, FileDiff, GitCASPort, GitCasError, MockGitCas, RepoId,
    RepoSnapshotPolicy, RetentionPolicy, RetentionTier, SnapshotMetadata, SnapshotTrigger,
    TreeEntry, TreeEntryKind, TripleEntry, VerificationReport,
};
pub use ports::{
    BackpressureSignal, BundleRegistryIndex, CircuitBreakerPort, CnsObserver, ConsolidationOutcome,
    ConsolidationRequest, DepletionSignal, EmbeddingGenerationError, InferenceError, InferencePort,
    InferenceResult, InferenceStreamChunk, InferenceUsage, MessageRecord, RegistryEntry,
    RegistryError, RegistryIndex, SessionRecord, SessionStoreError, Skill, SkillRegistryIndex,
    SkillZone, StructuredToolCall, TokenProb, TokenProbability, ToolInfo, ToolPort, ToolPortError,
};
pub use r7::{R7BotIdentity, default_r7_bots};
pub use secret::{SecretRef, derivation_contexts};
pub use sovereignty::{DataCategory, SovereigntyPort, UserSovereigntyState};
pub use template::{LLMParameters, TemplateCrate, TemplateFile};
pub use text::blake3_hash;
pub use time::now_rfc3339;
pub use visibility::{AccessControl, Confidence, TemporalBounds, Visibility};
