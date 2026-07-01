//! CNS context — variety sensing, cybernetic regulation, loop orchestration,
//! event audit trail, and energy estimation.
//!
//! Extracted from `AgentService` as part of the strangler-fig decomposition.

use hkask_agents::loop_system::LoopSystem;
use hkask_cns::{CalibratedEnergyEstimator, CnsRuntime, CyberneticsLoop};
use hkask_types::cns::CnsHealth;
use hkask_types::event::{NuEventSink, SpanNamespace};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Consolidated CNS context — variety sensing, cybernetic regulation,
/// loop orchestration, event audit trail, and energy estimation.
pub struct CnsContext {
    pub runtime: Arc<RwLock<CnsRuntime>>,
    pub cybernetics: Arc<RwLock<CyberneticsLoop>>,
    pub loops: Arc<LoopSystem>,
    pub events: Arc<dyn NuEventSink>,
    pub energy: Arc<CalibratedEnergyEstimator>,
}

impl CnsContext {
    pub fn new(
        runtime: Arc<RwLock<CnsRuntime>>,
        cybernetics: Arc<RwLock<CyberneticsLoop>>,
        loops: Arc<LoopSystem>,
        events: Arc<dyn NuEventSink>,
        energy: Arc<CalibratedEnergyEstimator>,
    ) -> Self {
        Self {
            runtime,
            cybernetics,
            loops,
            events,
            energy,
        }
    }

    /// Read the current CNS health snapshot.
    ///
    /// Acquires a read lock on the runtime and returns the health status.
    /// This is the canonical access path — replaces the pattern
    /// `cns().runtime.read().await.health().await`.
    pub async fn health(&self) -> CnsHealth {
        self.runtime.read().await.health().await
    }

    /// Read current variety counters across all monitored domains.
    ///
    /// Acquires a read lock on the runtime and returns per-namespace
    /// variety data. The returned map keys are namespace strings.
    pub async fn variety(&self) -> HashMap<SpanNamespace, u64> {
        self.runtime.read().await.variety().await
    }
}
