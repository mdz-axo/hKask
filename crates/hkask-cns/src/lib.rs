//! hKask CNS — Cybernetic Nervous System
//!
//! Homeostatic self-regulation: variety sensing, algedonic alerts, energy budgets,
//! OCAP governance, sovereignty enforcement. Per Ashby's Law of Requisite Variety.

pub mod algedonic; // Loop 6 subloop 6.4 — algedonic signal channel
pub mod allosteric; // ARL — Allosteric Regulation Logic (MWC gates)
pub mod circuit_breaker; // Loop 6 — regulation
pub mod cybernetics_loop; // Loop 6
pub mod dampener; // Loop 6 — regulation
pub mod energy; // Loop 6 — thermodynamic resource allocation
pub mod governed_inference; // Loop 6 → Loop 1 membrane
pub mod kill_zone; // Loop 6 subloop 6.5 — kill-zone detection
pub mod runtime; // Loop 6 — runtime
pub mod unified_tracker; // Loop 6 — variety tracking
pub mod variety; // Loop 6 subloop 6.3

pub use algedonic::{AlgedonicManager, DEFAULT_THRESHOLD, RuntimeAlert, cns_health_check};
pub use allosteric::{
    AllostericError, AllostericGate, AllostericGateConfig, BernoulliDistribution, mwc_sensitivity,
    mwc_state_function,
};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
pub use cybernetics_loop::{CyberneticsLoop, SetPoints};
pub use dampener::Dampener;
pub use energy::{EnergyBudget, EnergyError};
pub use governed_inference::GovernedInference;

pub use runtime::CnsRuntime;

// Re-export types moved to hkask-types for backward compatibility
pub use hkask_types::cns::{CircuitState, CnsHealth};
pub use hkask_types::ports::{CircuitBreakerPort, CnsPort};
pub use kill_zone::KillZoneDetector;
