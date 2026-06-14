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

pub use transcript::{TimedWord, TranscriptBundle, TranscriptSegment};
pub use voice::VoiceDesign;

#[cfg(feature = "sql")]
pub mod sql_impls;

pub use agent_def::{
    AgentDefinition, AgentKind, Charter, Contact, PersonaConstraints, RegisteredAgent,
    Responsibility, Right, ScheduledTask, UserProfile,
};
// allosteric types deleted — MWC sigmoid added zero runtime-observable behavior.
pub use audit::{AuditEntry, AuditOutcome};
pub use bundle::{
    BundleComplementarity, BundleConflict, BundleManifest, BundleManifestStep, BundleSkill,
    CascadePhase, ComplementarityType, ConflictResolution, ConflictType, SkillPolarity,
};
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
pub use error::{GitError, InfrastructureError, McpErrorKind};
pub use event::{NuEvent, NuEventSink};
pub use goal::Goal;
pub use id::{
    ApiKeyId, BotID, EmbeddingID, EventID, GoalID, Id, IdKind, PodID, TemplateID, TripleID, UserID,
    WalletId, WebID,
};

pub use identity::{HumanUser, RegistrationRequest, ReplicantIdentity, UserSession};

pub use lexicon::{HLexicon, LexiconTerm, TemplateType};
pub use loops::{CurationInput, CuratorHandle, ExperienceClassification};

pub use ports::{
    BundleRegistryIndex, CircuitBreakerPort, EmbeddingGenerationError, InferenceError,
    InferencePort, InferenceResult, InferenceStreamChunk, InferenceUsage, RegistryEntry,
    RegistryError, RegistryIndex, Skill, SkillRegistryIndex, SkillZone, StructuredToolCall,
    ToolInfo, ToolPort, ToolPortError,
};
pub use r7::{R7BotIdentity, default_r7_bots};
pub use secret::{SecretRef, ZeroizingSecret, derivation_contexts};
pub use sovereignty::{DataCategory, UserSovereigntyState};
pub use template::{LLMParameters, TemplateCrate, TemplateFile};
pub use text::blake3_hash;
pub use time::now_rfc3339;
pub use visibility::{AccessControl, Confidence, TemporalBounds, Visibility};
pub use wallet::{
    ApiKeyCapability, ApiKeyMaterial, ChainId, DepositAddress, DepositReference, Ed25519PublicKey,
    Encumbrance, EncumbranceStatus, PrivacyMode, RJoule, RateLimitConfig, TransactionType,
    WalletBalance, WalletConfig, WalletError, WalletTransaction,
};
