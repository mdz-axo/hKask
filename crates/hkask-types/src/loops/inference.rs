//! Loop 1: Inference — Capability handle
//!
//! The Inference loop governs the path from prompt to response:
//! prompt → context → model → response → parse → act
//!
//! Essential subloop:
//! - 1.1 Context Assembly (FILTER) — filter and assemble context for inference
//!
//! Governance (via InferenceRegulation from Cybernetics):
//! - Energy throttling — Cybernetics owns the energy budget
//! - Circuit breaking — Cybernetics governs circuit state
//! - Energy cap adjustment — Curation can adjust budgets through Cybernetics

use crate::id::WebID;

/// Inference loop capability handle.
///
/// Narrow identity handle for the Inference loop. The Inference loop does NOT
/// carry energy state or circuit state — those are owned by the Cybernetics
/// loop and accessed via `InferenceRegulation`.
///
/// # OCAP Boundaries
///
/// - **CAN** identify the agent for inference
/// - **CAN** request energy from Cybernetics via `InferenceRegulation`
/// - **CANNOT** carry energy state (owned by Cybernetics)
/// - **CANNOT** carry circuit state (owned by Cybernetics)
/// - **CANNOT** store triples (use `EpisodicWriteHandle` / `SemanticWriteHandle`)
pub struct InferenceHandle {
    /// Agent performing inference
    agent_webid: WebID,
}

impl InferenceHandle {
    /// Create a test handle with a synthetic WebID.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            agent_webid: WebID::new(),
        }
    }

    /// Create an inference handle for a specific agent.
    ///
    /// # Requires
    /// - `agent_webid` must be a valid agent identifier
    ///
    /// # Ensures
    /// - Handle is bound to the given agent
    pub fn new(agent_webid: WebID) -> Self {
        Self { agent_webid }
    }

    /// The agent performing inference.
    pub fn agent(&self) -> &WebID {
        &self.agent_webid
    }
}

/// Error returned when an energy budget is exceeded.
///
/// This error is produced by the Cybernetics loop's energy budget
/// subsystem when an agent attempts to consume more energy than
/// its allocated budget allows.
#[derive(Debug, Clone, thiserror::Error)]
#[error("energy budget exceeded: requested {requested}, remaining {remaining}")]
pub struct InferenceBudgetExceeded {
    pub requested: u64,
    pub remaining: u64,
}

/// Regulation interface for the Inference Loop.
///
/// The Cybernetics Loop uses this to throttle energy allocation
/// or circuit-break inference when error rates exceed thresholds.
/// The Curation Loop uses this to adjust energy budgets.
///
/// # Authority
///
/// Only the Cybernetics Loop holds an `InferenceRegulation` reference.
/// This ensures energy and circuit state flow downward from Cybernetics
/// to Inference, never sideways.
pub trait InferenceRegulation: Send + Sync {
    /// Throttle the inference loop's energy allocation.
    fn throttle_energy(&self, reason: &str, remaining_ratio: f64);

    /// Open or close the inference circuit breaker.
    fn set_circuit_state(&self, open: bool, reason: &str);

    /// Adjust the energy budget cap.
    fn adjust_energy_cap(&self, new_cap: u64);
}
