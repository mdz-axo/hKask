//! hKask CNS — Cybernetic Nervous System
//!
//! Homeostatic self-regulation: variety sensing, algedonic alerts, energy budgets,
//! OCAP governance, sovereignty enforcement. Per Ashby's Law of Requisite Variety.

pub(crate) mod algedonic; // Loop 6 subloop 6.4 — algedonic signal channel
pub mod allosteric; // ARL — Allosteric Regulation Logic (MWC gates)
pub mod circuit_breaker; // Loop 6 — regulation
pub mod composite_gas_estimator; // Composite routing: inference → token-based, others → table
pub mod cybernetics_loop; // Loop 6
pub(crate) mod dampener; // Loop 6 — regulation
pub mod energy; // Loop 6 — gas budget (replaces energy budget)
pub mod gas_budget_management; // Loop 6 — gas budget registration/reservation/settlement
pub mod governed_tool; // Loop 6 → all tool invocation membranes
pub(crate) mod inference_estimator; // Loop 6 → Inference gas estimation

pub mod runtime; // Loop 6 — runtime
pub mod set_points; // Loop 6 — set-points config & loaders
pub(crate) mod snapshot_loop; // Loop 6 — scheduled CAS snapshots
pub(crate) mod table_gas_estimator; // Per-server gas cost table

pub use algedonic::{DEFAULT_THRESHOLD, RuntimeAlert};
pub use allosteric::{AllostericError, AllostericGate, AllostericGateConfig, mwc_state_function};
pub use circuit_breaker::CircuitBreaker;
pub use composite_gas_estimator::CompositeGasEstimator;
pub use cybernetics_loop::CyberneticsLoop;
pub use energy::{
    AgentGasStatus, DEFAULT_GAS_ALERT_THRESHOLD, GasBudget, GasCost, GasError, QueueDepth,
    RBarThreshold,
};
pub use gas_budget_management::GasBudgetManager;
pub use governed_tool::{GasEstimator, GovernedTool};
pub use runtime::CnsRuntime;
pub use set_points::{
    CurationThresholdConfig, DEFAULT_COMMUNICATION_BACKPRESSURE_THRESHOLD,
    DEFAULT_CONNECTOR_LATENCY_MAX_SECS, DEFAULT_ERROR_RATE_MAX, DEFAULT_GAS_MIN_REMAINING_RATIO,
    DEFAULT_MAX_ITERATIONS, DEFAULT_VARIETY_MAX_DEFICIT, SetPoints, SetPointsConfig,
    load_curation_thresholds, load_set_points,
};
pub use snapshot_loop::{SnapshotLoop, SnapshotLoopConfig};

// Re-export types moved to hkask-types for backward compatibility
pub use hkask_types::cns::{CircuitState, CnsHealth};
pub use hkask_types::ports::CircuitBreakerPort;
