//! hKask Types — Foundation types for the hKask agent platform
//!
//! This crate provides:
//! - ID types (WebID, TemplateID, BotID, GoalID, SpecId, etc.)
//! - ν-event (cybernetic audit trail)
//! - hLexicon (canonical vocabulary)
//! - Visibility types (OCAP-enforced)
//! - Capability types (OCAP tokens)
//! - Template types (high-temperature templates, LLM parameters)
//! - Curation types (Curator, OCAP boundaries, curation decisions)
//! - CNS types (variety counters, algedonic alerts, kill zone detection)
//! - Sovereignty types (user sovereignty, acquisition resistance, kill-zone detection)
//! - Goal types (minimal coordination substrate for multi-agent collaboration)
//! - Spec types (DDMVSS domain types, completeness predicates, curation integration)

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
pub mod sovereignty;
pub mod spec;
pub mod template;
pub mod text;
pub mod visibility;

pub use agent_def::{
    AgentDefinition, AgentKind, AgentStandingSessionConfig, Charter, PersonaConstraints,
    ReadinessProbe, RegisteredAgent, ReportingConfig, Responsibility, Right,
};
pub use audit::{AuditContext, AuditEntry, AuditLogPort, AuditOutcome};
pub use capability::*;
pub use cns::*;
pub use curation::*;
pub use error::{
    ArchivalResult, AuthorizationError, GitArchivalError, GitError, HkaskError,
    InfrastructureError, McpErrorKind,
};
pub use event::{NuEvent, NuEventSink, NuEventSinkError, Phase, Span, SpanCategory};
pub use goal::*;
pub use goal_capability::*;
pub use id::*;
pub use identity::*;
pub use lexicon::{Domain, HLexicon, LexiconTerm, TemplateType};
pub use loops::{
    CnsAdminHandle, CnsGovernReadHandle, CnsGovernWriteHandle, CnsWriteHandle, ControlPrimitive,
    CuratorHandle, DataVisibilityTier, EnergyBudgetHandle, EpisodicBudgetExceeded,
    EpisodicReadHandle, EpisodicWriteHandle, ExperienceClassification, GovernanceHandle,
    InferenceHandle, LoopId, LoopMessage, LoopOrigin, LoopPayload, MessagePriority,
    RateLimiterHandle, SemanticReadHandle, SemanticWriteHandle, TraceId,
};
pub use ports::GitCASPort;
pub use r7::{R7BotIdentity, R7BotRegistry, default_r7_bots};
pub use secret::{SecretRef, derivation_contexts};
pub use sovereignty::{
    AcquisitionResistance, DataCategory, DataSovereigntyBoundary, KillZoneDetector,
    SovereigntyCheckResult, SovereigntyId, SovereigntyOperation, SovereigntyPort,
    UserSovereigntyState,
};
pub use spec::{
    Criterion, DomainAnchor, GoalSpec, Spec, SpecCategory, SpecCurationRecord, SpecCurator,
    SpecError, SpecId, SpecObserver, SpecStore,
};
pub use template::{
    HighTempTemplateType, LLMParameters, TemperatureRange, TemplateCrate, TemplateFile, TemplateId,
    TemplateInvocation, TemplateOutcome,
};
pub use text::{blake3_hash, estimate_tokens};
pub use visibility::*;
