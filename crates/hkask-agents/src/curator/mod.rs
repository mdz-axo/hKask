//! Curator Agent — Metacognitive observer (Loop 5)

pub mod bot_metrics;
pub mod context;
pub mod curation_loop;
pub mod metacognition;

pub use context::CuratorContext;
pub use curation_loop::CurationLoop;

pub use metacognition::HealthSnapshot;
pub use metacognition::MetacognitionError;

pub use metacognition::{MetacognitionConfig, MetacognitionLoop};
