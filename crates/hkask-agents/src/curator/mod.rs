//! Curator Agent — Human counterpart and escalation handler

pub mod escalation;

pub use escalation::{EscalationEntry, EscalationQueue, EscalationStats, EscalationStatus};
