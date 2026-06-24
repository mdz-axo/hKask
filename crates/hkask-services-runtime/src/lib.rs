//! hKask Runtime Services — text classification, provider intelligence, daemon handler.
//!
//! Merged from `hkask-services-classify` and `hkask-services-daemon`.

mod adaptive_monitor;
mod classify_impl;
mod daemon_impl;
mod provider_intel;

pub use adaptive_monitor::AdaptiveMonitor;
pub use classify_impl::{
    ClassifierConfig, TripleExtraction, classify_batch, extract_triples_batch, generate_raw,
    load_classifier_config,
};
pub use daemon_impl::ServiceDaemonHandler;
pub use provider_intel::{
    CostRate, DeepInfraProvider, FalProvider, FirecrawlProvider, LimitUnit, OpenRouterProvider,
    ProviderError, ProviderIntelligence, ProviderState, RunpodProvider, SelfTrackedConfig,
    SelfTrackedProvider, TogetherProvider, UsageStatus, create_provider,
};
