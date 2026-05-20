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
//!
//! **Algedonic Alert:** Variety deficit >100 → escalate to Curator/human

pub mod algedonic;
pub mod energy;
pub mod observers;
pub mod rate_limit;
pub mod spans;
pub mod variety;

pub use algedonic::{
    AlertSeverity, AlgedonicAlert, AlgedonicManager, CnsHealth, DEFAULT_THRESHOLD,
};
pub use energy::{
    EnergyAccount, EnergyBudget, EnergyEmitter, EnergyError, EnergySpanType, OpportunityCost,
    calculate_energy_cost, estimate_tokens,
};
pub use observers::composition::{
    CompositionMetrics, CompositionObserver, CompositionObserverState,
};
pub use rate_limit::{RateLimitConfig, RateLimiter};
pub use spans::{SpanCategory, SpanEmitter};
pub use variety::{VarietyCounter, VarietyMonitor};
