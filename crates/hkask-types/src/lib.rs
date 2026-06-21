//! hKask Types — Foundation types for the hKask agent platform
//!

pub mod cns;
pub mod crypto;

pub mod error;
pub mod event;

pub mod id;
pub mod kanban;


pub mod secret;
pub mod template;
pub mod template_type;
pub mod text;
pub mod time;

pub mod visibility;

pub mod wallet;

#[cfg(feature = "sql")]
pub mod sql_impls;

// ── Essential re-exports (used by ≥3 downstream crates) ─────────────────

pub use cns::CircuitState;
pub use crypto::Ed25519PublicKey;

pub use error::{InfrastructureError, McpErrorKind};
pub use event::{NuEvent, NuEventSink};

pub use id::{
    ApiKeyId, BoardId, BotID, ColumnId, CommentId, EmbeddingID, EscalationID, EventID, GoalID, Id,
    IdKind, PhaseId, PodID, TaskId, TemplateID, TripleID, UserID, WalletId, WebID,
};
pub use kanban::{
    Board, CapabilityPackage, ColumnDef, Comment, ConditionResult, ConsentProof, ContractState,
    ContractVerification, Phase, Priority, SpawnSpec, Task, TaskContract, TaskFilter, TaskSpec,
    TaskStatus, Verification, VerificationCriterion,
};

pub use visibility::{Confidence, Visibility};
pub use wallet::{
    ApiKeyCapability, ApiKeyMaterial, ChainId, DepositAddress, DepositReference, Encumbrance,
    EncumbranceStatus, PriceFeedConfig, PrivacyMode, RJoule, RateLimitConfig, TransactionType,
    WalletBalance, WalletConfig, WalletError, WalletTransaction,
};
