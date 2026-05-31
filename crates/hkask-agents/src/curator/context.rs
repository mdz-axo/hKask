//! CuratorContext — Runtime composition of Curator capability handles

use crate::curator::dampener::Dampener;
use crate::curator::dispatch::MessageDispatch;
use crate::curator::escalation::EscalationQueue;
use hkask_cns::CnsRuntime;
use hkask_types::loops::curation::CuratorDirective;
use hkask_types::loops::dispatch::TraceId;
use hkask_types::{CuratorHandle, WebID};
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

    pub fn with_dampener_window(
        handle: CuratorHandle,
        cns: Arc<CnsRuntime>,
        dispatch: Arc<MessageDispatch>,
        escalation_queue: Arc<EscalationQueue>,
        window: std::time::Duration,
    ) -> Self {
        Self {
            handle,
            cns,
            dispatch,
            escalation_queue,
            dampener: Arc::new(Dampener::with_window(window)),
        }
    }

    pub fn handle(&self) -> &CuratorHandle {
        &self.handle
    }

    pub fn cns(&self) -> &CnsRuntime {
        &self.cns
    }

    pub fn dispatch(&self) -> &MessageDispatch {
        &self.dispatch
    }

    pub fn escalation_queue(&self) -> &EscalationQueue {
        &self.escalation_queue
    }

    pub fn dampener(&self) -> &Dampener {
        &self.dampener
    }

    pub fn curator_id(&self) -> &WebID {
        self.handle.curator_id()
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
            .send_curator_directive(directive, *self.curator_id())
            .await;
        Some(trace_id)
    }
}
