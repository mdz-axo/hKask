//! hKask Types — Foundation types for the hKask agent platform
//!

pub mod agent;
pub mod agent_paths;
pub mod agent_registry;
pub mod cns;
pub mod corpus;
pub mod crypto;
pub mod curation;
pub mod curator;
pub mod error;
pub mod event;
pub mod fusion;
pub mod goal;
pub mod id;
pub mod identity;
pub mod keychain_keys;
pub mod loops;
pub mod macros;
pub mod observable_span;
pub mod secret;
pub mod server_config;
pub mod skill;
pub mod template;
pub mod template_type;

pub mod time;
pub mod transcript;
pub mod visibility;
// NOTE: Wallet types (RJoule, WalletConfig, ChainId, etc.) moved to hkask-wallet-types.

#[cfg(feature = "sql")]
pub mod sql_impls;

// ── Essential re-exports (used by ≥3 downstream crates) ─────────────────

pub use agent::AgentKind;
pub use agent::PersonaConstraints;
pub use agent_registry::{
    AgentDefinition, Charter, Contact, RegisteredAgent, Responsibility, Right, ScheduledTask,
    UserProfile,
};
pub use cns::CircuitState;
pub use crypto::Ed25519PublicKey;
pub use curation::{
    BoundaryClassification, DataCategory, DataSovereigntyBoundary, UserSovereigntyState,
};
pub use curator::{CurationThresholdConfig, CuratorDirective, CuratorHandle, EscalationSeverity};
pub use error::{CapabilityDenied, InfrastructureError, McpErrorKind, NotFound};
pub use event::{NuEvent, NuEventSink};
pub use goal::GoalState;
pub use id::{
    ApiKeyId, BoardId, BotID, ColumnId, CommentId, EmbeddingID, EscalationID, EventID, GoalID,
    HMemId, Id, IdKind, PhaseId, PodID, TaskId, TemplateID, UserID, WalletId, WebID,
};

pub use loops::{
    ActionDecision, ActionType, BudgetOption, Deviation, DeviationDirection,
    ExperienceClassification, ImpactReport, LoopAction, LoopActionParams, LoopId, LoopQuality,
    RegulationData, Signal, SignalMetric, TriggerOrigin,
};
pub use observable_span::ObservableSpan;
pub use skill::SkillPolarity;
pub use template::LLMParameters;
pub use template_type::TemplateType;
pub use transcript::{TimedWord, TranscriptBundle, TranscriptSegment};
pub use visibility::{Confidence, Dimension, Visibility};
