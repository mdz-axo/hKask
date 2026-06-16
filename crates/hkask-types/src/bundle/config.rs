//! Bundle configuration sub-structs — mirror existing manifest YAML fields
//!
//! These config structs are loaded from manifest YAML. Some are already
//! enforced at runtime (CNS spans via GovernedTool); others are future wiring targets.

use serde::{Deserialize, Serialize};

/// Loaded from manifest YAML. Not yet enforced by ManifestExecutor (future wiring target).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ConvergenceConfig {
    pub threshold: f64,
    pub max_iterations: u32,
    pub on_not_reached: String,
}

impl Default for ConvergenceConfig {
    fn default() -> Self {
        Self {
            threshold: 0.1,
            max_iterations: 3,
            on_not_reached: "abort".to_string(),
        }
    }
}

/// Gas (energy budget) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GasConfig {
    pub cap: u32,
    pub cost_per_token: f64,
    pub alert_threshold: f64,
    pub hard_limit: bool,
}
impl Default for GasConfig {
    fn default() -> Self {
        Self {
            cap: 10000,
            cost_per_token: 0.25,
            alert_threshold: 0.8,
            hard_limit: true,
        }
    }
}

/// Error handling configuration. Loaded from manifest YAML, future wiring target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ErrorHandlingConfig {
    pub on_gas_exceeded: String,
    pub on_timeout: String,
    pub max_retries: u32,
    pub retry_backoff_seconds: u32,
    pub on_validation_failure: String,
}
impl Default for ErrorHandlingConfig {
    fn default() -> Self {
        Self {
            on_gas_exceeded: "abort".into(),
            on_timeout: "retry".into(),
            max_retries: 2,
            retry_backoff_seconds: 1,
            on_validation_failure: "abort".into(),
        }
    }
}

/// OCAP configuration. Loaded from manifest YAML, future wiring target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OcapConfig {
    pub delegation_chain_required: bool,
    pub signature_algorithm: String,
    pub capability_expiry_seconds: u32,
    pub template_scoped: bool,
}
impl Default for OcapConfig {
    fn default() -> Self {
        Self {
            delegation_chain_required: true,
            signature_algorithm: "ed25519".into(),
            capability_expiry_seconds: 3600,
            template_scoped: true,
        }
    }
}

/// CNS monitoring configuration. Loaded from manifest YAML, spans handled by GovernedTool at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CnsConfig {
    pub emit_spans: bool,
    pub span_namespace: String,
    pub variety_monitoring: bool,
    pub algedonic_threshold: u32,
    pub escalation_target: String,
}
impl Default for CnsConfig {
    fn default() -> Self {
        Self {
            emit_spans: true,
            span_namespace: String::new(),
            variety_monitoring: true,
            algedonic_threshold: 100,
            escalation_target: "Curator".into(),
        }
    }
}

/// Audit trail configuration. Loaded from manifest YAML, future wiring target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_level: String,
    pub include_input: bool,
    pub include_output: bool,
    pub include_gas_cost: bool,
    pub include_cns_events: bool,
}
impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_level: "info".into(),
            include_input: true,
            include_output: true,
            include_gas_cost: true,
            include_cns_events: true,
        }
    }
}
