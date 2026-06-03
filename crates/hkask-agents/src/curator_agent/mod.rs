//! Curator Agent — Persona layer for the Curator (Loop 5)
//!
//! The Curator Agent is the persona/agent half of the Curation separation.
//! It holds metacognition, bot metrics, spec curation, and human-facing
//! reporting — everything that is NOT pure regulatory loop behavior.
//!
//! The Curation Loop (`curator::CurationLoop`) is the pure regulatory half:
//! sense/compute/act with no persona, no chat, no memory. The Curator Agent
//! *uses* the Curation Loop through Communication dispatch and receives
//! `CuratorDirective`s that it formats for human consumption.
//!
//! # Architecture
//!
//! ```text
//! CuratorAgent
//! ├── curation_loop: Arc<CurationLoop>   // pure regulatory
//! ├── metacognition: Arc<MetacognitionLoop> // persona: observe & adapt
//! └── context: Arc<CuratorContext>       // capability-disciplined access
//! ```

pub mod bot_metrics;
pub mod metacognition;
pub mod spec_curator;

use crate::curator::context::CuratorContext;
use crate::curator::curation_loop::CurationLoop;
use std::sync::Arc;

/// Curator Agent — the persona layer of Curation (Loop 5).
///
/// Composes the pure regulatory `CurationLoop` with the persona/agent
/// `MetacognitionLoop`. The agent receives `CuratorDirective`s from the
/// Curation Loop through Communication dispatch and formats human-readable
/// output for `kask chat`.
///
/// **Construction:** Use `CuratorAgent::new()` with a `CuratorContext`,
/// `MetacognitionConfig`, and optional consolidation port. The agent
/// internally creates both the `MetacognitionLoop` and `CurationLoop`.
///
/// **Singleton invariant:** There is exactly one `CuratorAgent` per hKask
/// system, just as there is exactly one `CurationLoop`.
pub struct CuratorAgent {
    curation_loop: Arc<CurationLoop>,
    metacognition: Arc<metacognition::MetacognitionLoop>,
    context: Arc<CuratorContext>,
}

impl CuratorAgent {
    /// Create a new Curator Agent with default configuration.
    ///
    /// The agent internally creates both the `MetacognitionLoop` and
    /// `CurationLoop`, connecting them through the shared `CuratorContext`.
    pub fn new(context: Arc<CuratorContext>) -> Self {
        let metacognition = Arc::new(metacognition::MetacognitionLoop::new(
            Arc::clone(&context),
            metacognition::MetacognitionConfig::default(),
        ));
        let curator_handle = context.handle().clone();
        let curation_loop = Arc::new(CurationLoop::new(curator_handle, Arc::clone(&context)));

        Self {
            curation_loop,
            metacognition,
            context,
        }
    }

    /// Create a Curator Agent with custom metacognition configuration.
    pub fn with_config(
        context: Arc<CuratorContext>,
        config: metacognition::MetacognitionConfig,
    ) -> Self {
        let metacognition = Arc::new(metacognition::MetacognitionLoop::new(
            Arc::clone(&context),
            config,
        ));
        let curator_handle = context.handle().clone();
        let curation_loop = Arc::new(CurationLoop::new(curator_handle, Arc::clone(&context)));

        Self {
            curation_loop,
            metacognition,
            context,
        }
    }

    /// Create a Curator Agent with a consolidation port.
    ///
    /// When episodic budget pressure triggers escalation, the consolidation
    /// bridge will fire to migrate episodic triples into semantic memory.
    pub fn with_consolidation(
        context: Arc<CuratorContext>,
        config: metacognition::MetacognitionConfig,
        consolidation: Arc<dyn hkask_types::ports::ConsolidationPort>,
    ) -> Self {
        let metacognition = Arc::new(metacognition::MetacognitionLoop::new(
            Arc::clone(&context),
            config,
        ));
        let curator_handle = context.handle().clone();
        let curation_loop = Arc::new(CurationLoop::with_consolidation(
            curator_handle,
            Arc::clone(&context),
            consolidation,
        ));

        Self {
            curation_loop,
            metacognition,
            context,
        }
    }

    /// Access the Curation Loop (pure regulatory).
    pub fn curation_loop(&self) -> &Arc<CurationLoop> {
        &self.curation_loop
    }

    /// Access the Metacognition Loop (persona/agent).
    pub fn metacognition(&self) -> &Arc<metacognition::MetacognitionLoop> {
        &self.metacognition
    }

    /// Access the CuratorContext (capability-disciplined runtime references).
    pub fn context(&self) -> &Arc<CuratorContext> {
        &self.context
    }
}

// Re-export persona types for convenience
pub use metacognition::{
    HealthSnapshot, MetacognitionConfig, MetacognitionError, MetacognitionLoop,
};
pub use spec_curator::DefaultSpecCurator;
