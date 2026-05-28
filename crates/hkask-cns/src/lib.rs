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

pub mod algedonic;
pub mod algedonic_escalation;
pub mod bot_metrics;
pub mod energy;
pub mod goal_variety;
pub mod observers;
pub mod rate_limit;
pub mod runtime;
pub mod spans;
pub mod variety;

pub use algedonic::{AlertSeverity, AlgedonicManager, CnsHealth, DEFAULT_THRESHOLD, RuntimeAlert};
pub use algedonic_escalation::{
    AlgedonicEscalationAdapter, CalibrationRecord, EscalationAction, EscalationResult,
    create_escalation_callback,
};
pub use bot_metrics::{
    BotEvaluationMetrics, BotHealthStatus, BotMetricsCollector, CapabilityGap, GapType,
};
pub use energy::{EnergyAccount, EnergyBudget, EnergyError, OpportunityCost};
pub use goal_variety::{GoalVarietyCounter, GoalVarietyMonitor};
pub use observers::composition::{
    CompositionMetrics, CompositionObserver, CompositionObserverState,
};
pub use observers::sovereignty::{
    SovereigntyEvent, SovereigntyEventType, SovereigntyObserver, SovereigntyObserverState,
};
pub use rate_limit::{RateLimitConfig, RateLimiter};
pub use runtime::CnsRuntime;
pub use spans::{CnsEmit, SpanCategory, SpanEmitter, SpanScope, SpanViolation, span_scope_for_bot};
pub use variety::{VarietyMonitor, VarietyTracker};
