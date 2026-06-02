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
use hkask_types::loops::dispatch::{LoopMessage, LoopOrigin};
use hkask_types::loops::{Deviation, HkaskLoop, LoopAction, LoopId, Signal};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Communication Loop — routes inter-loop messages.
///
/// Wraps `MessageDispatch` (the priority queue) and receives
/// `LoopMessage`s from the dispatch channel. The loop's `tick()`
/// dequeues messages and delivers them to target loop inboxes.
pub struct CommunicationLoop {
    /// Priority-ordered message queue
    dispatch: Arc<MessageDispatch>,
    /// Per-loop inbox senders for message delivery
    loop_senders: Arc<
        RwLock<std::collections::HashMap<LoopId, tokio::sync::mpsc::UnboundedSender<LoopMessage>>>,
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
            max_deliveries_per_tick: 64,
        }
    }

    /// Create a Communication Loop with custom delivery limit.
    pub fn with_max_deliveries(dispatch: Arc<MessageDispatch>, max: usize) -> Self {
        Self {
            dispatch,
            loop_senders: Arc::new(RwLock::new(std::collections::HashMap::new())),
            max_deliveries_per_tick: max,
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

    /// Get the number of pending messages across all priority queues.
    pub async fn pending_count(&self) -> usize {
        self.dispatch.len().await
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
                Some(origin) => match origin {
                    LoopOrigin::Inference => LoopId::Inference,
                    LoopOrigin::Episodic => LoopId::Episodic,
                    LoopOrigin::Semantic => LoopId::Semantic,
                    LoopOrigin::Communication => LoopId::Communication,
                    LoopOrigin::Curation => LoopId::Curation,
                    LoopOrigin::Cybernetics => LoopId::Cybernetics,
                    LoopOrigin::External => {
                        tracing::debug!(
                            target: "communication.loop",
                            trace_id = %msg.trace_id,
                            "Dropping message to External origin (no inbox)"
                        );
                        delivered += 1;
                        continue;
                    }
                },
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
