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
//! ├── curation_loop: `Arc<CurationLoop>`   // pure regulatory
//! ├── metacognition: `Arc<MetacognitionLoop>` // persona: observe & adapt
//! └── context: `Arc<CuratorContext>`       // capability-disciplined access
//! ```

pub mod cat;
pub mod metacognition;
pub mod spec_curator;

use crate::curator::context::CuratorContext;
use crate::curator::curation_loop::CurationLoop;
use crate::pod::CommunicationPosture;
use hkask_cns::types::loops::CurationInput;
use hkask_memory::ConsolidationBridge;
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
    /// Federation link manager — set via `with_federation()`. Handles
    /// InviteToFederation, PauseFederationLink, RevokeFederationMember,
    /// LeaveFederation, and DissolveFederation directives.
    link_manager: Option<Arc<dyn hkask_ports::federation::FederationDispatch>>,
}

impl CuratorAgent {
    /// Create a new Curator Agent with default configuration.
    ///
    /// The agent internally creates both the `MetacognitionLoop` and
    /// `CurationLoop`, connecting them through the shared `CuratorContext`.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — CuratorAgent composes Curation + Metacognition
    /// pre:  `context` is a valid `Arc<CuratorContext>`.
    /// post: Returns a `CuratorAgent` with default `MetacognitionConfig`,
    ///       a new `CurationLoop`, and a default `DefaultSpecCurator`.
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
            link_manager: None,
        }
    }

    /// Create a Curator Agent with custom metacognition configuration.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — custom metacognition configuration
    /// \[P7\] Constraining: Evolutionary Architecture — thresholds emerge from real usage
    /// pre:  `context` is a valid `Arc<CuratorContext>`; `config` is a
    ///       valid `MetacognitionConfig`.
    /// post: Returns a `CuratorAgent` with the given config, a new
    ///       `CurationLoop`, and a default `DefaultSpecCurator`.
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
            link_manager: None,
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
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — consolidation wired into CuratorAgent
    /// \[P2\] Constraining: Affirmative Consent — auto-consolidation is opt-in and consent-gated
    /// pre:  `context` is a valid `Arc<CuratorContext>`; `config` is a
    ///       valid `MetacognitionConfig`; `consolidation` is a valid
    ///       `Arc<ConsolidationBridge>`; `inbox_rx` and `inbox_tx` are
    ///       `Some` or `None`; `auto_consolidation_enabled` controls whether
    ///       the Curator daemon may auto-run consolidation.
    /// post: Returns a `CuratorAgent` with consolidation wired; if
    ///       `inbox_rx` is `Some`, the curation loop's inbox is set;
    ///       if `inbox_tx` is `Some`, the spec curator's channel is set.
    pub fn with_consolidation(
        context: Arc<CuratorContext>,
        config: metacognition::MetacognitionConfig,
        consolidation: Arc<ConsolidationBridge>,
        inbox_rx: Option<tokio::sync::mpsc::UnboundedReceiver<CurationInput>>,
        inbox_tx: Option<tokio::sync::mpsc::UnboundedSender<CurationInput>>,
        auto_consolidation_enabled: bool,
    ) -> Self {
        let metacognition = Arc::new(metacognition::MetacognitionLoop::new(
            Arc::clone(&context),
            config,
        ));
        let curator_handle = context.handle().clone();
        let mut curation_loop =
            CurationLoop::with_consolidation(curator_handle, Arc::clone(&context), consolidation)
                .with_auto_consolidation_enabled(auto_consolidation_enabled);
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
            link_manager: None,
        }
    }

    /// Access the Curation Loop (pure regulatory).
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — accessor for the pure regulatory loop
    /// pre:  (none — accessor).
    /// post: Returns a reference to the inner `Arc<CurationLoop>`.
    pub fn curation_loop(&self) -> &Arc<CurationLoop> {
        &self.curation_loop
    }

    /// Access the Metacognition Loop (persona/agent).
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — accessor for the persona/agent loop
    /// pre:  (none — accessor).
    /// post: Returns a reference to the inner `Arc<MetacognitionLoop>`.
    pub fn metacognition(&self) -> &Arc<metacognition::MetacognitionLoop> {
        &self.metacognition
    }

    /// Access the CuratorContext (capability-disciplined runtime references).
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — accessor for capability-disciplined context
    /// pre:  (none — accessor).
    /// post: Returns a reference to the inner `Arc<CuratorContext>`.
    pub fn context(&self) -> &Arc<CuratorContext> {
        &self.context
    }

    /// Access the DefaultSpecCurator for spec coherence and drift evaluation.
    ///
    /// When `CuratorContext` has a `loop_dispatch_tx`, the spec curator
    /// sends `SpecDriftAlert` payloads through the Communication Loop.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — DefaultSpecCurator detects specification drift
    /// pre:  (none — accessor).
    /// post: Returns a reference to the inner `DefaultSpecCurator`.
    pub fn spec_curator(&self) -> &spec_curator::DefaultSpecCurator {
        &self.spec_curator
    }

    /// Attach a FederationLinkManager for federation directive dispatch.
    pub fn with_federation(
        mut self,
        link_manager: Arc<dyn hkask_ports::federation::FederationDispatch>,
    ) -> Self {
        self.link_manager = Some(link_manager);
        self
    }

    /// Set communication posture for the metacognition loop (CAT framework).
    ///
    /// Creates a new `MetacognitionLoop` with the given persona name and
    /// convergence bias, using the existing config and context.
    pub fn with_communication_posture(mut self, posture: CommunicationPosture) -> Self {
        self.metacognition = Arc::new(metacognition::MetacognitionLoop::with_posture(
            Arc::clone(&self.context),
            self.metacognition.config().clone(),
            posture,
        ));
        self
    }

    /// Dispatch a CuratorDirective to the federation link manager.
    ///
    /// Called by the CurationLoop (or CLI) when federation-related directives
    /// need to be executed. Silently ignores directives that aren't federation-related.
    pub async fn handle_federation_directive(
        &self,
        directive: &hkask_types::curator::CuratorDirective,
    ) -> Result<(), String> {
        use hkask_types::curator::CuratorDirective;
        let lm = self
            .link_manager
            .as_ref()
            .ok_or("no federation link manager configured")?;

        match directive {
            CuratorDirective::InviteToFederation {
                peer_replica,
                peer_server_domain,
                peer_matrix_domain,
                peer_curator_matrix_id,
                message: _, // Dropped for now
            } => {
                lm.register_peer(
                    peer_replica.clone(),
                    peer_server_domain.clone(),
                    peer_matrix_domain.clone(),
                    peer_curator_matrix_id.clone(),
                )
                .await;
                lm.invite(peer_replica.clone())
                    .await
                    .map_err(|e| format!("invite failed: {e}"))?;
            }
            CuratorDirective::AcceptFederationInvite { invitation_id } => {
                // invitation_id is the replica ID of the inviter
                lm.accept(invitation_id.clone())
                    .await
                    .map_err(|e| format!("accept failed: {e}"))?;
            }
            CuratorDirective::RejectFederationInvite {
                invitation_id,
                reason: _, // Dropped for now
            } => {
                lm.reject(invitation_id.clone())
                    .await
                    .map_err(|e| format!("reject failed: {e}"))?;
            }
            CuratorDirective::PauseFederationLink {
                peer_replica,
                reason,
            } => {
                lm.pause(peer_replica.clone(), reason.clone())
                    .await
                    .map_err(|e| format!("pause failed: {e}"))?;
            }
            CuratorDirective::ResumeFederationLink { peer_replica } => {
                lm.resume(peer_replica.clone())
                    .await
                    .map_err(|e| format!("resume failed: {e}"))?;
            }
            CuratorDirective::RevokeFederationMember {
                peer_replica,
                reason,
            } => {
                lm.revoke(peer_replica.clone(), reason.clone())
                    .await
                    .map_err(|e| format!("revoke failed: {e}"))?;
            }
            CuratorDirective::LeaveFederation { reason } => {
                lm.leave(reason.clone())
                    .await
                    .map_err(|e| format!("leave failed: {e}"))?;
            }
            CuratorDirective::DissolveFederation { reason } => {
                lm.leave(format!("dissolved: {reason}"))
                    .await
                    .map_err(|e| format!("dissolve failed: {e}"))?;
            }
            _ => {} // Not a federation directive — silently ignore
        }
        Ok(())
    }
}

// Re-export persona types for convenience
pub use metacognition::{
    EscalationAlert, EscalationPolicy, EscalationSeverity, EscalationTrigger, HealthSnapshot,
    MetacognitionConfig, MetacognitionLoop,
};
pub use spec_curator::DefaultSpecCurator;
