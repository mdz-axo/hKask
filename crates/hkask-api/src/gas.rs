//! API-mode gas governance adapter for ensemble sessions.
//!
//! Bridges `CyberneticsLoop` to the ensemble's `GasGovernancePort` using
//! atomic counters for approximate `can_proceed` and fire-and-forget async
//! for `acquire` (actual budget consumption via the CyberneticsLoop).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use hkask_cns::{CyberneticsLoop, GasCost};
use hkask_types::WebID;

/// Default gas cap for API ensemble sessions (150k = same as CLI default).
pub(crate) const API_ENSEMBLE_GAS_CAP: u64 = 150_000;

/// Adapter bridging `CyberneticsLoop` to the ensemble's `GasGovernancePort`.
///
/// Provides synchronous access to the CyberneticsLoop's gas governance by
/// using an atomic counter for `can_proceed` (approximate) and a fire-and-forget
/// task spawn for `acquire` (actual budget consumption via async call).
///
/// This is the API-mode equivalent of the CLI's `CyberneticsLoopGasAdapter`.
pub(crate) struct ApiGasGovernanceAdapter {
    loop_ref: Arc<tokio::sync::RwLock<CyberneticsLoop>>,
    agent: WebID,
    gas_used: AtomicU64,
    gas_cap: AtomicU64,
}

impl ApiGasGovernanceAdapter {
    pub(crate) fn new(
        loop_ref: Arc<tokio::sync::RwLock<CyberneticsLoop>>,
        agent: WebID,
        cap: u64,
    ) -> Self {
        Self {
            loop_ref,
            agent,
            gas_used: AtomicU64::new(0),
            gas_cap: AtomicU64::new(cap),
        }
    }
}

impl hkask_agents::ensemble::GasGovernancePort for ApiGasGovernanceAdapter {
    fn can_proceed(&self, gas: u64) -> bool {
        let used = self.gas_used.load(Ordering::Relaxed);
        let cap = self.gas_cap.load(Ordering::Relaxed);
        used.saturating_add(gas) <= cap
    }

    fn acquire(&self, gas: u64) {
        self.gas_used.fetch_add(gas, Ordering::Relaxed);
        // Fire-and-forget: report to CyberneticsLoop asynchronously
        let loop_ref = self.loop_ref.clone();
        let agent = self.agent;
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let loop_read = loop_ref.read().await;
                let _ = loop_read.acquire_budget(&agent, GasCost(gas)).await;
            });
        }
    }
}
