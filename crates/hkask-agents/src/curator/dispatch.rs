//! Message Dispatch — Priority-ordered inter-loop message queuing
//!
//! Implements DISPATCH (messenger function 6.1: GUARD+ROUTE) from the 7-loop
//! architecture. `MessageDispatch` provides an in-memory priority queue that
//! orders `LoopMessage` instances for inter-loop communication.
//!
//! Priority ordering: Critical → Warning → Info.
//! Within the same priority, messages are dequeued in FIFO order.

use hkask_types::WebID;
use hkask_types::loops::curation::CuratorDirective;
use hkask_types::loops::dispatch::{
    LoopMessage, LoopOrigin, LoopPayload, MessagePriority, TraceId,
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// In-memory priority queue for inter-loop message dispatch.
///
/// `MessageDispatch` implements the DISPATCH messenger function (6.1):
/// it guards (priority-ordered) and routes (FIFO within priority) messages
/// between the 7 loops.
///
/// Three internal queues hold messages at Critical, Warning, and Info
/// priority levels. `receive()` always dequeues from the highest-priority
/// non-empty queue, ensuring that algedonic alerts and governance directives
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
    /// Creates a `LoopPayload::GovernanceDirective` with `LoopOrigin::Curation`
    /// and `MessagePriority::Warning` (governance directives are warnings by
    /// default; use `send()` directly for a different priority).
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
            CuratorDirective::AdjustEnergyBudget { agent, new_budget } => (
                "adjust_energy_budget".to_string(),
                *agent,
                serde_json::json!({
                    "new_budget": new_budget,
                }),
            ),
        };

        let message = LoopMessage::warning(
            LoopOrigin::Curation,
            LoopPayload::GovernanceDirective {
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
    /// Creates a message with `MessagePriority::Critical` and `LoopOrigin::Observability`,
    /// which is the standard pattern for algedonic alerts (variety deficit escalation).
    pub async fn send_escalation(&self, alert: LoopPayload, sender: WebID) -> TraceId {
        let message = LoopMessage::critical(LoopOrigin::Observability, alert).with_sender(sender);
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

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::loops::dispatch::{LoopOrigin, LoopPayload, MessagePriority};

    fn make_info_message(origin: LoopOrigin, payload: LoopPayload) -> LoopMessage {
        LoopMessage::info(origin, payload)
    }

    fn make_warning_message(origin: LoopOrigin, payload: LoopPayload) -> LoopMessage {
        LoopMessage::warning(origin, payload)
    }

    fn make_critical_message(origin: LoopOrigin, payload: LoopPayload) -> LoopMessage {
        LoopMessage::critical(origin, payload)
    }

    fn sample_payload(label: &str) -> LoopPayload {
        LoopPayload::Observation {
            category: label.to_string(),
            data: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn priority_ordering_critical_before_warning_before_info() {
        let dispatch = MessageDispatch::new();

        // Enqueue in reverse priority order
        let info_msg = make_info_message(LoopOrigin::Inference, sample_payload("info"));
        let warning_msg = make_warning_message(LoopOrigin::Governance, sample_payload("warning"));
        let critical_msg =
            make_critical_message(LoopOrigin::Observability, sample_payload("critical"));

        dispatch.send(info_msg.clone()).await;
        dispatch.send(warning_msg.clone()).await;
        dispatch.send(critical_msg.clone()).await;

        // Critical dequeued first
        let received = dispatch.receive().await.unwrap();
        assert_eq!(received.priority, MessagePriority::Critical);
        assert_eq!(received.trace_id, critical_msg.trace_id);

        // Then Warning
        let received = dispatch.receive().await.unwrap();
        assert_eq!(received.priority, MessagePriority::Warning);
        assert_eq!(received.trace_id, warning_msg.trace_id);

        // Then Info
        let received = dispatch.receive().await.unwrap();
        assert_eq!(received.priority, MessagePriority::Info);
        assert_eq!(received.trace_id, info_msg.trace_id);
    }

    #[tokio::test]
    async fn fifo_within_same_priority() {
        let dispatch = MessageDispatch::new();

        let msg1 = make_warning_message(LoopOrigin::Governance, sample_payload("first"));
        let msg2 = make_warning_message(LoopOrigin::Curation, sample_payload("second"));
        let msg3 = make_warning_message(LoopOrigin::External, sample_payload("third"));

        let id1 = dispatch.send(msg1).await;
        let id2 = dispatch.send(msg2).await;
        let id3 = dispatch.send(msg3).await;

        // FIFO: first in, first out within Warning priority
        let received = dispatch.receive().await.unwrap();
        assert_eq!(received.trace_id, id1);
        let received = dispatch.receive().await.unwrap();
        assert_eq!(received.trace_id, id2);
        let received = dispatch.receive().await.unwrap();
        assert_eq!(received.trace_id, id3);
    }

    #[tokio::test]
    async fn send_returns_trace_id() {
        let dispatch = MessageDispatch::new();

        let msg = make_info_message(LoopOrigin::Inference, sample_payload("test"));
        let expected_id = msg.trace_id;
        let returned_id = dispatch.send(msg).await;

        assert_eq!(returned_id, expected_id);
    }

    #[tokio::test]
    async fn send_curator_directive_creates_proper_loop_message() {
        let dispatch = MessageDispatch::new();

        let agent = WebID::new();
        let directive = CuratorDirective::CalibrateThreshold {
            domain: "variety".to_string(),
            new_threshold: 100,
        };

        let trace_id = dispatch
            .send_curator_directive(directive.clone(), agent)
            .await;

        let msg = dispatch.receive().await.unwrap();

        // Should be Warning priority (default for curator directives)
        assert_eq!(msg.priority, MessagePriority::Warning);
        // Should originate from Curation
        assert_eq!(msg.origin, LoopOrigin::Curation);
        // Should have the sender set
        assert_eq!(msg.sender, Some(agent));
        // Should be a GovernanceDirective payload
        match &msg.payload {
            LoopPayload::GovernanceDirective {
                directive_type,
                target: _,
                parameters,
            } => {
                assert_eq!(directive_type, "calibrate_threshold");
                assert_eq!(parameters["domain"], "variety");
                assert_eq!(parameters["new_threshold"], 100);
            }
            _ => panic!(
                "Expected GovernanceDirective payload, got {:?}",
                msg.payload
            ),
        }

        // Trace ID should match what was returned
        assert_eq!(msg.trace_id, trace_id);
    }

    #[tokio::test]
    async fn send_curator_directive_update_capabilities() {
        let dispatch = MessageDispatch::new();

        let agent = WebID::new();
        let directive = CuratorDirective::UpdateCapabilities {
            agent,
            additions: vec!["tool_a".to_string()],
            removals: vec!["tool_b".to_string()],
        };

        dispatch.send_curator_directive(directive, agent).await;

        let msg = dispatch.receive().await.unwrap();
        match &msg.payload {
            LoopPayload::GovernanceDirective {
                directive_type,
                target,
                ..
            } => {
                assert_eq!(directive_type, "update_capabilities");
                assert_eq!(*target, agent);
            }
            _ => panic!("Expected GovernanceDirective payload"),
        }
    }

    #[tokio::test]
    async fn send_curator_directive_adjust_energy_budget() {
        let dispatch = MessageDispatch::new();

        let agent = WebID::new();
        let directive = CuratorDirective::AdjustEnergyBudget {
            agent,
            new_budget: 5000,
        };

        dispatch.send_curator_directive(directive, agent).await;

        let msg = dispatch.receive().await.unwrap();
        match &msg.payload {
            LoopPayload::GovernanceDirective {
                directive_type,
                target,
                ..
            } => {
                assert_eq!(directive_type, "adjust_energy_budget");
                assert_eq!(*target, agent);
            }
            _ => panic!("Expected GovernanceDirective payload"),
        }
    }

    #[tokio::test]
    async fn send_escalation_creates_critical_priority_message() {
        let dispatch = MessageDispatch::new();

        let sender = WebID::new();
        let alert = LoopPayload::AlgedonicAlert {
            current: 10,
            threshold: 100,
            deficit: 90,
        };

        let trace_id = dispatch.send_escalation(alert.clone(), sender).await;

        let msg = dispatch.receive().await.unwrap();

        // Should be Critical priority
        assert_eq!(msg.priority, MessagePriority::Critical);
        // Should originate from Observability (standard for algedonic alerts)
        assert_eq!(msg.origin, LoopOrigin::Observability);
        // Should have the sender set
        assert_eq!(msg.sender, Some(sender));
        // Should carry the algedonic alert payload
        match &msg.payload {
            LoopPayload::AlgedonicAlert {
                current,
                threshold,
                deficit,
            } => {
                assert_eq!(*current, 10);
                assert_eq!(*threshold, 100);
                assert_eq!(*deficit, 90);
            }
            _ => panic!("Expected AlgedonicAlert payload, got {:?}", msg.payload),
        }

        // Trace ID should match what was returned
        assert_eq!(msg.trace_id, trace_id);
    }

    #[tokio::test]
    async fn empty_dispatch_returns_none_on_receive() {
        let dispatch = MessageDispatch::new();

        assert!(dispatch.is_empty().await);
        assert_eq!(dispatch.len().await, 0);
        assert!(dispatch.receive().await.is_none());
    }

    #[tokio::test]
    async fn len_tracks_total_across_priorities() {
        let dispatch = MessageDispatch::new();

        dispatch
            .send(make_critical_message(
                LoopOrigin::Observability,
                sample_payload("c1"),
            ))
            .await;
        dispatch
            .send(make_warning_message(
                LoopOrigin::Governance,
                sample_payload("w1"),
            ))
            .await;
        dispatch
            .send(make_info_message(
                LoopOrigin::Inference,
                sample_payload("i1"),
            ))
            .await;
        dispatch
            .send(make_info_message(
                LoopOrigin::Inference,
                sample_payload("i2"),
            ))
            .await;

        assert_eq!(dispatch.len().await, 4);
        assert!(!dispatch.is_empty().await);

        dispatch.receive().await; // critical
        assert_eq!(dispatch.len().await, 3);

        dispatch.receive().await; // warning
        dispatch.receive().await; // info
        assert_eq!(dispatch.len().await, 1);
    }

    #[tokio::test]
    async fn default_trait_creates_empty_dispatch() {
        let dispatch = MessageDispatch::default();
        assert!(dispatch.is_empty().await);
    }
}

#[cfg(test)]
mod cyber_tests {
    use super::*;
    use hkask_types::loops::dispatch::{LoopOrigin, LoopPayload, MessagePriority};

    /// PR 9h, Loop 6.1: Dispatch priority ordering — Critical before Warning before Info.
    ///
    /// Proves: MessageDispatch enforces priority ordering regardless of
    /// insertion order, ensuring algedonic alerts are processed before routine observations.
    #[tokio::test]
    async fn cyber_dispatch_priority_ordering() {
        let dispatch = MessageDispatch::new();

        // Send in reverse priority order: Warning, then Critical, then Info
        let warning_msg = LoopMessage::warning(
            LoopOrigin::Governance,
            LoopPayload::Observation {
                category: "warning".to_string(),
                data: serde_json::json!({}),
            },
        );
        let critical_msg = LoopMessage::critical(
            LoopOrigin::Observability,
            LoopPayload::Observation {
                category: "critical".to_string(),
                data: serde_json::json!({}),
            },
        );
        let info_msg = LoopMessage::info(
            LoopOrigin::Inference,
            LoopPayload::Observation {
                category: "info".to_string(),
                data: serde_json::json!({}),
            },
        );

        dispatch.send(warning_msg.clone()).await;
        dispatch.send(critical_msg.clone()).await;
        dispatch.send(info_msg.clone()).await;

        // Dequeue: Critical first, then Warning, then Info
        let received = dispatch.receive().await.unwrap();
        assert_eq!(
            received.priority,
            MessagePriority::Critical,
            "Critical should be dequeued first"
        );
        assert_eq!(received.trace_id, critical_msg.trace_id);

        let received = dispatch.receive().await.unwrap();
        assert_eq!(
            received.priority,
            MessagePriority::Warning,
            "Warning should be dequeued second"
        );
        assert_eq!(received.trace_id, warning_msg.trace_id);

        let received = dispatch.receive().await.unwrap();
        assert_eq!(
            received.priority,
            MessagePriority::Info,
            "Info should be dequeued last"
        );
        assert_eq!(received.trace_id, info_msg.trace_id);
    }

    /// PR 9h, Loop 6.2: Trace ID propagation — CORRELATE (SENSE) subloop.
    ///
    /// Proves: LoopMessage carries a trace_id that is preserved through
    /// send and receive, enabling cross-loop correlation.
    #[tokio::test]
    async fn cyber_trace_id_propagation() {
        let dispatch = MessageDispatch::new();

        // Create a message and capture its trace_id
        let msg = LoopMessage::warning(
            LoopOrigin::Curation,
            LoopPayload::Observation {
                category: "test".to_string(),
                data: serde_json::json!({"key": "value"}),
            },
        );
        let original_trace_id = msg.trace_id;

        // Send through dispatch
        let returned_trace_id = dispatch.send(msg).await;
        assert_eq!(
            original_trace_id, returned_trace_id,
            "send should return the same trace_id"
        );

        // Receive from dispatch
        let received = dispatch.receive().await.unwrap();
        assert_eq!(
            received.trace_id, original_trace_id,
            "trace_id should be preserved through send/receive"
        );

        // Trace ID is a proper UUID
        assert_ne!(
            received.trace_id.0,
            uuid::Uuid::nil(),
            "trace_id should be a real UUID"
        );
    }
}
