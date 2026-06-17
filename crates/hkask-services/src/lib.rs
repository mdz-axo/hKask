//! hKask Service Layer — shared startup infrastructure and domain operations.
//!
//! Foundation types (`ServiceError`, `ServiceConfig`, `HkaskSettings`) live in
//! `hkask-services-core` and are re-exported here for backward compatibility.
//!
//! # Module visibility
//!
//! Modules marked **Public API** are the stable surface. Prefer the re-exported
//! paths at crate root (e.g. `hkask_services::BackupService`) over the full
//! module path (`hkask_services::backup::BackupService`).
//!
//! Modules marked **Internal** are accessible via their full path but are not
//! part of the committed public API. They may change without semver notice.

// ── Re-exports from hkask-services-core ───────────────────────────────

pub use hkask_services_backup::config::{
    BackupConfig, EncryptionConfig, RetentionPolicy, backup_config_path, load_backup_config,
};
pub use hkask_services_backup::r#loop::BackupLoop;
pub use hkask_services_backup::metadata::{PruneReport, SnapshotMetadata, SnapshotTrigger};
pub use hkask_services_backup::scope::ArtifactType;
pub use hkask_services_backup::scope::{BackupScope, ListFilter, RestoreScope};
pub use hkask_services_backup::serialization::{
    ArtifactEnvelopeValue, artifact_git_path, deserialize_artifact, serialize_artifact,
};
pub use hkask_services_backup::{BackupError, BackupService};
pub use hkask_services_core::config::{DEFAULT_DB_PATH, ServiceConfig};
pub use hkask_services_core::error::ServiceError;
pub use hkask_services_core::settings::{
    HkaskSettings, load_settings, save_settings, settings_path,
};
pub use hkask_services_kanban::{KanbanError, KanbanService, UnjamFix, UnjamItem};
pub use hkask_services_kata::{
    ImprovementDirection, ImprovementSignal, KataEngine, KataError, KataHistory, KataManifest,
    KataResult, KataState, KataStep, PracticeEntry, StepExperience,
};

// ── Public API modules ─────────────────────────────────────────────────

pub mod bundle;
pub mod chat;
pub mod cns;
pub mod compose;
pub mod contacts;
pub mod context;
pub mod curator;
pub mod deletion_test;
pub mod discover;
pub mod embed;
pub mod experience;
pub mod goal;
pub mod inference;
pub mod kanban;
pub mod kata;
pub mod lifecycle;
pub mod onboarding;
pub mod pods;
pub mod scheduler;
pub mod skill;
pub mod skills;
pub mod sovereignty;
pub mod spec;
pub mod verification;
pub mod wallet;

// ── Internal modules (accessible, not part of committed API) ───────────

pub mod archival;
pub mod classify;
pub mod consolidation;
pub mod daemon_handler;

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
pub use context::{AgentService, PerAgentMemory};
pub use curator::{CuratorService, EscalationResponse};
pub use deletion_test::DeletionTest;
pub use discover::{
    DiscoverRequest, DiscoverResult, DiscoveredWork, DiscoveryService, default_corpus_config,
    download_and_cache, generate_corpus_yaml, slugify,
};
pub use embed::{
    ChunkingConfig, CorpusConfig, EmbedPhase, EmbedProgress, EmbedResult, EmbedService,
    EmbeddingConfig, Entity, EntityConfig, FoundationalRule, ProgressFn, ValidationConfig, Work,
};
pub use experience::CliExperienceRecorder;
pub use goal::{CreateGoalRequest, GoalResponse, GoalService};
pub use inference::{InferenceContext, InferenceService, ModelInfo};
pub use kanban::{KanbanError, KanbanService, UnjamFix, UnjamItem};
pub use kata::{
    ImprovementDirection, ImprovementSignal, KataEngine, KataError, KataHistory, KataManifest,
    KataResult, KataState, KataStep, PracticeEntry, StepExperience,
};
pub use lifecycle::{
    LifecycleError, ServerHealth, ServerLifecycle, ServerLifecycleConfig, run_lifecycle,
};
pub use onboarding::{
    MatrixRegistrationResult, OnboardingService, RegistryHandle, ReplicantContactConfig,
    ResolvedSecrets, SignInOutcome, conduit_health_check,
};
pub use pods::{CreatePodRequest, PodResponse, PodService, PodStatusResponse};
pub use scheduler::SchedulerService;
pub use skill::{
    SkillInfo, SkillPublishResult, compute_file_hash, discover_skills, find_public_skill,
    publish_skill, read_skill_namespace, read_skill_visibility, resolve_replicant_name,
};
pub use skills::{
    SkillAuditError, SkillAuditReport, SkillAuditor, SkillHealthScore, SkillStatus, TemplateSummary,
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
