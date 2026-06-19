//! CurationContext — Runtime composition of Curator capability handles

use crate::ports::A2APort;
use hkask_cns::CnsRuntime;
use hkask_storage::EscalationQueue;
use hkask_storage::NuEventStore;
use hkask_types::CuratorHandle;
use hkask_types::loops::curation::CuratorDirective;
use std::sync::Arc;
use tokio::sync::mpsc;

/// CuratorContext — aggregates the runtime references the Curator needs.
pub struct CuratorContext {
    handle: CuratorHandle,
    cns: Arc<CnsRuntime>,
    /// Direct channel for issuing CuratorDirectives to Cybernetics.
    /// None when running standalone (e.g., CLI metacognition) where no
    /// CyberneticsLoop receiver exists.
    curator_directive_tx: Option<mpsc::UnboundedSender<CuratorDirective>>,
    escalation_queue: Arc<EscalationQueue>,
    /// NuEvent store for algedonic review queries.
    /// Curation reads from the persistent log, not live CNS state.
    nu_event_store: Option<Arc<NuEventStore>>,
    /// A2A port for A2A messaging (e.g. directing bots).
    /// Optional so existing construction sites don't break.
    a2a_port: Option<Arc<dyn A2APort>>,
}

impl CuratorContext {
    /// \[P9\] Motivating: Homeostatic Self-Regulation — CuratorContext bundles regulatory dependencies
    ///       `Arc<CnsRuntime>`; `curator_directive_tx` is `Some` or `None`;
    ///       `escalation_queue` is a valid `Arc<EscalationQueue>`.
    ///       port.
    pub fn new(
        handle: CuratorHandle,
        cns: Arc<CnsRuntime>,
        curator_directive_tx: Option<mpsc::UnboundedSender<CuratorDirective>>,
        escalation_queue: Arc<EscalationQueue>,
    ) -> Self {
        Self {
            handle,
            cns,
            curator_directive_tx,
            escalation_queue,
            nu_event_store: None,
            a2a_port: None,
        }
    }

    /// Create CuratorContext with a NuEvent store for algedonic review.
    ///
    /// \[P9\] Motivating: Homeostatic Self-Regulation — NuEvent store enables algedonic review
    ///       a valid `Arc<NuEventStore>`.
    ///       A2A port.
    pub fn with_nu_event_store(
        handle: CuratorHandle,
        cns: Arc<CnsRuntime>,
        curator_directive_tx: Option<mpsc::UnboundedSender<CuratorDirective>>,
        escalation_queue: Arc<EscalationQueue>,
        nu_event_store: Arc<NuEventStore>,
    ) -> Self {
        Self {
            handle,
            cns,
            curator_directive_tx,
            escalation_queue,
            nu_event_store: Some(nu_event_store),
            a2a_port: None,
        }
    }

    /// Builder: attach an A2A port for A2A bot-directed messaging.
    ///
    /// \[P4\] Motivating: Clear Boundaries — A2A port lets Curator direct bots
    pub fn with_a2a(mut self, a2a_port: Arc<dyn A2APort>) -> Self {
        self.a2a_port = Some(a2a_port);
        self
    }

    /// Access the CuratorHandle (capability handle).
    ///
    /// \[P9\] Motivating: Homeostatic Self-Regulation — accessor for the Curator capability handle
    pub fn handle(&self) -> &CuratorHandle {
        &self.handle
    }

    /// Access the CNS runtime for health checks and variety queries.
    pub(crate) fn cns(&self) -> &Arc<CnsRuntime> {
        &self.cns
    }

    /// Access the NuEvent store for algedonic review queries.
    ///
    /// Curation reads from the persistent event log, not live CNS state.
    /// Returns None if no NuEvent store is configured (graceful degradation).
    pub(crate) fn nu_event_store(&self) -> Option<&Arc<NuEventStore>> {
        self.nu_event_store.as_ref()
    }

    /// Access the escalation queue for posting human review items.
    pub(crate) fn escalation_queue(&self) -> &Arc<EscalationQueue> {
        &self.escalation_queue
    }

    /// Access the A2A port for A2A messaging.
    ///
    /// Returns None if no A2A port is configured (graceful degradation).
    pub(crate) fn a2a(&self) -> Option<&Arc<dyn A2APort>> {
        self.a2a_port.as_ref()
    }

    /// Issue a CuratorDirective unconditionally on the direct channel.
    ///
    /// Curation (Loop 5) governs Cybernetics (Loop 6) per the authority DAG,
    /// so Curator directives MUST NOT be dampened by a Cybernetics dampener.
    /// Dampening is applied at the Cybernetics receipt boundary instead.
    ///
    /// When no channel is configured (e.g., standalone CLI), this is a no-op.
    ///
    /// \[P9\] Motivating: Homeostatic Self-Regulation — issue directives to the Curation Loop
    ///       logs a warning if the send fails. If `None`, this is a no-op.
    pub async fn issue_directive(&self, directive: CuratorDirective) {
        if let Some(ref tx) = self.curator_directive_tx
            && let Err(e) = tx.send(directive)
        {
            tracing::warn!(
                target: "curator.context",
                error = %e,
                "Failed to send CuratorDirective on direct channel"
            );
        }
    }
}
