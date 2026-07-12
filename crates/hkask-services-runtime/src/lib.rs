//! hKask Runtime Services — text classification, provider intelligence, daemon handler.
//!
//! Merged from `hkask-services-classify` and `hkask-services-daemon`.

mod adaptive_monitor;
mod classify_impl;
mod daemon_impl;
mod dual_classify;
pub mod guard;
mod provider_intel;

pub use adaptive_monitor::AdaptiveMonitor;
pub use classify_impl::{
    ClassifierConfig, TripleExtraction, classify_batch, extract_triples_batch, generate_raw,
    load_classifier_config,
};
pub use daemon_impl::ServiceDaemonHandler;
pub use dual_classify::{
    DualClassifierConfig, DualClassifyResult, DualTripleExtraction, IntegratedExtraction,
    build_dual_config, classify_dual_batch, extract_triples_dual_batch, integrate_dual_triples,
};
pub use guard::{ContentGuard, GuardResult, GuardViolation};
pub use provider_intel::{
    CostRate, DeepInfraProvider, FalProvider, FirecrawlProvider, LimitUnit, OpenRouterProvider,
    ProviderError, ProviderIntelligence, ProviderState, RunpodProvider, SelfTrackedConfig,
    SelfTrackedProvider, TogetherProvider, UsageStatus, create_provider,
};
