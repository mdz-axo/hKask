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

pub mod bot_health;
pub mod bot_metrics;
pub mod metacognition;
pub mod spec_curator;

use hkask_rsolidity as rs;
use crate::curator::context::CuratorContext;
use crate::curator::curation_loop::CurationLoop;
use hkask_memory::ConsolidationBridge;
use hkask_types::loops::CurationInput;
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
    /// Spec curator — evaluates spec coherence and drift, sends
    /// SpecDriftAlert through the Communication Loop when drift
    /// exceeds threshold.
    spec_curator: spec_curator::DefaultSpecCurator,
}

impl CuratorAgent {
    /// Create a new Curator Agent with default configuration.
    ///
    /// The agent internally creates both the `MetacognitionLoop` and
    /// `CurationLoop`, connecting them through the shared `CuratorContext`.
    ///
    /// REQ: P9-agt-curator-agent-new
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — CuratorAgent composes Curation + Metacognition
    /// pre:  `context` is a valid `Arc<CuratorContext>`.
    /// post: Returns a `CuratorAgent` with default `MetacognitionConfig`,
    ///       a new `CurationLoop`, and a default `DefaultSpecCurator`.
    #[rs::contract(id = "P9-agt-curator-agent-new", principle = "P9")]
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
            spec_curator: spec_curator::DefaultSpecCurator::default(),
        }
    }

    /// Create a Curator Agent with custom metacognition configuration.
    ///
    /// REQ: P9-agt-curator-agent-new-with-config
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — custom metacognition configuration
    /// \[P7\] Constraining: Evolutionary Architecture — thresholds emerge from real usage
    /// pre:  `context` is a valid `Arc<CuratorContext>`; `config` is a
    ///       valid `MetacognitionConfig`.
    /// post: Returns a `CuratorAgent` with the given config, a new
    ///       `CurationLoop`, and a default `DefaultSpecCurator`.
    #[rs::contract(id = "P9-agt-curator-agent-new-with-config", principle = "P9")]
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
            spec_curator: spec_curator::DefaultSpecCurator::default(),
        }
    }

    /// Create a Curator Agent with a consolidation port.
    ///
    /// When episodic budget pressure triggers escalation, the consolidation
    /// bridge will fire to migrate episodic triples into semantic memory.
    ///
    /// `inbox_rx` — unified CurationInput channel from Cybernetics + SpecCurator.
    /// `inbox_tx` — transmits CurationInput to the same channel (for SpecCurator).
    ///
    /// REQ: P9-agt-curator-agent-new-with-consolidation
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — consolidation wired into CuratorAgent
    /// pre:  `context` is a valid `Arc<CuratorContext>`; `config` is a
    ///       valid `MetacognitionConfig`; `consolidation` is a valid
    ///       `Arc<ConsolidationBridge>`; `inbox_rx` and `inbox_tx` are
    ///       `Some` or `None`.
    /// post: Returns a `CuratorAgent` with consolidation wired; if
    ///       `inbox_rx` is `Some`, the curation loop's inbox is set;
    ///       if `inbox_tx` is `Some`, the spec curator's channel is set.
    #[rs::contract(id = "P9-agt-curator-agent-new-with-consolidation", principle = "P9")]
    pub fn with_consolidation(
        context: Arc<CuratorContext>,
        config: metacognition::MetacognitionConfig,
        consolidation: Arc<ConsolidationBridge>,
        inbox_rx: Option<tokio::sync::mpsc::UnboundedReceiver<CurationInput>>,
        inbox_tx: Option<tokio::sync::mpsc::UnboundedSender<CurationInput>>,
    ) -> Self {
        let metacognition = Arc::new(metacognition::MetacognitionLoop::new(
            Arc::clone(&context),
            config,
        ));
        let curator_handle = context.handle().clone();
        let mut curation_loop =
            CurationLoop::with_consolidation(curator_handle, Arc::clone(&context), consolidation);
        if let Some(rx) = inbox_rx {
            curation_loop = curation_loop.with_inbox(rx);
        }
        let curation_loop = Arc::new(curation_loop);
        let mut spec_curator = spec_curator::DefaultSpecCurator::default();
        if let Some(tx) = inbox_tx {
            spec_curator = spec_curator.with_spec_channel(tx);
        }

        Self {
            curation_loop,
            metacognition,
            context,
            spec_curator,
        }
    }

    /// Access the Curation Loop (pure regulatory).
    ///
    /// REQ: P9-agt-curator-agent-curation-loop
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — accessor for the pure regulatory loop
    /// pre:  (none — accessor).
    /// post: Returns a reference to the inner `Arc<CurationLoop>`.
    #[rs::contract(id = "P9-agt-curator-agent-curation-loop", principle = "P9")]
    pub fn curation_loop(&self) -> &Arc<CurationLoop> {
        &self.curation_loop
    }

    /// Access the Metacognition Loop (persona/agent).
    ///
    /// REQ: P9-agt-curator-agent-metacognition-loop
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — accessor for the persona/agent loop
    /// pre:  (none — accessor).
    /// post: Returns a reference to the inner `Arc<MetacognitionLoop>`.
    #[rs::contract(id = "P9-agt-curator-agent-metacognition-loop", principle = "P9")]
    pub fn metacognition(&self) -> &Arc<metacognition::MetacognitionLoop> {
        &self.metacognition
    }

    /// Access the CuratorContext (capability-disciplined runtime references).
    ///
    /// REQ: P9-agt-curator-agent-context
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — accessor for capability-disciplined context
    /// pre:  (none — accessor).
    /// post: Returns a reference to the inner `Arc<CuratorContext>`.
    #[rs::contract(id = "P9-agt-curator-agent-context", principle = "P9")]
    pub fn context(&self) -> &Arc<CuratorContext> {
        &self.context
    }

    /// Access the DefaultSpecCurator for spec coherence and drift evaluation.
    ///
    /// When `CuratorContext` has a `loop_dispatch_tx`, the spec curator
    /// sends `SpecDriftAlert` payloads through the Communication Loop.
    ///
    /// REQ: P9-agt-curator-agent-spec-curator
    /// expect: "The system regulates agent behavior through cybernetic feedback" [P9]
    /// \[P9\] Motivating: Homeostatic Self-Regulation — DefaultSpecCurator detects specification drift
    /// pre:  (none — accessor).
    /// post: Returns a reference to the inner `DefaultSpecCurator`.
    #[rs::contract(id = "P9-agt-curator-agent-spec-curator", principle = "P9")]
    pub fn spec_curator(&self) -> &spec_curator::DefaultSpecCurator {
        &self.spec_curator
    }
}

// Re-export persona types for convenience
pub use metacognition::{
    EscalationAlert, EscalationPolicy, EscalationSeverity, EscalationTrigger, HealthSnapshot,
    MetacognitionConfig, MetacognitionError, MetacognitionLoop,
};
pub use spec_curator::DefaultSpecCurator;
