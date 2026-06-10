//! hKask CNS — Cybernetic Nervous System
//!
//! Homeostatic self-regulation: variety sensing, algedonic alerts, energy budgets,
//! OCAP governance, sovereignty enforcement. Per Ashby's Law of Requisite Variety.

pub(crate) mod algedonic; // Loop 6 subloop 6.4 — algedonic signal channel
pub mod circuit_breaker; // Loop 6 — regulation
pub mod composite_energy_estimator; // Composite routing: inference → token-based, others → table
pub mod cybernetics_loop; // Loop 6
pub(crate) mod dampener; // Loop 6 — regulation
pub mod energy; // Loop 6 — energy budgets (hJoules)
pub mod energy_budget_management; // Loop 6 — energy budget registration/reservation/settlement
pub mod governed_tool; // Loop 6 → all tool invocation membranes
pub(crate) mod inference_estimator; // Loop 6 → Inference energy estimation

pub mod runtime; // Loop 6 — runtime
pub mod set_points; // Loop 6 — set-points config & loaders
pub(crate) mod snapshot_loop; // Loop 6 — scheduled CAS snapshots
pub(crate) mod table_energy_estimator; // Per-server energy cost table
// variety module merged into runtime.rs (TASK 2 deletion test):
// VarietyMonitor and VarietyTracker are now co-located with their
// sole consumer (CnsRuntime), increasing module depth.
pub mod variety {
    //! Thin re-export for backward compatibility.
    //! All types live in `crate::runtime` since v0.27.2.
    pub use crate::runtime::VarietyMonitor;
}

pub use algedonic::{DEFAULT_THRESHOLD, RuntimeAlert};
pub use circuit_breaker::CircuitBreaker;
pub use composite_energy_estimator::CompositeEnergyEstimator;
pub use cybernetics_loop::CyberneticsLoop;
pub use energy::{
    AgentEnergyStatus, DEFAULT_ENERGY_ALERT_THRESHOLD, EnergyBudget, EnergyCost, EnergyError,
};
pub use energy_budget_management::EnergyBudgetManager;
pub use governed_tool::{EnergyEstimator, GovernedTool};
// allosteric types deleted — MWC sigmoid added zero runtime-observable behavior.
pub use hkask_types::cns::{QueueDepth, RBarThreshold};
pub use runtime::CnsRuntime;
pub use set_points::{
    CurationThresholdConfig, DEFAULT_COMMUNICATION_BACKPRESSURE_THRESHOLD,
    DEFAULT_CONNECTOR_LATENCY_MAX_SECS, DEFAULT_ENERGY_MIN_REMAINING_RATIO, DEFAULT_ERROR_RATE_MAX,
    DEFAULT_MAX_ITERATIONS, DEFAULT_VARIETY_MAX_DEFICIT, SetPoints, SetPointsConfig,
    load_curation_thresholds, load_set_points,
};
pub use snapshot_loop::{SnapshotLoop, SnapshotLoopConfig};

// Re-export types moved to hkask-types for backward compatibility
pub use hkask_types::cns::{CircuitState, CnsHealth};
pub use hkask_types::ports::CircuitBreakerPort;
