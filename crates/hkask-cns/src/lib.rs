//! hKask CNS — Cybernetic Nervous System
//!
//! Homeostatic self-regulation: variety sensing, algedonic alerts, energy budgets,
//! OCAP governance, sovereignty enforcement. Per Ashby's Law of Requisite Variety.

pub mod algedonic;
pub mod circuit_breaker;
pub mod cybernetics_loop;
pub mod dampener;
pub mod energy;
pub mod inference_loop;
pub mod observers;
pub mod runtime;
pub mod unified_tracker;
pub mod variety;

pub use algedonic::{AlgedonicManager, DEFAULT_THRESHOLD, RuntimeAlert, cns_health_check};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
pub use cybernetics_loop::{CyberneticsLoop, SetPoints};
pub use dampener::Dampener;
pub use energy::{EnergyBudget, EnergyError};
pub use inference_loop::InferenceLoop;

pub use runtime::CnsRuntime;

// Re-export types moved to hkask-types for backward compatibility
pub use hkask_types::cns::{CircuitState, CnsHealth};
pub use hkask_types::ports::{CircuitBreakerPort, CnsPort};
