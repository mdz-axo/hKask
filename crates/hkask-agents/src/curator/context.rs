//! CuratorContext — Runtime composition of Curator capability handles

use crate::communication::dispatch::MessageDispatch;
use crate::curator::dampener::Dampener;
use crate::curator::escalation::EscalationQueue;
use hkask_cns::CnsRuntime;
use hkask_types::CuratorHandle;
use hkask_types::loops::curation::CuratorDirective;
use hkask_types::loops::dispatch::TraceId;
use std::sync::Arc;
use tracing::info;

/// CuratorContext — aggregates the runtime references the Curator needs.
pub struct CuratorContext {
    handle: CuratorHandle,
    cns: Arc<CnsRuntime>,
    dispatch: Arc<MessageDispatch>,
    escalation_queue: Arc<EscalationQueue>,
    dampener: Arc<Dampener>,
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
            dampener: Arc::new(Dampener::new()),
        }
    }

    /// Access the CNS runtime for health checks and variety queries.
    pub(crate) fn cns(&self) -> &Arc<CnsRuntime> {
        &self.cns
    }

    /// Access the escalation queue for posting human review items.
    pub(crate) fn escalation_queue(&self) -> &Arc<EscalationQueue> {
        &self.escalation_queue
    }

    /// Issue a CuratorDirective with DAMPEN filtering.
    pub async fn issue_directive(&self, directive: CuratorDirective) -> Option<TraceId> {
        if self.dampener.should_dampen(&directive).await {
            info!(
                target: "curator.context",
                directive_type = ?directive,
                "Directive dampened (repeated within window)"
            );
            return None;
        }

        let trace_id = self
            .dispatch
            .send_curator_directive(directive, *self.handle.curator_id())
            .await;
        Some(trace_id)
    }
}
