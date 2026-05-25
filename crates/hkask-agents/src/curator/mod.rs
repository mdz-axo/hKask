//! Curator Agent — Human counterpart and escalation handler

pub mod escalation;
pub mod metacognition;

pub use escalation::{EscalationEntry, EscalationQueue, EscalationStats, EscalationStatus};
pub use metacognition::{
    BotHealthStatus, BotStatusReport, EscalationThresholds, MetacognitionConfig,
    MetacognitionError, MetacognitionLoop, SystemHealthSnapshot,
};
