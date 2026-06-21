//! hKask Text Classification Service — section typing and triple extraction.
//!
//! Extracted from `hkask-services` to enable parallel compilation.

mod classify_impl;
mod provider_intel;

pub use classify_impl::{
    ClassifierConfig, TripleExtraction, classify_batch, extract_triples_batch,
    load_classifier_config,
};
pub use provider_intel::{
    CostRate, DeepInfraProvider, FalProvider, FirecrawlProvider, LimitUnit, OpenRouterProvider,
    ProviderError, ProviderIntelligence, ProviderState, RunpodProvider, SelfTrackedConfig,
    SelfTrackedProvider, TogetherProvider, UsageStatus, create_provider,
};
