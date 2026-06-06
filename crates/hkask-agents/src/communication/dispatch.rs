//! Message Dispatch — Priority-ordered inter-loop message queuing
//!
//! Implements DISPATCH (messenger function 4.1: GUARD+ROUTE) from the 6-loop
//! architecture. `MessageDispatch` provides an in-memory priority queue that
//! orders `LoopMessage` instances for inter-loop communication.
//!
//! Priority ordering: Critical → Warning → Info.
//! Within the same priority, messages are dequeued in FIFO order.

use hkask_types::WebID;
use hkask_types::loops::LoopId;
use hkask_types::loops::curation::CuratorDirective;
use hkask_types::loops::dispatch::{LoopMessage, LoopPayload, MessagePriority, TraceId};
use std::collections::VecDeque;
use std::collections::hash_map::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// In-memory priority queue for inter-loop message dispatch.
///
/// `MessageDispatch` implements the DISPATCH messenger function (6.1):
/// it guards (priority-ordered) and routes (FIFO within priority) messages
/// between the 6 loops.
///
/// Internally a single `HashMap<MessagePriority, VecDeque<LoopMessage>>`
/// replaces the former 3 separate `Arc<Mutex<Vec<...>>>` fields. This
/// eliminates per-priority lock contention — one lock covers all queues —
/// and lets `receive()` drain in strict priority order without acquiring
/// three separate locks.
///
/// This is an in-memory queue — it does NOT persist to SQLite (unlike
/// `EscalationQueue`). Uses `tokio::sync::Mutex` for async compatibility.
pub struct MessageDispatch {
    queues: Arc<Mutex<HashMap<MessagePriority, VecDeque<LoopMessage>>>>,
}

impl MessageDispatch {
    /// Create a new empty `MessageDispatch`.
    pub fn new() -> Self {
        let mut queues = HashMap::new();
        queues.insert(MessagePriority::Critical, VecDeque::new());
        queues.insert(MessagePriority::Warning, VecDeque::new());
        queues.insert(MessagePriority::Info, VecDeque::new());
        Self {
            queues: Arc::new(Mutex::new(queues)),
        }
    }

    /// Enqueue a message and return its trace ID.
    ///
    /// The message is placed into the queue corresponding to its priority.
    pub async fn send(&self, message: LoopMessage) -> TraceId {
        let trace_id = message.trace_id;
        let priority = message.priority;
        self.queues
            .lock()
            .await
            .get_mut(&priority)
            .expect("dispatch queue initialized with all priorities")
            .push_back(message);
        trace_id
    }

    /// Dequeue the highest-priority message.
    ///
    /// Returns the first message from the highest-priority non-empty queue:
    /// Critical → Warning → Info. Returns `None` if all queues are empty.
    pub async fn receive(&self) -> Option<LoopMessage> {
        let mut queues = self.queues.lock().await;
        for priority in [
            MessagePriority::Critical,
            MessagePriority::Warning,
            MessagePriority::Info,
        ] {
            if let Some(queue) = queues.get_mut(&priority)
                && let Some(msg) = queue.pop_front()
            {
                return Some(msg);
            }
        }
        None
    }

    /// Total number of queued messages across all priorities.
    pub async fn len(&self) -> usize {
        self.queues.lock().await.values().map(|q| q.len()).sum()
    }

    /// Whether all queues are empty.
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// Convenience method: wrap a `CuratorDirective` as a `LoopMessage` and enqueue it.
    ///
    /// Creates a `LoopPayload::CurationDirective` with `LoopId::Curation`
    /// and `MessagePriority::Warning` (curation directives are warnings by
    /// default; use `send()` directly for a different priority).
    ///
    /// Per the authority DAG: Curation → Cybernetics. The origin is Curation
    /// and the payload carries a Curation-originated directive.
    pub async fn send_curator_directive(
        &self,
        directive: CuratorDirective,
        sender: WebID,
    ) -> TraceId {
        let (directive_type, target, parameters) = match &directive {
            CuratorDirective::CalibrateThreshold {
                domain,
                new_threshold,
            } => (
                "calibrate_threshold".to_string(),
                WebID::new(), // no specific target for threshold calibration
                serde_json::json!({
                    "domain": domain,
                    "new_threshold": new_threshold,
                }),
            ),
            CuratorDirective::UpdateCapabilities {
                agent,
                additions,
                removals,
            } => (
                "update_capabilities".to_string(),
                *agent,
                serde_json::json!({
                    "additions": additions,
                    "removals": removals,
                }),
            ),
            CuratorDirective::OverrideGasBudget { agent, new_budget } => (
                "override_gas_budget".to_string(),
                *agent,
                serde_json::json!({
                    "new_budget": new_budget,
                }),
            ),
            CuratorDirective::SeekMoreEvidence {
                context,
                channel,
                confidence,
            } => (
                "seek_more_evidence".to_string(),
                WebID::new(),
                serde_json::json!({
                    "context": context,
                    "channel": channel,
                    "confidence": confidence,
                }),
            ),
            CuratorDirective::ReplenishBudget {
                agent,
                amount,
                priority,
            } => {
                let mut params = serde_json::json!({
                    "amount": amount,
                });
                if let Some(p) = priority {
                    params["priority"] = serde_json::json!(p);
                }
                ("replenish_budget".to_string(), *agent, params)
            }
        };

        let message = LoopMessage::warning(
            LoopId::Curation,
            LoopPayload::CurationDirective {
                directive_type,
                target,
                parameters,
            },
        )
        .with_sender(sender)
        .with_target(LoopId::Cybernetics);

        self.send(message).await
    }

    /// Convenience method: enqueue an algedonic alert as a Critical-priority message.
    ///
    /// Creates a message with `MessagePriority::Critical` and `LoopId::Cybernetics`,
    /// which is the standard pattern for algedonic alerts (variety deficit escalation).
    pub async fn send_escalation(&self, alert: LoopPayload, sender: WebID) -> TraceId {
        let message = LoopMessage::critical(LoopId::Cybernetics, alert).with_sender(sender);
        self.send(message).await
    }
}

impl Default for MessageDispatch {
    fn default() -> Self {
        Self::new()
    }
}
