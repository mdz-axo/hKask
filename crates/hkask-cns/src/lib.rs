//! hKask CNS — Cybernetic Nervous System
//!
//! Homeostatic self-regulation: variety sensing, algedonic alerts, energy budgets,
//! OCAP governance, sovereignty enforcement. Per Ashby's Law of Requisite Variety.

pub mod algedonic;
pub mod circuit_breaker;
pub mod cybernetics_loop;
pub mod dampener;
pub mod energy;
pub mod observers;
pub mod runtime;
pub mod unified_tracker;
pub mod variety;

pub use algedonic::{AlgedonicManager, CnsHealth, DEFAULT_THRESHOLD, RuntimeAlert};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use cybernetics_loop::{CyberneticsLoop, SetPoints};
pub use dampener::{DEFAULT_DAMPEN_WINDOW, Dampener};
pub use energy::{EnergyBudget, EnergyError};
pub use observers::sovereignty::SovereigntyObserverState;
pub use runtime::CnsRuntime;
