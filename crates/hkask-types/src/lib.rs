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
pub mod capability;
pub mod cns;
pub mod curation;
pub mod error;
pub mod event;
pub mod goal;
pub mod goal_capability;
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
pub use audit::{AuditContext, AuditEntry, AuditLogPort, AuditOutcome};
pub use capability::*;
pub use cns::{CircuitState, CnsHealth, RetryConfig};
pub use curation::{CurationDecision, OCAPBoundary};
pub use error::{GitError, HkaskError, InfrastructureError, McpErrorKind};
pub use event::{NuEvent, NuEventSink, Phase, Span, SpanNamespace};
pub use goal::*;
pub use goal_capability::*;
pub use id::*;
pub use identity::*;
pub use lexicon::{Domain, HLexicon, LexiconTerm, TemplateType};
pub use loops::{
    AUTHORITY_EDGES, ActionType, CuratorDirective, CuratorHandle, Deviation, DeviationDirection,
    EpisodicReadHandle, EpisodicWriteHandle, ExperienceClassification, HkaskLoop, LoopAction,
    LoopId, LoopMessage, LoopPayload, MessagePriority, Signal,
};
pub use ports::{
    CircuitBreakerPort, CnsPort, ConsolidationOutcome, ConsolidationPort, GitCASPort,
    InferenceError, InferencePort, InferenceResult, InferenceUsage, MessageRecord, RegistryEntry,
    RegistryError, RegistryIndex, SessionRecord, SessionStoreError, StandingSessionPort, TokenProb,
    TokenProbability,
};
pub use r7::{R7BotIdentity, default_r7_bots};
pub use secret::{SecretRef, derivation_contexts};
pub use soap_config::InferenceConfig;
pub use sovereignty::{
    DataCategory, KillZoneConfig, KillZoneState, KillZoneThresholds, SovereigntyCheckResult,
    SovereigntyId, SovereigntyOperation, SovereigntyPort, UserSovereigntyState,
};
pub use template::{
    LLMParameters, TemplateCrate, TemplateFile, TemplateId, TemplateInvocation, TemplateOutcome,
};
pub use text::blake3_hash;
pub use visibility::*;
