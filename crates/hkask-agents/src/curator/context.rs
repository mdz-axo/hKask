//! CurationContext — Runtime composition of Curator capability handles

use crate::a2a::A2ARuntime;
use crate::consent::ConsentManager;
use hkask_cns::CnsRuntime;
use hkask_cns::types::loops::CommunicationEvent;
use hkask_storage::EscalationQueue;
use hkask_storage::NuEventStore;
use hkask_templates::ManifestExecutor;
use hkask_types::DataCategory;
use hkask_types::curator::{CuratorDirective, CuratorHandle};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

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
    a2a_port: Option<Arc<A2ARuntime>>,
    /// Manifest executor for invoking KnowAct templates.
    /// Per P3 (Generative Space), selection intelligence lives in the
    /// template system. Uses `RwLock<Option<>>` for late binding — the
    /// executor is constructed after MCP pods, but CuratorContext must
    /// exist before them. Set via `set_manifest_executor()`.
    manifest_executor: RwLock<Option<Arc<ManifestExecutor>>>,
    /// Consent manager for P2 sovereignty checks. Optional because some
    /// CuratorContext users (standalone CLI metacognition) do not have
    /// a consent store. When present, the Curation Loop uses it to gate
    /// auto-consolidation behind affirmative consent.
    consent_manager: Option<Arc<ConsentManager>>,
    /// Pending communication events from Matrix, drained by metacognition.
    /// Events arrive here from CurationLoop.sense() on each loop tick (~10s).
    /// Metacognition drains them on CLI-triggered cycles — there may be a
    /// one-cycle delay between push and drain. This is eventual consistency;
    /// no events are lost.
    pub(crate) pending_communication: Arc<RwLock<Vec<CommunicationEvent>>>,
}

impl CuratorContext {
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — CuratorContext bundles regulatory dependencies
    /// pre:  `handle` is a valid `CuratorHandle`; `cns` is a valid
    ///       `Arc<CnsRuntime>`; `curator_directive_tx` is `Some` or `None`;
    ///       `escalation_queue` is a valid `Arc<EscalationQueue>`.
    /// post: Returns a `CuratorContext` with no NuEvent store, no A2A
    ///       port, and no manifest executor.
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
            manifest_executor: RwLock::new(None),
            pending_communication: Arc::new(RwLock::new(Vec::new())),
            consent_manager: None,
        }
    }

    /// Create CuratorContext with a NuEvent store for algedonic review.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — NuEvent store enables algedonic review
    /// pre:  All arguments are valid (same as `new`); `nu_event_store` is
    ///       a valid `Arc<NuEventStore>`.
    /// post: Returns a `CuratorContext` with `nu_event_store` set, no
    ///       A2A port, and no manifest executor.
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
            manifest_executor: RwLock::new(None),
            pending_communication: Arc::new(RwLock::new(Vec::new())),
            consent_manager: None,
        }
    }

    /// Builder: attach an A2A port for A2A bot-directed messaging.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P4\] Motivating: Clear Boundaries — A2A port lets Curator direct bots
    /// pre:  `a2a_runtime` is a valid `Arc<A2ARuntime>`.
    /// post: Returns `self` with `a2a_port` set to `Some(a2a_runtime)`.
    pub fn with_a2a(mut self, a2a_runtime: Arc<A2ARuntime>) -> Self {
        self.a2a_port = Some(a2a_runtime);
        self
    }

    /// Builder: attach a ConsentManager so the Curation Loop can enforce P2
    /// affirmative consent before autonomous actions such as auto-consolidation.
    ///
    /// expect: "Agent consent is explicitly granted, scoped, and revocable"
    /// \[P2\] Motivating: Affirmative Consent — CuratorContext carries the consent manager
    /// pre:  `consent_manager` is a valid `Arc<ConsentManager>`.
    /// post: Returns `self` with the consent manager set.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_consent_manager(mut self, consent_manager: Arc<ConsentManager>) -> Self {
        self.consent_manager = Some(consent_manager);
        self
    }

    /// Late-binding setter: attach a ManifestExecutor after construction.
    ///
    /// The ManifestExecutor depends on McpDispatcher, which is built after
    /// CuratorContext. This setter allows the executor to be wired in later
    /// without changing the construction order.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P3\] Motivating: Generative Space — late-binding template executor
    /// pre:  `executor` is a valid `Arc<ManifestExecutor>`.
    /// post: `manifest_executor` is set to `Some(executor)`.
    pub async fn set_manifest_executor(&self, executor: Arc<ManifestExecutor>) {
        *self.manifest_executor.write().await = Some(executor);
    }

    /// Access the CuratorHandle (capability handle).
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — accessor for the Curator capability handle
    /// pre:  (none — accessor).
    /// post: Returns a reference to the inner `CuratorHandle`.
    pub fn handle(&self) -> &CuratorHandle {
        &self.handle
    }

    /// Access the CNS runtime for health checks and variety queries.
    pub(crate) fn cns(&self) -> &Arc<CnsRuntime> {
        &self.cns
    }

    /// Drain pending communication events for processing.
    pub async fn drain_communication_events(&self) -> Vec<CommunicationEvent> {
        let mut events = self.pending_communication.write().await;
        std::mem::take(&mut *events)
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
    pub(crate) fn a2a(&self) -> Option<&Arc<A2ARuntime>> {
        self.a2a_port.as_ref()
    }

    /// Access the ConsentManager, if one has been wired.
    ///
    /// Returns None in contexts where sovereignty checks are not applicable
    /// (e.g., standalone CLI metacognition).
    pub(crate) fn consent_manager(&self) -> Option<&Arc<ConsentManager>> {
        self.consent_manager.as_ref()
    }

    /// Access the ManifestExecutor for template invocations.
    ///
    /// Returns None if no executor has been set yet (late binding —
    /// set via `set_manifest_executor()` after MCP pods are built).
    pub(crate) async fn manifest_executor(&self) -> Option<Arc<ManifestExecutor>> {
        self.manifest_executor.read().await.clone()
    }

    /// Issue a CuratorDirective through the OCAP-gated channel.
    ///
    /// Curation (Loop 5) governs Cybernetics (Loop 6) per the authority DAG,
    /// so Curator directives MUST NOT be dampened by a Cybernetics dampener.
    /// Dampening is applied at the Cybernetics receipt boundary instead.
    ///
    /// **OCAP Verification (Magna Carta Curator Responsibility #1):**
    /// Every directive issuance verifies the CuratorHandle's write capability
    /// before sending. Directives that fail OCAP verification are refused —
    /// this is the Magna Carta enforcement gate at the directive boundary.
    ///
    /// When no channel is configured (e.g., standalone CLI), this is a no-op.
    ///
    /// expect: "The system regulates agent behavior through cybernetic feedback"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — OCAP-gated directive issuance
    /// \[P4\] Constraining: Clear Boundaries — directives require write capability
    /// pre:  `directive` is a valid `CuratorDirective`; `self.handle` is a
    ///       valid `CuratorHandle`.
    /// post: If the handle lacks write capability, the directive is refused
    ///       and an error is logged. Otherwise, if `curator_directive_tx` is
    ///       `Some`, the directive is sent; logs a warning if the send fails.
    ///       If `curator_directive_tx` is `None`, this is a no-op.
    pub async fn issue_directive(&self, directive: CuratorDirective) {
        // Magna Carta Curator Responsibility #1: OCAP verification.
        // The CuratorHandle must prove write capability before any directive
        // issuance. This is a structural gate — the singleton handle is always
        // authorized — but the check exists as a contract boundary.
        if !self.handle.can_write(&DataCategory::Public) {
            tracing::error!(
                target: "curator.context",
                directive = directive.variant_name(),
                "OCAP verification failed: CuratorHandle lacks write capability. Directive refused."
            );
            return;
        }

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
