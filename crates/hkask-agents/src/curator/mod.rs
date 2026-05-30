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
#[allow(deprecated)]
pub use metacognition::SystemHealthSnapshot;
pub use metacognition::{
    BotHealthStatus, BotStatusReport, EscalationThresholds, HealthSnapshot, MetacognitionConfig,
    MetacognitionError, MetacognitionLoop,
};
