//! Communication Loop — dumb transport pipe (Loop 4)
//!
//! send → observe delivery → confirm
//!
//! The Communication Loop routes `LoopMessage`s between the 6 loops.
//! It does NOT dampen, throttle, or circuit-break — those are Cybernetics
//! regulation actions applied TO communication channels.
//!
//! Essential messenger function:
//! - 4.1 DISPATCH (GUARD+ROUTE) — priority-ordered message queuing

use crate::communication::dispatch::MessageDispatch;
use hkask_types::loops::dispatch::{DispatchTarget, LoopMessage, WorkerKind};
use hkask_types::loops::{Deviation, HkaskLoop, LoopAction, LoopId, Signal};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Communication Loop — routes inter-loop messages.
///
/// Wraps `MessageDispatch` (the priority queue) and receives
/// `LoopMessage`s from the dispatch channel. The loop's `tick()`
/// dequeues messages and delivers them to target loop inboxes.
///
/// Note: `max_deliveries_per_tick` is NOT a rate limiter — it's a delivery
/// batch limit that prevents unbounded event loop blocking. Actual throttling
/// and circuit-breaking are Cybernetics (L6) concerns applied TO the
/// Communication dispatch boundary, not within Communication itself.
pub(crate) struct CommunicationLoop {
    /// Priority-ordered message queue
    dispatch: Arc<MessageDispatch>,
    /// Per-loop inbox senders for message delivery
    loop_senders: Arc<
        RwLock<std::collections::HashMap<LoopId, tokio::sync::mpsc::UnboundedSender<LoopMessage>>>,
    >,
    /// Per-worker inbox senders for worker message delivery
    worker_senders: Arc<
        RwLock<
            std::collections::HashMap<WorkerKind, tokio::sync::mpsc::UnboundedSender<LoopMessage>>,
        >,
    >,
    /// Maximum messages to deliver per tick (prevents unbounded delivery)
    max_deliveries_per_tick: usize,
}

impl CommunicationLoop {
    /// Create a new Communication Loop with an empty dispatch and no target loops.
    pub fn new(dispatch: Arc<MessageDispatch>) -> Self {
        Self {
            dispatch,
            loop_senders: Arc::new(RwLock::new(std::collections::HashMap::new())),
            worker_senders: Arc::new(RwLock::new(std::collections::HashMap::new())),
            max_deliveries_per_tick: 64,
        }
    }

    /// Register a target loop's inbox channel for message delivery.
    ///
    /// When a `LoopMessage` targets `loop_id`, it will be sent through
    /// the provided sender. The target loop reads from the corresponding
    /// receiver in its own `sense()` or dedicated inbox processing.
    pub async fn register_loop_inbox(
        &self,
        loop_id: LoopId,
        sender: tokio::sync::mpsc::UnboundedSender<LoopMessage>,
    ) {
        let mut senders = self.loop_senders.write().await;
        senders.insert(loop_id, sender);
    }

    /// Register a worker's inbox channel for message delivery.
    ///
    /// When a `LoopMessage` targets a `DispatchTarget::Worker(kind)`,
    /// it will be sent through the provided sender.
    pub async fn register_worker_inbox(
        &self,
        worker_kind: WorkerKind,
        sender: tokio::sync::mpsc::UnboundedSender<LoopMessage>,
    ) {
        let mut senders = self.worker_senders.write().await;
        senders.insert(worker_kind, sender);
    }

    // ====================================================================
    // Explicit 4-stage cycle: sense → compare → compute → act
    // ====================================================================

    /// **Sense stage** (sense → compare → compute → act):
    /// Read queue depth via `dispatch.len()` and count registered loop
    /// senders. Produces afferent signals for queue depth and registered
    /// loop count.
    pub async fn sense(&self) -> Vec<Signal> {
        <Self as HkaskLoop>::sense(self).await
    }

    /// **Compare stage** (sense → compare → compute → act):
    /// Check if queue depth exceeds the backpressure threshold (set-point
    /// 100 messages). Detects deviations when queue depth is above the
    /// healthy operating limit.
    pub async fn compare(&self, signals: &[Signal]) -> Vec<Deviation> {
        <Self as HkaskLoop>::compare(self, signals).await
    }

    /// **Compute stage** (sense → compare → compute → act):
    /// Determine routing decision (deliver, reject, defer). Communication
    /// is a dumb transport pipe — it does NOT produce regulatory actions.
    /// Queue-depth signals flow upward to Cybernetics for regulation.
    pub async fn compute(&self, deviations: &[Deviation]) -> Vec<LoopAction> {
        <Self as HkaskLoop>::compute(self, deviations).await
    }

    /// **Act stage** (sense → compare → compute → act):
    /// Deliver message or emit backpressure signal. Dequeues messages
    /// from the dispatch and delivers them to target loop inboxes, up to
    /// `max_deliveries_per_tick` per cycle.
    pub async fn act(&self, actions: &[LoopAction]) {
        <Self as HkaskLoop>::act(self, actions).await
    }
}

#[async_trait::async_trait]
impl HkaskLoop for CommunicationLoop {
    fn id(&self) -> LoopId {
        LoopId::Communication
    }

    /// Sense: read queue depths and delivery state.
    async fn sense(&self) -> Vec<Signal> {
        let queue_depth = self.dispatch.len().await;
        let senders = self.loop_senders.read().await;
        let registered_loops = senders.len();

        // Produce signals about dispatch health
        vec![
            Signal::new(
                LoopId::Communication,
                "queue_depth",
                queue_depth as f64,
                100.0, // set-point: 100 messages before considered overloaded
            ),
            Signal::new(
                LoopId::Communication,
                "registered_loops",
                registered_loops as f64,
                6.0, // set-point: all 6 loops types should be registered
            ),
        ]
    }

    /// Compute: Communication is a dumb transport pipe — it does NOT
    /// dampen, throttle, or circuit-break. It produces no regulatory
    /// actions. Queue-depth signals are emitted in `sense()` for
    /// Cybernetics to consume through its own sense cycle.
    async fn compute(&self, _deviations: &[Deviation]) -> Vec<LoopAction> {
        // Communication does not govern. Signals flow upward;
        // Cybernetics decides whether to throttle.
        Vec::new()
    }

    /// Act: dequeue messages from dispatch and deliver to target loop inboxes.
    ///
    /// Delivers up to `max_deliveries_per_tick` messages per cycle,
    /// then routes any overflow actions through the dispatch channel.
    async fn act(&self, actions: &[LoopAction]) {
        // Route any regulatory actions produced by compute()
        for action in actions {
            tracing::info!(
                target: "communication.loop",
                action_type = ?action.action_type,
                target_loop = %action.target,
                "Communication Loop regulatory action"
            );
        }

        // Deliver queued messages to target loops
        let senders = self.loop_senders.read().await;
        let mut delivered = 0;
        while delivered < self.max_deliveries_per_tick {
            let msg = match self.dispatch.receive().await {
                Some(m) => m,
                None => break, // queue empty
            };

            let target_id = match msg.target_loop {
                Some(DispatchTarget::Loop(id)) => id,
                Some(DispatchTarget::Worker(kind)) => {
                    // Route to worker inbox
                    let worker_senders = self.worker_senders.read().await;
                    if let Some(sender) = worker_senders.get(&kind) {
                        if let Err(e) = sender.send(msg) {
                            tracing::warn!(
                                target: "communication.loop",
                                worker_kind = %kind,
                                error = %e,
                                "Failed to deliver message to worker inbox"
                            );
                        }
                    } else {
                        tracing::warn!(
                            target: "communication.loop",
                            worker_kind = %kind,
                            "No inbox registered for worker — message dropped"
                        );
                    }
                    delivered += 1;
                    continue;
                }
                None => {
                    // Broadcast — no specific target; log and skip
                    tracing::debug!(
                        target: "communication.loop",
                        trace_id = %msg.trace_id,
                        origin = ?msg.origin,
                        "Broadcast message has no specific target loop"
                    );
                    delivered += 1;
                    continue;
                }
            };

            if let Some(sender) = senders.get(&target_id) {
                if let Err(e) = sender.send(msg) {
                    tracing::warn!(
                        target: "communication.loop",
                        target_loop = %target_id,
                        error = %e,
                        "Failed to deliver message to target loop inbox"
                    );
                }
            } else {
                tracing::warn!(
                    target: "communication.loop",
                    target_loop = %target_id,
                    "No inbox registered for target loop — message dropped"
                );
            }

            delivered += 1;
        }

        if delivered >= self.max_deliveries_per_tick {
            tracing::warn!(
                target: "communication.loop",
                delivered = delivered,
                max = self.max_deliveries_per_tick,
                "Hit delivery limit per tick — messages remain in queue"
            );
        }
    }
}
