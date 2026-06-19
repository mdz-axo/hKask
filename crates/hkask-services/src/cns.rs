//! CNS health, alerts, and variety queries.
//!
//! `CnsService` wraps the shared `CnsRuntime` from `AgentService`,
//! hiding the `Arc<RwLock<>>` pattern so callers don't repeat
//! `cns_runtime.read().await.xxx().await` at every call site.

use hkask_rsolidity::contract;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use hkask_cns::{CnsRuntime, RuntimeAlert, SetPoints, SetPointsConfig, load_set_points};
use hkask_types::cns::CnsHealth;
use hkask_types::event::SpanNamespace;

/// Service for CNS health checks, algedonic alerts, and variety counters.
///
/// Wraps the shared `CnsRuntime` behind a clean async interface.
/// Constructed during `AgentService::build()` — never created directly.
#[derive(Clone)]
pub struct CnsService {
    runtime: Arc<RwLock<CnsRuntime>>,
}

impl CnsService {
    /// Create from the shared CNS runtime.
    ///
    pub fn new(runtime: Arc<RwLock<CnsRuntime>>) -> Self {
        Self { runtime }
    }

    /// Current CNS health snapshot.
    ///
    pub async fn health(&self) -> CnsHealth {
        self.runtime.read().await.health().await
    }

    /// Active algedonic alerts.
    ///
    pub async fn alerts(&self) -> Vec<RuntimeAlert> {
        self.runtime.read().await.alerts().await
    }

    /// Variety counter snapshots keyed by canonical CNS namespace.
    ///
    pub async fn variety(&self) -> HashMap<SpanNamespace, u64> {
        self.runtime.read().await.variety().await
    }

    /// Get the current CNS set-points.
    ///
    /// Reads from the active runtime when available, falling back to
    /// defaults from environment (`HKASK_CNS_CONFIG`) or hard-coded values.
    ///
    pub fn get_set_points(&self) -> SetPoints {
        load_set_points()
    }

    /// Compute updated set-points from a partial config.
    ///
    /// Missing fields fall back to defaults. Does not persist —
    /// persistence to YAML and runtime update is a separate operation.
    ///
    pub fn update_set_points(&self, config: &SetPointsConfig) -> SetPoints {
        SetPoints::from_config(config)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_cns::DEFAULT_THRESHOLD;

    //
    // A freshly constructed CnsRuntime should report healthy status
    // with zero alerts and zero deficits. This is the baseline —
    // any deviation indicates a problem in CNS initialization.
    #[tokio::test]
    async fn health_returns_defaults_for_empty_runtime() {
        let runtime = Arc::new(RwLock::new(CnsRuntime::with_threshold(DEFAULT_THRESHOLD)));
        let svc = CnsService::new(runtime);

        let health = svc.health().await;
        assert!(health.healthy, "Fresh CNS runtime should be healthy");
        assert_eq!(health.overall_deficit, 0);
        assert_eq!(health.critical_count, 0);
        assert_eq!(health.warning_count, 0);
    }

    #[tokio::test]
    async fn alerts_returns_empty_for_fresh_runtime() {
        let runtime = Arc::new(RwLock::new(CnsRuntime::with_threshold(DEFAULT_THRESHOLD)));
        let svc = CnsService::new(runtime);

        let alerts = svc.alerts().await;
        assert!(alerts.is_empty(), "Fresh CNS runtime should have no alerts");
    }

    #[tokio::test]
    async fn variety_returns_empty_for_fresh_runtime() {
        let runtime = Arc::new(RwLock::new(CnsRuntime::with_threshold(DEFAULT_THRESHOLD)));
        let svc = CnsService::new(runtime);

        let variety = svc.variety().await;
        assert!(
            variety.is_empty(),
            "Fresh CNS runtime should have no variety domains"
        );
    }
}
