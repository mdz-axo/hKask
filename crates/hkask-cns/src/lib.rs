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
pub mod governed_tool; // Loop 6 → all tool invocation membranes
pub(crate) mod inference_estimator; // Loop 6 → Inference gas estimation
pub(crate) mod kill_zone; // Loop 6 subloop 6.5 — kill-zone detection
pub mod prompt_decomposition; // CNS variety sensing — prompt analysis for REPL
pub mod runtime; // Loop 6 — runtime
pub(crate) mod table_gas_estimator; // Per-server gas cost table

pub(crate) mod unified_tracker; // Loop 6 — variety tracking
pub(crate) mod variety; // Loop 6 subloop 6.3

pub use algedonic::{DEFAULT_THRESHOLD, RuntimeAlert};
pub use allosteric::{AllostericError, AllostericGate, AllostericGateConfig, mwc_state_function};
pub use circuit_breaker::CircuitBreaker;
pub use composite_gas_estimator::CompositeGasEstimator;
pub use cybernetics_loop::{CyberneticsLoop, SetPoints, SetPointsConfig, load_set_points};
pub use energy::{AgentGasStatus, GasBudget, GasError};
pub use governed_tool::{GasEstimator, GovernedTool};
pub use runtime::CnsRuntime;

// Re-export types moved to hkask-types for backward compatibility
pub use hkask_types::cns::{CircuitState, CnsHealth};
pub use hkask_types::ports::CircuitBreakerPort;
// KillZoneDetector: pub(crate) — only consumed via CnsRuntime methods
