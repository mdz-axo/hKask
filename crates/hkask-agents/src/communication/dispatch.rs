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
use std::sync::Arc;
use tokio::sync::Mutex;

/// In-memory priority queue for inter-loop message dispatch.
///
/// `MessageDispatch` implements the DISPATCH messenger function (6.1):
/// it guards (priority-ordered) and routes (FIFO within priority) messages
/// between the 6 loops.
///
/// Three internal queues hold messages at Critical, Warning, and Info
/// priority levels. `receive()` always dequeues from the highest-priority
/// non-empty queue, ensuring that algedonic alerts and cybernetics directives
/// are processed before routine observations.
///
/// This is an in-memory queue — it does NOT persist to SQLite (unlike
/// `EscalationQueue`). Use `tokio::sync::Mutex` for async compatibility.
pub struct MessageDispatch {
    critical: Arc<Mutex<Vec<LoopMessage>>>,
    warning: Arc<Mutex<Vec<LoopMessage>>>,
    info: Arc<Mutex<Vec<LoopMessage>>>,
}

impl MessageDispatch {
    /// Create a new empty `MessageDispatch`.
    pub fn new() -> Self {
        Self {
            critical: Arc::new(Mutex::new(Vec::new())),
            warning: Arc::new(Mutex::new(Vec::new())),
            info: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Enqueue a message and return its trace ID.
    ///
    /// The message is placed into the queue corresponding to its priority.
    pub async fn send(&self, message: LoopMessage) -> TraceId {
        let trace_id = message.trace_id;
        let queue = match message.priority {
            MessagePriority::Critical => &self.critical,
            MessagePriority::Warning => &self.warning,
            MessagePriority::Info => &self.info,
        };
        queue.lock().await.push(message);
        trace_id
    }

    /// Dequeue the highest-priority message.
    ///
    /// Returns the first message from the highest-priority non-empty queue:
    /// Critical → Warning → Info. Returns `None` if all queues are empty.
    pub async fn receive(&self) -> Option<LoopMessage> {
        if let Some(msg) = self.critical.lock().await.pop_front() {
            return Some(msg);
        }
        if let Some(msg) = self.warning.lock().await.pop_front() {
            return Some(msg);
        }
        self.info.lock().await.pop_front()
    }

    /// Total number of queued messages across all priorities.
    pub async fn len(&self) -> usize {
        let critical_len = self.critical.lock().await.len();
        let warning_len = self.warning.lock().await.len();
        let info_len = self.info.lock().await.len();
        critical_len + warning_len + info_len
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
            CuratorDirective::OverrideEnergyBudget { agent, new_budget } => (
                "override_energy_budget".to_string(),
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
        };

        let message = LoopMessage::warning(
            LoopId::Curation,
            LoopPayload::CurationDirective {
                directive_type,
                target,
                parameters,
            },
        )
        .with_sender(sender);

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

/// Helper trait for popping from the front of a `Vec`.
trait VecPopFront<T> {
    fn pop_front(&mut self) -> Option<T>;
}

impl<T> VecPopFront<T> for Vec<T> {
    fn pop_front(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            Some(self.remove(0))
        }
    }
}
