//! hKask Service Layer — shared domain operations for CLI and API surfaces.
//!
//! This crate provides a unified service layer that eliminates duplication
//! between `hkask-cli` and `hkask-api`. Each surface composes a
//! `ServiceContext` (assembled once at startup) and delegates business logic
//! to domain service modules.
//!
//! # Architecture
//!
//! ```text
//! hkask-cli ──→ hkask-services ──→ hkask-agents
//! hkask-api  ──→ hkask-services ──→ hkask-cns
//!                                 ──→ hkask-memory
//!                                 ──→ hkask-templates
//!                                 ──→ hkask-types
//!                                 ──→ hkask-storage
//! ```
//!
//! Domain crates NEVER depend on `hkask-services`. Neither `hkask-cli` nor
//! `hkask-api` directly depend on domain crates for business operations.

pub mod agent;
pub mod archival;
pub mod chat;
pub mod compose;
pub mod config;
pub mod consolidation;
pub mod context;
pub mod curator;
pub mod embed;
pub mod ensemble;
pub mod error;
pub mod goal;
pub mod inference;
pub mod onboarding;
pub mod pods;
pub mod skill;
pub mod sovereignty;
pub mod spec;
pub mod user;
pub mod verification;

pub use agent::{AgentReceipt, AgentService};
pub use archival::{ArchivalService, ArchiveResult, SnapshotResult};
pub use chat::{ChatRequest, ChatResponse, ChatService, PreparedChat, TokenUsage};
pub use compose::{
    CentroidValidation, CognitionConfig, ComposeRequest, ComposeResult, ComposeService,
    EmbeddingSection, RetrievalSection, ValidationSection, cosine_distance,
};
pub use config::ServiceConfig;
pub use config::{DEFAULT_DB_PATH, DEFAULT_OKAPI_BASE_URL};
pub use consolidation::ConsolidationService;
pub use context::ServiceContext;
pub use curator::{CuratorContext, CuratorService, MetacognitionSummary};
pub use embed::{
    ChunkingConfig, CorpusConfig, EmbedResult, EmbedService, EmbeddingConfig, FoundationalRule,
    ValidationConfig, Work,
};
pub use ensemble::{EnsembleContext, EnsembleService, ParticipantInfo, map_participant_role};
pub use error::ServiceError;
pub use goal::{GoalContext, GoalService};
pub use inference::{InferenceContext, InferenceService, ModelInfo};
pub use onboarding::{OnboardingService, RegistryHandle, ResolvedSecrets, SignInOutcome};
pub use pods::{PodContext, PodService};
pub use skill::{SkillInfo, SkillPublishResult, SkillService};
pub use sovereignty::{
    AccessCheck, SovereigntyContext, SovereigntyService, SovereigntyStatus, parse_data_category,
};
pub use spec::{CapturedSpec, EvaluatedSpec, SpecService};
pub use user::UserService;
pub use verification::{
    Assertion, AssertionResult, Manifest, PrincipleResult, VerificationReport, VerificationService,
};
