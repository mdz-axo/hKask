//! Curator Agent — Human counterpart and escalation handler

pub mod context;
pub mod curation_loop;
pub mod dampener;
pub mod escalation;
pub mod metacognition;

pub use context::CuratorContext;
pub use curation_loop::CurationLoop;
pub use dampener::Dampener;
pub use escalation::{EscalationEntry, EscalationQueue, EscalationStats, EscalationStatus};
pub use hkask_cns::bot_metrics::BotHealthStatus;
pub use metacognition::{
    BotStatusReport, EscalationThresholds, HealthSnapshot, MetacognitionConfig, MetacognitionError,
    MetacognitionLoop,
};
