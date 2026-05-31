//! hKask CNS — Cybernetic Nervous System
//!
//! Minimal observability: variety counting, algedonic alerts, energy budgets.
//! Per Ashby's Law of Requisite Variety.

pub mod acp_alert_sender;
pub mod algedonic;
pub mod algedonic_escalation;
pub mod bot_metrics;
pub mod energy;
pub mod observers;
pub mod runtime;
pub mod unified_tracker;
pub mod variety;

pub use acp_alert_sender::AcpAlertSender;
pub use algedonic::{
    AlertSeverity, AlgedonicManager, CnsHealth, DEFAULT_EXPECTED_VARIETY, DEFAULT_THRESHOLD,
    RuntimeAlert,
};
pub use algedonic_escalation::{
    AcpSender, AlgedonicEscalationAdapter, CalibrationRecord, EscalationAction, EscalationResult,
    compute_spec_drift, create_escalation_callback,
};
pub use bot_metrics::{BotEvaluationMetrics, BotHealthStatus, CapabilityGap, GapType};
pub use energy::{EnergyAccount, EnergyBudget, EnergyError, OpportunityCost};
pub use observers::sovereignty::{
    SovereigntyEvent, SovereigntyEventType, SovereigntyObserverState,
};
pub use runtime::{AlertSubscription, CnsRuntime};
pub use unified_tracker::UnifiedVarietyTracker;
pub use variety::{VarietyMonitor, VarietyTracker};
