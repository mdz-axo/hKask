//! hKask Service Layer — shared startup infrastructure and domain operations.
//!
//! Foundation types live in `hkask-services-core`.
//!
//! # Module visibility
//!
//! Modules marked **Public API** are the stable surface. Prefer the re-exported
//! paths at crate root (e.g. `hkask_services::BackupService`) over the full
//! module path (`hkask_services::backup::BackupService`).
//!
//! Modules marked **Internal** are accessible via their full path but are not
//! part of the committed public API. They may change without semver notice.

// ── Re-exports from extracted service crates ───────────────────────────

pub use crate::lifecycle::{
    LifecycleError, ServerHealth, ServerLifecycle, ServerLifecycleConfig, run_lifecycle,
};
pub use hkask_agents::consent::ConsentManager;
pub use hkask_inference::model_constants;
pub use hkask_inference::{FusionConfig, InferenceConfig, InferenceRouter};
pub use hkask_services_backup::config::{
    BackupConfig, EncryptionConfig, RetentionPolicy, backup_config_path, load_backup_config,
};
pub use hkask_services_backup::r#loop::BackupLoop;
pub use hkask_services_backup::metadata::{PruneReport, SnapshotMetadata, SnapshotTrigger};
pub use hkask_services_backup::pod_ops::{PodBackupCap, PodBackupOps};
pub use hkask_services_backup::scope::ArtifactType;
pub use hkask_services_backup::scope::{BackupScope, ListFilter, RestoreScope};
pub use hkask_services_backup::serialization::{
    ArtifactEnvelopeValue, artifact_git_path, deserialize_artifact, serialize_artifact,
};
pub use hkask_services_backup::{BackupError, BackupService};
pub use hkask_services_context::{AgentService, PerAgentMemory};
pub use hkask_services_core::config::{DEFAULT_DB_PATH, ServiceConfig};
pub use hkask_services_core::error::ServiceError;
pub use hkask_services_core::settings::{
    HkaskSettings, load_settings, save_settings, settings_path,
};
pub use hkask_services_core::{InferenceContext, InferenceService, ModelInfo};
pub use hkask_services_discover::{
    DiscoverRequest, DiscoverResult, DiscoveredWork, DiscoveryService, default_corpus_config,
    download_and_cache, generate_corpus_yaml, slugify,
};
pub use hkask_services_embed::{
    ChunkingConfig, CorpusConfig, EmbedPhase, EmbedProgress, EmbedResult, EmbedService,
    EmbeddingConfig, Entity, EntityConfig, FoundationalRule, ProgressFn, ValidationConfig, Work,
};
pub use hkask_services_kanban::{KanbanError, KanbanService, UnjamFix, UnjamItem};
pub use hkask_services_kata::{
    ImprovementDirection, ImprovementSignal, KataEngine, KataError, KataHistory, KataManifest,
    KataResult, KataState, KataStep, PracticeEntry, StepExperience,
};
pub use hkask_services_onboarding::{
    MatrixRegistrationResult, OnboardingService, RegistryHandle, ReplicantContactConfig,
    ResolvedSecrets, SignInOutcome, conduit_ensure_healthy, conduit_health_check,
};
pub use hkask_services_skill::resolve_replicant_name;
pub use hkask_services_verification::{
    Assertion, AssertionResult, Manifest, PrincipleResult, VerificationReport, VerificationService,
};
pub use hkask_services_wallet::WalletService;

// ── Remaining inline modules ───────────────────────────────────────────

pub mod bundle;
pub mod chat;
pub mod cloud;
pub mod lifecycle;
pub mod memory;
pub use memory::MemoryService;

pub mod cns;
pub mod compose;
pub mod contacts;
pub mod curator;
pub mod federation;

pub mod experience;
pub mod goal;
pub mod pods;
pub mod scheduler;
pub mod skill;
pub mod skills;
pub mod spec;

// ── Internal modules ───────────────────────────────────────────────────

pub mod archival;
pub mod consolidation;

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
pub use contacts::ContactService;
pub use curator::{CuratorService, EscalationResponse};
pub use federation::FederationService;

pub use experience::CliExperienceRecorder;
pub use goal::{CreateGoalRequest, GoalResponse, GoalService};
pub use pods::{CreatePodRequest, PodResponse, PodService, PodStatusResponse};
pub use scheduler::SchedulerService;
pub use skills::{
    SkillAuditError, SkillAuditReport, SkillAuditor, SkillHealthScore, SkillStatus, TemplateSummary,
};
pub use spec::{
    CoherenceResult, SpecCaptureRequest, SpecCaptureResponse, SpecDetail, SpecListEntry,
    SpecService, WritingQualityResult,
};
