//! hKask Service Layer — shared startup infrastructure and domain operations.

pub mod archival;
pub mod chat;
pub mod cns;
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
pub mod settings;
pub mod skill;
pub mod spec;
pub mod verification;

pub use archival::{ArchivalService, ArchiveResult, SnapshotResult};
pub use chat::{
    ChatRequest, ChatResponse, ChatService, PreparedChat, TokenUsage, TurnRequest, TurnResult,
};
pub use cns::CnsService;
pub use compose::{
    CentroidValidation, CognitionConfig, ComposeRequest, ComposeResult, ComposeService,
    EmbeddingSection, RetrievalSection, ValidationSection, cosine_distance,
};
pub use config::ServiceConfig;
pub use config::{DEFAULT_DB_PATH, DEFAULT_OKAPI_BASE_URL};
pub use context::{AgentService, PerAgentMemory};
pub use curator::{CuratorService, EscalationResponse};
pub use embed::{
    ChunkingConfig, CorpusConfig, EmbedResult, EmbedService, EmbeddingConfig, FoundationalRule,
    ValidationConfig, Work,
};
pub use ensemble::{CyberneticsLoopGasAdapter, EnsembleService};
pub use error::ServiceError;
pub use goal::{CreateGoalRequest, GoalResponse, GoalService};
pub use inference::{InferenceContext, InferenceService, ModelInfo};
pub use onboarding::{OnboardingService, RegistryHandle, ResolvedSecrets, SignInOutcome};
pub use pods::{CreatePodRequest, PodResponse, PodService, PodStatusResponse};
pub use settings::settings_path;
pub use skill::{
    SkillInfo, SkillPublishResult, compute_file_hash, discover_skills, find_public_skill,
    publish_skill, read_skill_namespace, read_skill_visibility,
};
pub use spec::{
    CoherenceResult, SpecCaptureRequest, SpecCaptureResponse, SpecDetail, SpecListEntry,
    SpecService, WritingQualityResult,
};
pub use verification::{
    Assertion, AssertionResult, Manifest, PrincipleResult, VerificationReport, VerificationService,
};
