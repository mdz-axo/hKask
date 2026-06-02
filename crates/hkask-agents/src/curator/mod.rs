//! Curator Agent — Metacognitive observer (Loop 5)

pub mod bot_metrics;
pub mod context;
pub mod curation_loop;
pub mod metacognition;

pub use bot_metrics::{BotEvaluationMetrics, BotHealthStatus, CapabilityGap, GapType};
pub use context::CuratorContext;
pub use curation_loop::CurationLoop;
pub use hkask_cns::Dampener;
pub use metacognition::{
    BotStatusReport, EscalationThresholds, HealthSnapshot, MetacognitionConfig, MetacognitionError,
    MetacognitionLoop,
};
