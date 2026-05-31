//! hKask CNS — Cybernetic Nervous System
//!
//! Minimal observability: variety counting, algedonic alerts, energy budgets.
//! Per Ashby's Law of Requisite Variety.

pub mod algedonic;
pub mod bot_metrics;
pub mod energy;
pub mod observers;
pub mod runtime;
pub mod unified_tracker;
pub mod variety;

pub use algedonic::{
    AlertSeverity, AlgedonicManager, CnsHealth, DEFAULT_EXPECTED_VARIETY, DEFAULT_THRESHOLD,
    RuntimeAlert,
};
pub use bot_metrics::{BotEvaluationMetrics, BotHealthStatus, CapabilityGap, GapType};
pub use energy::{EnergyAccount, EnergyBudget, EnergyError, OpportunityCost};
pub use observers::sovereignty::{
    SovereigntyEvent, SovereigntyEventType, SovereigntyObserverState,
};
pub use runtime::{AlertSubscription, CnsRuntime};
pub use unified_tracker::UnifiedVarietyTracker;
pub use variety::{VarietyMonitor, VarietyTracker};
