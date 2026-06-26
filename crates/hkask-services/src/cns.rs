//! CNS health, alerts, and variety queries.
//!
//! `CnsService` wraps the shared `CnsRuntime` from `AgentService`,
//! hiding the `Arc<RwLock<>>` pattern so callers don't repeat
//! `cns_runtime.read().await.xxx().await` at every call site.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use hkask_cns::{CnsRuntime, RuntimeAlert, SetPoints, SetPointsConfig, load_set_points};
use hkask_types::cns::CnsHealth;
use hkask_types::event::SpanNamespace;

/// Service for CNS health checks, algedonic alerts, and variety counters.
///
/// Wraps the shared `CnsRuntime` behind a clean async interface.
/// Lightweight and freely cloneable — wraps an `Arc<RwLock<CnsRuntime>>`.
/// Constructed during `AgentService::build()` and also usable standalone
/// via `CnsService::new(runtime)`.
#[derive(Clone)]
pub struct CnsService {
    runtime: Arc<RwLock<CnsRuntime>>,
}

impl CnsService {
    /// Create from the shared CNS runtime.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  runtime must be a valid Arc<Rw`Lock<CnsRuntime>`>
    /// post: returns CnsService wrapping the runtime
    pub fn new(runtime: Arc<RwLock<CnsRuntime>>) -> Self {
        Self { runtime }
    }

    /// Current CNS health snapshot.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  runtime must be initialized
    /// post: returns CnsHealth with healthy flag, alert count, and deficit summary
    pub async fn health(&self) -> CnsHealth {
        self.runtime.read().await.health().await
    }

    /// Active algedonic alerts.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  runtime must be initialized
    /// post: returns `Vec<RuntimeAlert>` of currently active alerts; empty Vec if none
    pub async fn alerts(&self) -> Vec<RuntimeAlert> {
        self.runtime.read().await.alerts().await
    }

    /// Variety counter snapshots keyed by canonical CNS namespace.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  runtime must be initialized
    /// post: returns Hash`Map<SpanNamespace, u64>` of variety counters; empty map if no counters
    pub async fn variety(&self) -> HashMap<SpanNamespace, u64> {
        self.runtime.read().await.variety().await
    }

    /// Get the current CNS set-points.
    ///
    /// Reads from the active runtime when available, falling back to
    /// defaults from environment (`HKASK_CNS_CONFIG`) or hard-coded values.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  none (always succeeds)
    /// post: returns SetPoints from env config or hard-coded defaults
    pub fn get_set_points(&self) -> SetPoints {
        load_set_points()
    }

    /// Compute updated set-points from a partial config.
    ///
    /// Missing fields fall back to defaults. Does not persist —
    /// persistence to YAML and runtime update is a separate operation.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  config must be a valid SetPointsConfig; missing fields use defaults
    /// post: returns SetPoints computed from config merged with defaults
    pub fn update_set_points(&self, config: &SetPointsConfig) -> SetPoints {
        SetPoints::from_config(config)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_cns::DEFAULT_VARIETY_MAX_DEFICIT;

    //
    // A freshly constructed CnsRuntime should report healthy status
    // with zero alerts and zero deficits. This is the baseline —
    // any deviation indicates a problem in CNS initialization.
    #[tokio::test]
    async fn health_returns_defaults_for_empty_runtime() {
        let runtime = Arc::new(RwLock::new(CnsRuntime::with_threshold(
            DEFAULT_VARIETY_MAX_DEFICIT as u64,
        )));
        let svc = CnsService::new(runtime);

        let health = svc.health().await;
        assert!(health.healthy, "Fresh CNS runtime should be healthy");
        assert_eq!(health.overall_deficit, 0);
        assert_eq!(health.critical_count, 0);
        assert_eq!(health.warning_count, 0);
    }

    #[tokio::test]
    async fn alerts_returns_empty_for_fresh_runtime() {
        let runtime = Arc::new(RwLock::new(CnsRuntime::with_threshold(
            DEFAULT_VARIETY_MAX_DEFICIT as u64,
        )));
        let svc = CnsService::new(runtime);

        let alerts = svc.alerts().await;
        assert!(alerts.is_empty(), "Fresh CNS runtime should have no alerts");
    }

    #[tokio::test]
    async fn variety_returns_empty_for_fresh_runtime() {
        let runtime = Arc::new(RwLock::new(CnsRuntime::with_threshold(
            DEFAULT_VARIETY_MAX_DEFICIT as u64,
        )));
        let svc = CnsService::new(runtime);

        let variety = svc.variety().await;
        assert!(
            variety.is_empty(),
            "Fresh CNS runtime should have no variety domains"
        );
    }
}
