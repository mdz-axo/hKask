//! hKask CNS — Cybernetic Nervous System
//!
//! Implements cybernetic monitoring per Ashby's Law of Requisite Variety.
//! Emits ν-events (NuEvent) for audit trail and algedonic alerts for escalation.
//!
//! **Span Categories:**
//! - `cns.connector.*` — External I/O (LLM dispatch, OCR, embeddings)
//! - `cns.pipeline.*` — Multi-stage processing flows
//! - `cns.tool.*` — Tool governance and invocation
//! - `cns.prompt.*` — Prompt feedback loop (render, validate, outcome)
//! - `cns.agent_pod.*` — Agent lifecycle (populate, register, activate, delegate)
//! - `cns.goal.*` — Goal primitive (create, transition, verify, complete, subgoal)
//! - `cns.review.*` — Review queue (submitted, reviewed, approved, rejected)
//! - `cns.spec.*` — Spec primitive (spec validation, compliance, verification)
//!
//! **Algedonic Alert:** Variety deficit >100 → escalate to Curator/human

pub mod acp_alert_sender;
pub mod algedonic;
pub mod algedonic_escalation;
pub mod bot_metrics;
pub mod energy;
pub mod goal_variety;
pub mod observers;
pub mod rate_limit;
pub mod runtime;
pub mod spans;
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
pub use bot_metrics::{
    BotEvaluationMetrics, BotHealthStatus, BotMetricsCollector, CapabilityGap, GapType,
};
pub use energy::{EnergyAccount, EnergyBudget, EnergyError, OpportunityCost};
pub use goal_variety::{GoalVarietyCounter, GoalVarietyMonitor};
pub use hkask_types::SpanCategory;
pub use observers::sovereignty::{
    SovereigntyEvent, SovereigntyEventType, SovereigntyObserver, SovereigntyObserverState,
};
pub use rate_limit::{RateLimitConfig, RateLimiter};
pub use runtime::{
    AlertSubscription, CnsAdminHandle, CnsGovernReadHandle, CnsGovernWriteHandle, CnsRuntime,
    CnsWriteHandle,
};
pub use spans::{
    CnsEmit, SpanEmitter, SpanScope, SpanViolation, curator_span_scope, span_scope_for_domain,
    span_scope_for_r7_bot,
};
pub use unified_tracker::UnifiedVarietyTracker;
pub use variety::{VarietyMonitor, VarietyTracker};
