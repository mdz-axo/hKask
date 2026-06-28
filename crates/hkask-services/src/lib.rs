//! hKask Service Layer — shared startup infrastructure and domain operations.
//!
//! Foundation types live in `hkask-services-core`.
//!
//! # Module visibility
//!
//! Modules marked **Public API** are the stable surface. Prefer the re-exported
//! paths at crate root (e.g. `hkask_services::AgentService`) over the full
//! module path (`hkask_services::context::AgentService`).
//!
//! Modules marked **Internal** are accessible via their full path but are not
//! part of the committed public API. They may change without semver notice.

// ── Re-exports from extracted service crates ───────────────────────────

pub use hkask_agents::consent::ConsentManager;
pub use hkask_inference::model_constants;
pub use hkask_inference::{
    FusionConfig, FusionMode, FusionSkill, InferenceConfig, InferenceRouter, ProviderId,
};
pub use hkask_services_chat::chat::{
    ChatRequest, ChatResponse, ChatService, PreparedChat, TokenUsage, TurnRequest, TurnResult,
};
pub use hkask_services_chat::memory::MemoryService;
pub use hkask_services_compose::{
    CentroidValidation, CognitionConfig, ComposeRequest, ComposeResult, ComposeService,
    EmbeddingSection, RetrievalSection, ValidationSection, cosine_distance,
};
pub use hkask_services_context::{AgentService, PerAgentMemory};
pub use hkask_services_core::config::{DEFAULT_DB_PATH, ServiceConfig};
pub use hkask_services_core::error::ServiceError;
pub use hkask_services_core::parse_data_category;
pub use hkask_services_core::settings::{
    HkaskSettings, load_settings, save_settings, settings_path,
};
pub use hkask_services_core::verification::{
    Assertion, AssertionResult, Manifest, PrincipleResult, VerificationReport, VerificationService,
};
pub use hkask_services_core::{InferenceContext, InferenceService, ModelInfo};
pub use hkask_services_corpus::{
    ChunkingConfig, CorpusConfig, DiscoverRequest, DiscoverResult, DiscoveredWork,
    DiscoveryService, EmbedPhase, EmbedProgress, EmbedResult, EmbedService, EmbeddingConfig,
    Entity, EntityConfig, FoundationalRule, ProgressFn, ValidationConfig, Work,
    default_corpus_config, download_and_cache, generate_corpus_yaml, slugify,
};
pub use hkask_services_curator::{CuratorService, EscalationResponse};
pub use hkask_services_kanban::{KanbanError, KanbanService, UnjamFix, UnjamItem};
pub use hkask_services_kata::{
    ImprovementDirection, ImprovementSignal, KataEngine, KataError, KataHistory, KataManifest,
    KataResult, KataState, KataStep, PracticeEntry, StepExperience,
};
pub use hkask_services_onboarding::{
    MatrixRegistrationResult, OnboardingService, RegistryHandle, ReplicantContactConfig,
    ResolvedSecrets, SignInOutcome, conduit_ensure_healthy, conduit_health_check,
};
pub use hkask_services_runtime::{
    AdaptiveMonitor, ClassifierConfig, CostRate, DeepInfraProvider, FalProvider, FirecrawlProvider,
    LimitUnit, OpenRouterProvider, ProviderError, ProviderIntelligence, ProviderState,
    RunpodProvider, SelfTrackedConfig, SelfTrackedProvider, ServiceDaemonHandler, TogetherProvider,
    TripleExtraction, UsageStatus, classify_batch, create_provider, extract_triples_batch,
    generate_raw, load_classifier_config,
};
pub use hkask_services_skill::resolve_replicant_name;
pub mod skill;
pub use hkask_services_wallet::WalletService;

// ── Remaining inline modules ───────────────────────────────────────────

#[cfg(test)]
mod tests {
    /// Extract dependency names starting with `hkask-services-` from the
    /// `[dependencies]` section of Cargo.toml via naive line-scanning.
    fn parse_service_deps(toml: &str) -> Vec<String> {
        let mut in_deps = false;
        let mut deps = Vec::new();
        for line in toml.lines() {
            let trimmed = line.trim();
            if trimmed == "[dependencies]" {
                in_deps = true;
                continue;
            }
            if in_deps && trimmed.starts_with('[') {
                break;
            }
            if in_deps && let Some(name) = trimmed.split('=').next() {
                let name = name.trim();
                if name.starts_with("hkask-services-") {
                    deps.push(name.to_string());
                }
            }
        }
        deps
    }

    fn is_reexported(dep_name: &str, lib_source: &str) -> bool {
        let module_name = dep_name.replace('-', "_");
        let needle = format!("pub use {}::", module_name);
        lib_source.contains(&needle)
    }

    #[test]
    fn all_service_deps_are_reexported() {
        let cargo_toml = include_str!("../Cargo.toml");
        let lib_source = include_str!("../src/lib.rs");

        let service_deps = parse_service_deps(cargo_toml);
        assert!(
            !service_deps.is_empty(),
            "expected at least one hkask-services-* dependency"
        );

        let missing: Vec<_> = service_deps
            .iter()
            .filter(|dep| !is_reexported(dep, lib_source))
            .collect();

        assert!(
            missing.is_empty(),
            "{} hkask-services-* dep(s) missing re-exports:\n  {}\n\n\
             Add `pub use <crate_name>::...` to lib.rs for each.",
            missing.len(),
            missing
                .iter()
                .map(|d| d.as_str())
                .collect::<Vec<_>>()
                .join("\n  ")
        );
    }
}
