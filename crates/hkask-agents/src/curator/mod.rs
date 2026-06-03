//! Curator Agent — Metacognitive observer (Loop 5)

pub mod bot_metrics;
pub mod context;
pub mod curation_gate;
pub mod curation_loop;
pub mod metacognition;
pub mod spec_curator;

pub use context::CuratorContext;
pub use curation_gate::{CurationConfidenceGate, CurationDecision, CurationPort};
pub use curation_loop::CurationLoop;

pub use metacognition::HealthSnapshot;
pub use metacognition::MetacognitionError;

pub use metacognition::{MetacognitionConfig, MetacognitionLoop};
