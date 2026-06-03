//! CurationContext — Runtime composition of Curator capability handles

use crate::communication::dispatch::MessageDispatch;
use crate::escalation::EscalationQueue;
use hkask_cns::CnsRuntime;
use hkask_storage::NuEventStore;
use hkask_types::CuratorHandle;
use hkask_types::loops::curation::CuratorDirective;
use hkask_types::loops::dispatch::TraceId;
use std::sync::Arc;

/// CuratorContext — aggregates the runtime references the Curator needs.
pub struct CuratorContext {
    handle: CuratorHandle,
    cns: Arc<CnsRuntime>,
    dispatch: Arc<MessageDispatch>,
    escalation_queue: Arc<EscalationQueue>,
    /// NuEvent store for algedonic review queries.
    /// Curation reads from the persistent log, not live CNS state.
    nu_event_store: Option<Arc<NuEventStore>>,
}

impl CuratorContext {
    pub fn new(
        handle: CuratorHandle,
        cns: Arc<CnsRuntime>,
        dispatch: Arc<MessageDispatch>,
        escalation_queue: Arc<EscalationQueue>,
    ) -> Self {
        Self {
            handle,
            cns,
            dispatch,
            escalation_queue,
            nu_event_store: None,
        }
    }

    /// Create CuratorContext with a NuEvent store for algedonic review.
    pub fn with_nu_event_store(
        handle: CuratorHandle,
        cns: Arc<CnsRuntime>,
        dispatch: Arc<MessageDispatch>,
        escalation_queue: Arc<EscalationQueue>,
        nu_event_store: Arc<NuEventStore>,
    ) -> Self {
        Self {
            handle,
            cns,
            dispatch,
            escalation_queue,
            nu_event_store: Some(nu_event_store),
        }
    }

    /// Access the CuratorHandle (capability handle).
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

    /// Issue a CuratorDirective unconditionally.
    ///
    /// Curation (Loop 5) governs Cybernetics (Loop 6) per the authority DAG,
    /// so Curator directives MUST NOT be dampened by a Cybernetics dampener.
    /// Dampening is applied at the Cybernetics receipt boundary instead.
    pub async fn issue_directive(&self, directive: CuratorDirective) -> Option<TraceId> {
        let trace_id = self
            .dispatch
            .send_curator_directive(directive, *self.handle.curator_id())
            .await;
        Some(trace_id)
    }
}
