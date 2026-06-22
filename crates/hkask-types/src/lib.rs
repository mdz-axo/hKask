//! hKask Types — Foundation types for the hKask agent platform
//!

pub mod agent;

pub mod agent_paths;
pub mod cns;
pub mod crypto;
pub mod curation;
pub mod error;
pub mod event;
pub mod goal;
pub mod id;
pub mod identity;
pub mod keychain_keys;
pub mod loops;
pub mod secret;
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
pub use cns::CircuitState;
pub use crypto::Ed25519PublicKey;
pub use curation::{
    BoundaryClassification, DataCategory, DataSovereigntyBoundary, UserSovereigntyState,
};
pub use error::{InfrastructureError, McpErrorKind};
pub use event::{NuEvent, NuEventSink};
pub use goal::GoalState;
pub use id::{
    ApiKeyId, BoardId, BotID, ColumnId, CommentId, EmbeddingID, EscalationID, EventID, GoalID, Id,
    IdKind, PhaseId, PodID, SovereigntyId, TaskId, TemplateID, TripleID, UserID, WalletId, WebID,
};

pub use loops::LoopId;
pub use skill::SkillPolarity;
pub use template::LLMParameters;
pub use template_type::TemplateType;
pub use transcript::{TimedWord, TranscriptBundle, TranscriptSegment};
pub use visibility::{Confidence, Visibility};
