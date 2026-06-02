//! hKask CNS — Cybernetic Nervous System
//!
//! Homeostatic self-regulation: variety sensing, algedonic alerts, energy budgets,
//! OCAP governance, sovereignty enforcement. Per Ashby's Law of Requisite Variety.

pub mod algedonic;
pub mod bot_metrics;
pub mod cybernetics_loop;
pub mod energy;
pub mod observers;
pub mod runtime;
pub mod unified_tracker;
pub mod variety;

pub use algedonic::{AlgedonicManager, CnsHealth, DEFAULT_THRESHOLD, RuntimeAlert};
pub use bot_metrics::{BotEvaluationMetrics, BotHealthStatus, CapabilityGap, GapType};
pub use cybernetics_loop::{CyberneticsLoop, SetPoints};
pub use energy::{EnergyBudget, EnergyError};
pub use observers::sovereignty::SovereigntyObserverState;
pub use runtime::CnsRuntime;
