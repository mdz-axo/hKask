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

// G2 Justification: This module exposes 50 public items because it is a re-export facade for downstream crates. Each re-export maps to a core domain type used by ≥3 downstream crates. Submodule reorganization planned for v0.28.0.

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
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::bundle::BundleComplementarity instead"
)]
pub use bundle::BundleComplementarity;
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::bundle::BundleConflict instead"
)]
pub use bundle::BundleConflict;
pub use bundle::BundleManifest;
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::bundle::BundleManifestStep instead"
)]
pub use bundle::BundleManifestStep;
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::bundle::BundleSkill instead"
)]
pub use bundle::BundleSkill;
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::bundle::SkillPolarity instead"
)]
pub use bundle::SkillPolarity;
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
    ApiKeyId, BotID, EmbeddingID, EscalationID, EventID, GoalID, Id, IdKind, PodID, TemplateID,
    TripleID, UserID, WalletId, WebID,
};

#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::identity::HumanUser instead"
)]
pub use identity::HumanUser;
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::identity::RegistrationRequest instead"
)]
pub use identity::RegistrationRequest;
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::identity::ReplicantIdentity instead"
)]
pub use identity::ReplicantIdentity;
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::identity::UserSession instead"
)]
pub use identity::UserSession;

#[deprecated(since = "0.28.0", note = "Use hkask_types::lexicon::HLexicon instead")]
pub use lexicon::HLexicon;
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::lexicon::LexiconTerm instead"
)]
pub use lexicon::LexiconTerm;
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::lexicon::TemplateType instead"
)]
pub use lexicon::TemplateType;
pub use loops::{CurationInput, CuratorHandle, ExperienceClassification};

pub use ports::{
    BundleRegistryIndex, CircuitBreakerPort, EmbeddingGenerationError, InferenceError,
    InferencePort, InferenceResult, InferenceStreamChunk, InferenceUsage, RegistryEntry,
    RegistryError, RegistryIndex, Skill, SkillRegistryIndex, SkillZone, StructuredToolCall,
    ToolInfo, ToolPort, ToolPortError,
};
#[deprecated(since = "0.28.0", note = "Use hkask_types::r7::R7BotIdentity instead")]
pub use r7::R7BotIdentity;
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::r7::default_r7_bots instead"
)]
pub use r7::default_r7_bots;

#[deprecated(since = "0.28.0", note = "Use hkask_types::secret::SecretRef instead")]
pub use secret::SecretRef;
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::secret::ZeroizingSecret instead"
)]
pub use secret::ZeroizingSecret;
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::secret::derivation_contexts instead"
)]
pub use secret::derivation_contexts;

#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::sovereignty::DataCategory instead"
)]
pub use sovereignty::DataCategory;
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::sovereignty::UserSovereigntyState instead"
)]
pub use sovereignty::UserSovereigntyState;

#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::template::LLMParameters instead"
)]
pub use template::LLMParameters;
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::template::TemplateCrate instead"
)]
pub use template::TemplateCrate;
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::template::TemplateFile instead"
)]
pub use template::TemplateFile;

#[deprecated(since = "0.28.0", note = "Use hkask_types::text::blake3_hash instead")]
pub use text::blake3_hash;

#[deprecated(since = "0.28.0", note = "Use hkask_types::time::now_rfc3339 instead")]
pub use time::now_rfc3339;

#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::visibility::AccessControl instead"
)]
pub use visibility::AccessControl;
pub use visibility::Confidence;
#[deprecated(
    since = "0.28.0",
    note = "Use hkask_types::visibility::TemporalBounds instead"
)]
pub use visibility::TemporalBounds;
pub use visibility::Visibility;
pub use wallet::{
    ApiKeyCapability, ApiKeyMaterial, ChainId, DepositAddress, DepositReference, Ed25519PublicKey,
    Encumbrance, EncumbranceStatus, PrivacyMode, RJoule, RateLimitConfig, TransactionType,
    WalletBalance, WalletConfig, WalletError, WalletTransaction,
};
