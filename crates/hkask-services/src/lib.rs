//! hKask Service Layer — shared startup infrastructure and domain operations.

pub mod agent;
pub mod archival;
pub mod chat;
pub mod compose;
pub mod config;
pub mod consolidation;
pub mod context;
pub mod embed;
pub mod ensemble;
pub mod error;
pub mod inference;
pub mod onboarding;
pub mod skill;
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
pub use embed::{
    ChunkingConfig, CorpusConfig, EmbedResult, EmbedService, EmbeddingConfig, FoundationalRule,
    ValidationConfig, Work,
};
pub use ensemble::{EnsembleContext, EnsembleService, ParticipantInfo, map_participant_role};
pub use error::ServiceError;
pub use inference::{InferenceContext, InferenceService, ModelInfo};
pub use onboarding::{OnboardingService, RegistryHandle, ResolvedSecrets, SignInOutcome};
pub use skill::{SkillInfo, SkillPublishResult, SkillService};
pub use spec::{CapturedSpec, EvaluatedSpec, SpecService};
pub use user::UserService;
pub use verification::{
    Assertion, AssertionResult, Manifest, PrincipleResult, VerificationReport, VerificationService,
};
