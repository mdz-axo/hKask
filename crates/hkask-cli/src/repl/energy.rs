//! RAII gas governance for the REPL's hold-settle pattern.
//!
//! The REPL uses a two-phase gas accounting pattern:
//! 1. Reserve a heuristic estimate before inference
//! 2. Settle with the actual token cost after
//!
//! EnergyGuard encapsulates this pattern as an owned-consumption guard.
//! `settle()` consumes the guard — the compiler guarantees settle is called
//! exactly once. No Drop fallback, no release-build gas leaks.
//!
//! # REQ: P9-gas-settle — EnergyGuard guarantees settle() is called exactly once
//! expect: "I can access all hKask functionality through the kask CLI"

use hkask_agents::InferenceLoop;
use hkask_cns::{CyberneticsLoop, EnergyCost};
use hkask_types::WebID;
use std::sync::Arc;
use tokio::sync::RwLock;

/// RAII guard for the hold-settle gas pattern.
///
/// Created via `try_reserve()`, which checks `can_proceed` and reserves
/// the heuristic amount. Call `settle(actual_cost)` after inference to
/// reconcile with the actual token cost. `settle` consumes the guard —
/// compiler-enforced: the guard cannot be dropped without settling.
pub(crate) struct EnergyGuard {
    cybernetics_loop: Arc<RwLock<CyberneticsLoop>>,
    inference_loop: Arc<InferenceLoop>,
    webid: WebID,
    rt: tokio::runtime::Handle,
    heuristic: u64,
}

impl EnergyGuard {
    /// Attempt to reserve gas for a pending operation.
    ///
    /// Returns `None` if the energy budget is exhausted (hard limit reached).
    /// On success, the heuristic amount is reserved and the guard is returned.
    pub(crate) fn try_reserve(
        cybernetics_loop: &Arc<RwLock<CyberneticsLoop>>,
        inference_loop: &Arc<InferenceLoop>,
        webid: &WebID,
        rt: &tokio::runtime::Handle,
        heuristic: u64,
    ) -> Option<Self> {
        let can = rt.block_on(async {
            cybernetics_loop
                .read()
                .await
                .can_proceed(webid, EnergyCost(heuristic))
                .await
        });
        if !can {
            return None;
        }
        let _ = rt.block_on(async {
            cybernetics_loop
                .read()
                .await
                .reserve_gas(webid, EnergyCost(heuristic))
                .await
        });
        Some(Self {
            cybernetics_loop: Arc::clone(cybernetics_loop),
            inference_loop: Arc::clone(inference_loop),
            webid: *webid,
            rt: rt.clone(),
            heuristic,
        })
    }

    /// The heuristic cost used for reservation.
    pub(crate) fn heuristic(&self) -> u64 {
        self.heuristic
    }

    /// Settle gas with actual cost and sync InferenceLoop from L6 budget.
    /// Consumes self — compiler-enforced: settle must be called exactly once.
    ///
    /// # REQ: P9-gas-settle — EnergyGuard guarantees settle() is called exactly once
    /// expect: "I can access all hKask functionality through the kask CLI"
    pub(crate) fn settle(self, actual: u64) {
        let _ = self.rt.block_on(async {
            self.cybernetics_loop
                .read()
                .await
                .settle_gas(&self.webid, EnergyCost(self.heuristic), EnergyCost(actual))
                .await
        });
        // Sync InferenceLoop's sense signal from the authoritative L6 budget.
        if let Some(status) = self.rt.block_on(async {
            self.cybernetics_loop
                .read()
                .await
                .agent_gas_status(&self.webid)
                .await
        }) {
            self.inference_loop
                .sync_gas_state(status.remaining.as_raw(), status.cap.as_raw());
        }
        // self dropped here — no Drop impl needed
    }
}
