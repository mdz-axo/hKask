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
pub mod spans;
pub mod variety;

pub use algedonic::{AlgedonicAlert, AlgedonicManager, AlertSeverity, CnsHealth, DEFAULT_THRESHOLD};
pub use spans::{SpanCategory, SpanEmitter};
pub use variety::{VarietyCounter, VarietyMonitor};
