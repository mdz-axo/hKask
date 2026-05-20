//! hKask Ensemble — Multi-agent chat coordination

pub mod adapters;
pub mod capability;
pub mod chat;
pub mod cns_spans;
pub mod confidence_router;
pub mod deliberation;
pub mod macaroon;
pub mod metrics;
pub mod multi_okapi;
pub mod ocap_enforcement;
pub mod okapi_integration;
pub mod ports;
pub mod resilience;
pub mod webid_registry;

pub use adapters::{
    MockCapabilityProvider, MockInferenceClient, MockMetricsSource, OkapiAdapterError,
    OkapiCapabilityFetcher, OkapiHttpClient, OkapiSseAdapter,
};
pub use capability::{
    default_system_capability, read_only_capability, AuthorizationError, CapabilityId,
    OkapiCapability, OkapiOperation,
};
pub use cns_spans::{OkapiCnsSpan, ValidationResult};
pub use confidence_router::{
    ConfidenceConfig, ConfidenceRouter, LegacyRouterError, OkapiClient, OkapiClientTrait,
    OkapiResponse, RouterError, compute_confidence,
};
pub use multi_okapi::{
    CapabilityRouter, HealthChecker, HealthStatus, MultiOkapiClient, OkapiInstance,
};
pub use ocap_enforcement::{
    OcapEnforcer, OcapEnforcementResult, OcapContext, enforce_okapi_ocap,
};
pub use okapi_integration::{OkapiIntegration, OkapiIntegrationError};
pub use ports::{
    CapabilityProvider, GenerateRequest, GenerateOptions, GenerateResponse, InferenceClient,
    MetricsSource, OkapiCapabilities, OkapiMetrics, TokenProb, TokenProbability,
};
pub use resilience::{
    CircuitBreaker, CircuitBreakerConfig, CircuitState, CircuitBreakerStats,
    RetryConfig, RetryError, ResilientOkapiClient, retry_with_backoff,
};
pub use metrics::{MetricsRegistry, CounterMetric, GaugeMetric, HistogramMetric, OkapiMetricsCollector};
pub use webid_registry::{WebIDCapabilityRegistry, WebIDCapabilityEntry, RegistryError, authorize_operation};
pub use macaroon::{Macaroon, MacaroonBuilder, Caveat, CaveatContext, MacaroonError};
