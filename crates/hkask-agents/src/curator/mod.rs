//! Curator Agent — Human counterpart and escalation handler

pub mod context;
pub mod dampener;
pub mod dispatch;
pub mod escalation;
pub mod metacognition;

pub use context::CuratorContext;
pub use dampener::Dampener;
pub use dispatch::MessageDispatch;
pub use escalation::{EscalationEntry, EscalationQueue, EscalationStats, EscalationStatus};
pub use metacognition::{
    BotStatusReport, EscalationThresholds, HealthSnapshot, MetacognitionConfig, MetacognitionError,
    MetacognitionLoop,
};
// Re-export CNS BotHealthStatus for consumers who import from curator
pub use hkask_cns::bot_metrics::BotHealthStatus;
