//! hKask CNS — Cybernetic Nervous System
//!
//! Homeostatic self-regulation: variety sensing, algedonic alerts, energy budgets,
//! OCAP governance, sovereignty enforcement. Per Ashby's Law of Requisite Variety.

pub mod algedonic; // Loop 6 subloop 6.4 — algedonic signal channel
pub mod allosteric; // ARL — Allosteric Regulation Logic (MWC gates)
pub mod circuit_breaker; // Loop 6 — regulation
pub mod composite_gas_estimator; // Composite routing: inference → token-based, others → table
pub mod cybernetics_loop; // Loop 6
pub mod dampener; // Loop 6 — regulation
pub mod energy; // Loop 6 — gas budget (replaces energy budget)
pub mod governed_tool; // Loop 6 → all tool invocation membranes
pub mod inference_estimator; // Loop 6 → Inference gas estimation
pub mod kill_zone; // Loop 6 subloop 6.5 — kill-zone detection
pub mod runtime; // Loop 6 — runtime
pub mod table_gas_estimator; // Per-server gas cost table

pub mod unified_tracker; // Loop 6 — variety tracking
pub mod variety; // Loop 6 subloop 6.3

pub use algedonic::{AlgedonicManager, DEFAULT_THRESHOLD, RuntimeAlert, cns_health_check};
pub use allosteric::{AllostericGate, AllostericGateConfig, mwc_sensitivity, mwc_state_function};
// CircuitBreaker: re-exported for runtime wiring
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
pub use composite_gas_estimator::CompositeGasEstimator;
pub use cybernetics_loop::{CyberneticsLoop, SetPoints, SetPointsConfig};
pub use dampener::Dampener;
pub use energy::{GasBudget, GasError};
pub use governed_tool::{GasEstimator, GovernedTool};
pub use inference_estimator::InferenceGasEstimator;
pub use table_gas_estimator::TableGasEstimator;

pub use runtime::CnsRuntime;

// Re-export types moved to hkask-types for backward compatibility
pub use hkask_types::cns::{CircuitState, CnsHealth};
pub use hkask_types::ports::{CircuitBreakerPort, CnsPort};
// KillZoneDetector: pub(crate) — only consumed via CnsRuntime methods
