//! CNS Query Port — Hexagonal boundary for CNS observability
//!
//! This port trait decouples the MetacognitionLoop from the concrete CnsRuntime
//! implementation, following hexagonal architecture principles (R7).
//!
//! Domain-native types (`HealthStatus`, `AlertInfo`, `AlertLevel`) are defined
//! here so that consumers of the port do not need to depend on `hkask_cns`.

use async_trait::async_trait;

/// Domain-native alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

/// Domain-native alert information
#[derive(Debug, Clone)]
pub struct AlertInfo {
    pub domain: String,
    pub deficit: u64,
    pub threshold: u64,
    pub severity: AlertLevel,
    pub escalated: bool,
    pub message: String,
}

/// Domain-native CNS health status
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub overall_deficit: u64,
    pub critical_count: usize,
    pub warning_count: usize,
    pub healthy: bool,
}

/// Port trait for querying CNS observability data
///
/// Implementations:
/// - `CnsRuntimeAdapter` (hkask-agents) — Production adapter wrapping CnsRuntime
/// - Mock implementations for testing
#[async_trait]
pub trait CnsQueryPort: Send + Sync {
    async fn health(&self) -> HealthStatus;

    async fn variety(&self) -> Vec<(String, u64)>;

    async fn alerts(&self) -> Vec<AlertInfo>;

    async fn critical_alerts(&self) -> Vec<AlertInfo>;

    async fn variety_for_domain(&self, domain: &str) -> u64;
}
