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
    AuthorizationError, CapabilityId, OkapiCapability, OkapiOperation, default_system_capability,
    read_only_capability,
};
pub use cns_spans::{OkapiCnsSpan, ValidationResult};
pub use confidence_router::{
    ConfidenceConfig, ConfidenceRouter, LegacyRouterError, OkapiClient, OkapiClientTrait,
    OkapiResponse, RouterError, compute_confidence,
};
pub use macaroon::{Caveat, CaveatContext, Macaroon, MacaroonBuilder, MacaroonError};
pub use metrics::{
    CounterMetric, GaugeMetric, HistogramMetric, MetricsRegistry, OkapiMetricsCollector,
};
pub use multi_okapi::{
    CapabilityRouter, HealthChecker, HealthStatus, MultiOkapiClient, OkapiInstance,
};
pub use ocap_enforcement::{OcapContext, OcapEnforcementResult, OcapEnforcer, enforce_okapi_ocap};
pub use okapi_integration::{OkapiIntegration, OkapiIntegrationError};
pub use ports::{
    CapabilityProvider, GenerateOptions, GenerateRequest, GenerateResponse, InferenceClient,
    MetricsSource, OkapiCapabilities, OkapiMetrics, TokenProb, TokenProbability,
};
pub use resilience::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerStats, CircuitState, ResilientOkapiClient,
    RetryConfig, RetryError, retry_with_backoff,
};
pub use webid_registry::{
    RegistryError, WebIDCapabilityEntry, WebIDCapabilityRegistry, authorize_operation,
};
