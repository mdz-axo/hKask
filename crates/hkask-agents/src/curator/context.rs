//! CuratorContext — Runtime composition of Curator capability handles
//!
//! The CuratorContext aggregates the runtime adapters that the Curator needs
//! to exercise its capabilities. It is the runtime companion to `CuratorHandle`
//! (defined in `hkask-types`), which defines OCAP boundaries at the type level.
//!
//! # OCAP Boundaries (enforced by CuratorHandle at the type level)
//!
//! - **CAN** read all loop state (via `cns`, `dispatch`)
//! - **CAN** write governance policy (via `dispatch` sending directives)
//! - **CAN** write observability policy (via `cns.calibrate_threshold()`)
//! - **CAN** write to semantic memory (via future `SemanticWriteHandle` adapter)
//! - **CANNOT** run inference (not in context)
//! - **CANNOT** emit spans directly (not in context)
//! - **CANNOT** access private episodic triples (not in context)

use crate::adapters::CnsGovernWriteAdapter;
use crate::curator::dampener::Dampener;
use crate::curator::dispatch::MessageDispatch;
use crate::curator::escalation::EscalationQueue;
use hkask_types::loops::curation::CuratorDirective;
use hkask_types::loops::dispatch::TraceId;
use hkask_types::{CuratorHandle, WebID};
use std::sync::Arc;
use tracing::info;

/// CuratorContext — Aggregates the runtime adapters the Curator needs.
///
/// This struct holds the actual runtime references that the Curation loop
/// uses to exercise its capabilities. The `CuratorHandle` provides type-level
/// OCAP enforcement; `CuratorContext` provides the runtime wiring.
///
/// # Subloops served
///
/// - 5.1 Escalation Routing (ROUTE) — via `escalation_queue` and `dispatch`
/// - 5.2 Bot Evaluation / Kata Coaching (ADAPT) — via `cns` reads and `dispatch` directives
/// - 5.3 Threshold Calibration (ADAPT) — via `cns.calibrate_threshold()`
///
/// # DAMPEN integration (6.3)
///
/// The `dampener` field provides DAMPEN functionality: suppressing repeated
/// directives within a configurable time window. The `issue_directive()`
/// method automatically checks dampening before sending.
pub struct CuratorContext {
    /// The Curator's capability handle (OCAP enforcement at the type level)
    handle: CuratorHandle,
    /// CNS governance write adapter (read + calibrate thresholds)
    cns: Arc<CnsGovernWriteAdapter>,
    /// Message dispatch for inter-loop communication
    dispatch: Arc<MessageDispatch>,
    /// Escalation queue for human review
    escalation_queue: Arc<EscalationQueue>,
    /// Dampener for suppressing repeated directives (6.3 DAMPEN)
    dampener: Arc<Dampener>,
}

impl CuratorContext {
    /// Create a new CuratorContext with all required handles and default dampener.
    pub fn new(
        handle: CuratorHandle,
        cns: Arc<CnsGovernWriteAdapter>,
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

    /// Create a new CuratorContext with a custom dampener window.
    pub fn with_dampener_window(
        handle: CuratorHandle,
        cns: Arc<CnsGovernWriteAdapter>,
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

    /// The Curator's capability handle (OCAP boundaries).
    pub fn handle(&self) -> &CuratorHandle {
        &self.handle
    }

    /// CNS governance write adapter (read + calibrate thresholds).
    pub fn cns(&self) -> &CnsGovernWriteAdapter {
        &self.cns
    }

    /// Message dispatch for inter-loop communication.
    pub fn dispatch(&self) -> &MessageDispatch {
        &self.dispatch
    }

    /// Escalation queue for human review.
    pub fn escalation_queue(&self) -> &EscalationQueue {
        &self.escalation_queue
    }

    /// Dampener for suppressing repeated directives.
    pub fn dampener(&self) -> &Dampener {
        &self.dampener
    }

    /// The Curator's WebID (convenience accessor).
    pub fn curator_id(&self) -> &WebID {
        self.handle.curator_id()
    }

    /// Issue a CuratorDirective through the dispatch with DAMPEN filtering.
    ///
    /// This method:
    /// 1. Checks the dampener to see if this directive was issued recently
    /// 2. If dampened, logs and returns None without sending
    /// 3. If not dampened, sends through dispatch and returns the TraceId
    ///
    /// This implements the DAMPEN messenger function (6.3) on the
    /// Curation→Governance→Observability→Curation feedback edge.
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
