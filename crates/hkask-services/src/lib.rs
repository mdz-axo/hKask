//! hKask Service Layer — shared startup infrastructure and domain operations.

pub mod archival;
pub mod bundle;
pub mod chat;
pub mod cns;
pub mod compose;
pub mod config;
pub mod consolidation;
pub mod contacts;
pub mod context;
pub mod curator;
pub mod daemon_handler;
pub mod discover;
pub mod embed;
pub mod ensemble;
pub mod error;
pub mod goal;
pub mod inference;
pub mod onboarding;
pub mod pods;
pub mod scheduler;
pub mod settings;
pub mod skill;
pub mod sovereignty;
pub mod spec;
pub mod verification;
pub mod wallet;

pub use archival::{ArchivalService, ArchiveResult, SnapshotResult};
pub use bundle::{BundleComposeResult, BundleService};
pub use chat::{
    ChatRequest, ChatResponse, ChatService, PreparedChat, TokenUsage, TurnRequest, TurnResult,
};
pub use cns::CnsService;
pub use compose::{
    CentroidValidation, CognitionConfig, ComposeRequest, ComposeResult, ComposeService,
    EmbeddingSection, RetrievalSection, ValidationSection, cosine_distance,
};
pub use config::DEFAULT_DB_PATH;
pub use config::ServiceConfig;
pub use contacts::ContactService;
pub use context::{AgentService, PerAgentMemory};
pub use curator::{CuratorService, EscalationResponse};
pub use discover::{
    DiscoverRequest, DiscoverResult, DiscoveredWork, DiscoveryService, download_and_cache,
};
pub use embed::{
    ChunkingConfig, CorpusConfig, EmbedPhase, EmbedProgress, EmbedResult, EmbedService,
    EmbeddingConfig, Entity, EntityConfig, FoundationalRule, ProgressFn, ValidationConfig, Work,
};
pub use ensemble::{CyberneticsLoopGasAdapter, EnsembleService};
pub use error::ServiceError;
pub use goal::{CreateGoalRequest, GoalResponse, GoalService};
pub use inference::{InferenceContext, InferenceService, ModelInfo};
pub use onboarding::{
    OnboardingService, RegistryHandle, ReplicantContactConfig, ResolvedSecrets, SignInOutcome,
};
pub use pods::{CreatePodRequest, PodResponse, PodService, PodStatusResponse};
pub use scheduler::SchedulerService;
pub use settings::settings_path;
pub use skill::{
    SkillInfo, SkillPublishResult, compute_file_hash, discover_skills, find_public_skill,
    publish_skill, read_skill_namespace, read_skill_visibility, resolve_replicant_name,
};
pub use sovereignty::SovereigntyService;
pub use spec::{
    CoherenceResult, SpecCaptureRequest, SpecCaptureResponse, SpecDetail, SpecListEntry,
    SpecService, WritingQualityResult,
};
pub use verification::{
    Assertion, AssertionResult, Manifest, PrincipleResult, VerificationReport, VerificationService,
};
pub use wallet::WalletService;
