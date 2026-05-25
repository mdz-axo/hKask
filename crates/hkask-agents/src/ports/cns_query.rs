//! CNS Query Port — Hexagonal boundary for CNS observability
//!
//! This port trait decouples the MetacognitionLoop from the concrete CnsRuntime
//! implementation, following hexagonal architecture principles (R7).

use async_trait::async_trait;
use hkask_cns::{CnsHealth, RuntimeAlert};

/// Port trait for querying CNS observability data
///
/// Implementations:
/// - `CnsRuntime` (hkask-cns) — Production implementation
/// - Mock implementations for testing
#[async_trait]
pub trait CnsQueryPort: Send + Sync {
    /// Get current CNS health status
    async fn health(&self) -> CnsHealth;

    /// Get variety counters for all domains
    async fn variety(&self) -> Vec<(String, u64)>;

    /// Get all algedonic alerts
    async fn alerts(&self) -> Vec<RuntimeAlert>;

    /// Get critical alerts only
    async fn critical_alerts(&self) -> Vec<RuntimeAlert>;

    /// Get variety counter for specific domain
    async fn variety_for_domain(&self, domain: &str) -> u64;
}
