//! CNS context — variety sensing, cybernetic regulation, loop orchestration,
//! event audit trail, and energy estimation.
//!
//! Extracted from `AgentService` as part of the strangler-fig decomposition.
//! Consolidates five CNS concerns into a single context.

use hkask_agents::loop_system::LoopSystem;
use hkask_cns::{CalibratedEnergyEstimator, CnsRuntime, CyberneticsLoop};
use hkask_types::event::NuEventSink;
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
}
