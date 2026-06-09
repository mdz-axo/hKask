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
use hkask_types::cns::QueueDepth;
use hkask_types::loops::dispatch::LoopMessage;
use hkask_types::loops::{Deviation, HkaskLoop, LoopAction, LoopId, Signal, SignalMetric};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
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
    /// Max messages delivered per tick (backpressure limit)
    max_deliveries_per_tick: usize,
    /// Lock-free counter read by CyberneticsLoop for backpressure sensing
    queue_depth_counter: Arc<AtomicU64>,
}

impl CommunicationLoop {
    /// Create a new Communication Loop with an empty dispatch and no target loops.
    pub fn new(dispatch: Arc<MessageDispatch>) -> Self {
        Self {
            dispatch,
            loop_senders: Arc::new(RwLock::new(std::collections::HashMap::new())),
            max_deliveries_per_tick: 64,
            queue_depth_counter: Arc::new(AtomicU64::new(0)),
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

    /// Get a clone of the shared queue depth counter.
    ///
    /// CyberneticsLoop reads this counter to sense communication backpressure
    /// without needing a direct reference to MessageDispatch.
    pub fn queue_depth_counter(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.queue_depth_counter)
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
        self.queue_depth_counter
            .store(queue_depth as u64, Ordering::Relaxed);
        let senders = self.loop_senders.read().await;
        let registered_loops = senders.len();

        // Produce signals about dispatch health
        vec![
            Signal::new(
                LoopId::Communication,
                SignalMetric::QueueDepth,
                queue_depth as f64,
                QueueDepth::DEFAULT_BACKPRESSURE.as_raw(),
            ),
            Signal::new(
                LoopId::Communication,
                SignalMetric::RegisteredLoops,
                registered_loops as f64,
                7.0, // set-point: all 7 loops should be registered (Inference, Episodic, Semantic, Communication, Curation, Cybernetics, Snapshot)
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
                Some(id) => id,
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
