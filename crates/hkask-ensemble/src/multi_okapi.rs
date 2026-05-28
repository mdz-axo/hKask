//! Multi-Okapi Failover — Simplified
//!
//! Minimal stub for multi-Okapi support. Full implementation deferred to v1.1.
//! Configuration-driven via multi-okapi.yaml manifest.
//! ℏKask v0.21.2
//!
//! **Note**: Core types are re-exported from hkask-templates to eliminate duplication.

// Re-export canonical Multi-Okapi types from hkask-templates
pub use hkask_templates::multi_okapi::{
    FailoverConfig, HealthCheckConfig, HealthStatus, MultiOkapiConfig, MultiOkapiManager,
    OkapiInstance, OkapiInstanceConfig, RoutingConfig, load_multi_okapi_config,
};
