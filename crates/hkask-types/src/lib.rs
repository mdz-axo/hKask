//! hKask Types — Foundation types for the hKask agent platform
//!

pub mod agent;
pub mod audit;
pub mod bundle;
pub mod cns;
pub mod crypto;
pub mod curation;
pub mod error;
pub mod event;
pub mod goal;
pub mod id;
pub mod identity;
pub mod kanban;
pub mod ocr;

pub mod loops;
pub mod r7;
pub mod secret;
pub mod sovereignty;
pub mod template;
pub mod template_type;
pub mod text;
pub mod time;
pub mod token;
pub mod visibility;

pub mod transcript;
pub mod voice;
pub mod wallet;

#[cfg(feature = "sql")]
pub mod sql_impls;

// ── Essential re-exports (used by ≥3 downstream crates) ─────────────────

pub use agent::{
    AgentDefinition, AgentKind, Charter, Contact, PersonaConstraints, RegisteredAgent,
    Responsibility, Right, ScheduledTask, UserProfile,
};
pub use audit::{AuditEntry, AuditOutcome};
pub use bundle::BundleManifest;
pub use cns::CircuitState;
pub use crypto::Ed25519PublicKey;
pub use curation::CurationDecision;
pub use error::{InfrastructureError, McpErrorKind};
pub use event::{NuEvent, NuEventSink};
pub use goal::Goal;
pub use id::{
    ApiKeyId, BoardId, BotID, ColumnId, CommentId, EmbeddingID, EscalationID, EventID, GoalID, Id,
    IdKind, PhaseId, PodID, TaskId, TemplateID, TripleID, UserID, WalletId, WebID,
};
pub use kanban::{
    Board, CapabilityPackage, ColumnDef, Comment, ConditionResult, ConsentProof, ContractState,
    ContractVerification, Phase, Priority, SpawnSpec, Task, TaskContract, TaskFilter, TaskSpec,
    TaskStatus, Verification, VerificationCriterion,
};
pub use loops::{CurationInput, CuratorHandle, ExperienceClassification};
pub use transcript::{TimedWord, TranscriptBundle, TranscriptSegment};
pub use visibility::{Confidence, Visibility};
pub use voice::VoiceDesign;
pub use wallet::{
    ApiKeyCapability, ApiKeyMaterial, ChainId, DepositAddress, DepositReference, Encumbrance,
    EncumbranceStatus, PriceFeedConfig, PrivacyMode, RJoule, RateLimitConfig, TransactionType,
    WalletBalance, WalletConfig, WalletError, WalletTransaction,
};
