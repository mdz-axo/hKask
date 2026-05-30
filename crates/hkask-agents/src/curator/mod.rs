//! Curator Agent — Human counterpart and escalation handler

pub mod escalation;
pub mod metacognition;

pub use escalation::{EscalationEntry, EscalationQueue, EscalationStats, EscalationStatus};
#[allow(deprecated)]
pub use metacognition::SystemHealthSnapshot;
pub use metacognition::{
    BotHealthStatus, BotStatusReport, EscalationThresholds, HealthSnapshot, MetacognitionConfig,
    MetacognitionError, MetacognitionLoop,
};
