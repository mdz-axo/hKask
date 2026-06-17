//! hKask Types — Foundation types for the hKask agent platform
//!
//! G2 Justification: Re-export facade for the most-used types across the workspace.
//! Each re-export maps to a type used by ≥3 downstream crates. Less-commonly-used
//! types are accessed via their submodule paths (e.g., `hkask_types::sovereignty::DataCategory`).

pub mod agent;
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
pub mod kanban;
pub mod ocr;

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
pub use capability::{
    AuthContext, CapabilityChecker, CapabilitySpec, DelegationAction, DelegationResource,
    DelegationToken, DelegationTokenBuilder, SYSTEM_MAX_ATTENUATION, SYSTEM_MAX_RECURSION,
    TOKEN_ERR_EXPIRED, TOKEN_ERR_INVALID_SIGNATURE, TOKEN_ERR_NO_CHECKER, VerificationOutcome,
    capabilities_match, capability_from_server_id, require_read_access, require_write_access,
    token_err_insufficient_access, token_err_tool_access_denied, verify_delegation_token,
    verify_delegation_token_now,
};
pub use cns::CircuitState;
pub use curation::CurationDecision;
pub use error::{InfrastructureError, McpErrorKind};
pub use event::{NuEvent, NuEventSink};
pub use goal::Goal;
pub use id::{
    ApiKeyId, BoardId, BotID, ColumnId, CommentId, EmbeddingID, EscalationID, EventID, GoalID, Id, IdKind,
    PodID, TaskId, TemplateID, PhaseId, TripleID, UserID, WalletId, WebID,
};
pub use kanban::{
    Board, ColumnDef, ConsentProof, Task, TaskFilter, TaskSpec, TaskStatus, Verification,
    VerificationCriterion, Comment, Phase, Priority, CapabilityPackage, TaskContract, SpawnSpec,
};
pub use loops::{CurationInput, CuratorHandle, ExperienceClassification};
pub use ports::{
    BundleRegistryIndex, CircuitBreakerPort, EmbeddingGenerationError, InferenceError,
    InferencePort, InferenceResult, InferenceStreamChunk, InferenceUsage, RegistryEntry,
    RegistryError, RegistryIndex, Skill, SkillRegistryIndex, SkillZone, StructuredToolCall,
    ToolInfo, ToolPort, ToolPortError,
};
pub use transcript::{TimedWord, TranscriptBundle, TranscriptSegment};
pub use visibility::{Confidence, Visibility};
pub use voice::VoiceDesign;
pub use wallet::{
    ApiKeyCapability, ApiKeyMaterial, ChainId, DepositAddress, DepositReference, Ed25519PublicKey,
    Encumbrance, EncumbranceStatus, PriceFeedConfig, PrivacyMode, RJoule, RateLimitConfig,
    TransactionType, WalletBalance, WalletConfig, WalletError, WalletTransaction,
};
