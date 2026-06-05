//! Loop-routed tool dispatch — Tool invocation through the Communication Loop
//!
//! When loop-routed tool dispatch is enabled, tool invocations flow through
//! the Communication Loop instead of direct `ToolPort::invoke()` calls.
//! This provides:
//!
//! - **Cross-loop traceability**: Every invocation carries a `TraceId`
//! - **Priority-aware ordering**: Critical tools (circuit-break resets) can
//!   be prioritized over routine info queries
//! - **Delivery confirmation**: The Communication Loop confirms delivery
//!
//! The flow is:
//!
//! ```text
//! GovernedTool → LoopMessage(ToolInvocation) → Communication Loop →
//!   LoopRoutedToolDispatch (tool worker) → ToolPort::invoke() →
//!   LoopMessage(ToolResult) → Communication Loop → original caller
//! ```
//!
//! This module implements the tool worker side: it receives `ToolInvocation`
//! messages from the Communication Loop inbox, executes them against the
//! inner `ToolPort`, and sends `ToolResult` messages back through the dispatch.

use hkask_types::capability::DelegationToken;
use hkask_types::loops::dispatch::{LoopMessage, LoopPayload, MessagePriority, WorkerKind};
use hkask_types::loops::{Deviation, HkaskLoop, LoopAction, LoopId, Signal};
use hkask_types::ports::ToolPort;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tool dispatch worker that receives `ToolInvocation` messages from the
/// Communication Loop and executes them against the inner `ToolPort`.
///
/// This is NOT a full loop — it doesn't produce regulatory actions. It
/// operates as a specialized message handler within the Communication Loop's
/// delivery pipeline. The `HkaskLoop` implementation is minimal: `sense()`
/// reports dispatch queue depth, `act()` processes pending invocations.
///
/// To use loop-routed dispatch, register this with the `LoopSystem` and
/// route `ToolInvocation` messages to `WorkerKind::ToolDispatch`.
pub struct LoopRoutedToolDispatch {
    /// Inner tool port (typically `GovernedTool` wrapping `RawMcpToolPort`)
    inner: Arc<dyn ToolPort>,
    /// System-level delegation token for inner invoke calls.
    /// OCAP verification was already performed by GovernedTool before the
    /// invocation entered the Communication Loop. This token authorizes
    /// the pass-through execution.
    system_token: DelegationToken,
    /// Dispatch sender for returning `ToolResult` messages
    dispatch_tx: tokio::sync::mpsc::UnboundedSender<LoopMessage>,
    /// Inbox for receiving `ToolInvocation` messages from the Communication Loop
    inbox: Arc<RwLock<tokio::sync::mpsc::UnboundedReceiver<LoopMessage>>>,
    /// Maximum invocations per tick cycle
    max_invocations_per_tick: usize,
}

impl LoopRoutedToolDispatch {
    /// Create a new loop-routed tool dispatch worker.
    ///
    /// Returns `(dispatch_instance, inbox_sender)`. Register the sender with
    /// the Communication Loop so `ToolInvocation` messages targeted at
    /// `WorkerKind::ToolDispatch` are delivered here.
    ///
    /// The `system_token` is used for inner `ToolPort::invoke()` calls. OCAP
    /// verification was already performed by GovernedTool before the invocation
    /// entered the Communication Loop, so this token serves as a pass-through
    /// authorization.
    pub fn new(
        inner: Arc<dyn ToolPort>,
        system_token: DelegationToken,
        dispatch_tx: tokio::sync::mpsc::UnboundedSender<LoopMessage>,
    ) -> (Self, tokio::sync::mpsc::UnboundedSender<LoopMessage>) {
        let (inbox_tx, inbox_rx) = tokio::sync::mpsc::unbounded_channel::<LoopMessage>();
        let instance = Self {
            inner,
            system_token,
            dispatch_tx,
            inbox: Arc::new(RwLock::new(inbox_rx)),
            max_invocations_per_tick: 16,
        };
        (instance, inbox_tx)
    }

    /// Set the maximum number of invocations per tick cycle.
    pub fn with_max_invocations_per_tick(mut self, max: usize) -> Self {
        self.max_invocations_per_tick = max;
        self
    }

    /// Process pending `ToolInvocation` messages from the inbox.
    ///
    /// For each invocation, call the inner `ToolPort` and send a `ToolResult`
    /// message back through the dispatch channel. Returns the number of
    /// invocations processed.
    pub async fn process_invocations(&self) -> usize {
        let mut inbox = self.inbox.write().await;
        let mut processed = 0;

        while processed < self.max_invocations_per_tick {
            let msg = match inbox.try_recv() {
                Ok(m) => m,
                Err(_) => break,
            };

            match msg.payload {
                LoopPayload::ToolInvocation {
                    trace_id,
                    server,
                    tool,
                    args,
                    agent,
                } => {
                    // Execute the tool invocation using the system pass-through token.
                    // OCAP verification was already performed by GovernedTool before
                    // the invocation entered the Communication Loop.
                    let tool_name = tool.clone();
                    let result = self
                        .inner
                        .invoke(&server, &tool, args, &self.system_token)
                        .await;

                    let (result_value, success) = match result {
                        Ok(v) => (v, true),
                        Err(e) => (serde_json::json!({ "error": e.to_string() }), false),
                    };

                    // Send ToolResult back through the dispatch channel
                    let result_msg = LoopMessage::new(
                        if success {
                            MessagePriority::Info
                        } else {
                            MessagePriority::Warning
                        },
                        LoopId::Communication,
                        LoopPayload::ToolResult {
                            trace_id,
                            server,
                            tool,
                            result: result_value,
                            success,
                            gas_cost: 0, // Gas is settled by GovernedTool
                            agent,
                        },
                    )
                    .with_target(msg.origin);

                    if let Err(e) = self.dispatch_tx.send(result_msg) {
                        tracing::warn!(
                            target: "tool_dispatch",
                            trace_id = %trace_id,
                            tool = %tool_name,
                            error = %e,
                            "Failed to dispatch ToolResult — Communication Loop may be closed"
                        );
                    }

                    tracing::debug!(
                        target: "tool_dispatch",
                        trace_id = %trace_id,
                        tool = %tool_name,
                        success,
                        "Processed loop-routed tool invocation"
                    );

                    processed += 1;
                }
                other => {
                    // Not a ToolInvocation — log and skip
                    tracing::debug!(
                        target: "tool_dispatch",
                        payload_type = ?other,
                        "Received non-invocation payload in tool dispatch inbox — skipping"
                    );
                }
            }
        }

        processed
    }
}

#[async_trait::async_trait]
impl HkaskLoop for LoopRoutedToolDispatch {
    fn id(&self) -> LoopId {
        // Tool dispatch is a worker within Communication (Loop 4), not a governing loop.
        LoopId::Communication
    }

    /// Tool dispatch is a worker within the Communication loop.
    fn worker_kind(&self) -> Option<WorkerKind> {
        Some(WorkerKind::ToolDispatch)
    }

    /// Sense: report dispatch queue depth.
    async fn sense(&self) -> Vec<Signal> {
        let _queue_depth = {
            let _inbox = self.inbox.read().await;
            // Approximate queue depth — can't easily count unbounded channel depth
            0
        };

        vec![Signal::new(
            LoopId::Communication,
            "tool_dispatch_queue_depth",
            _queue_depth as f64,
            10.0, // Set-point: 10 pending invocations before considered overloaded
        )]
    }

    /// Compute: no regulatory actions (tool dispatch is not a governing loop).
    async fn compute(&self, _deviations: &[Deviation]) -> Vec<LoopAction> {
        Vec::new()
    }

    /// Act: process pending tool invocations from the inbox.
    async fn act(&self, _actions: &[LoopAction]) {
        let processed = self.process_invocations().await;
        if processed > 0 {
            tracing::debug!(
                target: "tool_dispatch",
                processed = processed,
                "Tool dispatch worker processed invocations"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::capability::{DelegationAction, DelegationResource, DelegationTokenBuilder};
    use hkask_types::id::WebID;
    use hkask_types::loops::dispatch::TraceId;
    use hkask_types::ports::ToolPortError;

    /// Mock tool port that returns a fixed result.
    struct MockToolPort {
        response: serde_json::Value,
    }

    impl MockToolPort {
        fn new(response: serde_json::Value) -> Self {
            Self { response }
        }
    }

    #[async_trait::async_trait]
    impl ToolPort for MockToolPort {
        async fn invoke(
            &self,
            _server: &str,
            tool: &str,
            _args: serde_json::Value,
            _token: &DelegationToken,
        ) -> Result<serde_json::Value, ToolPortError> {
            if tool == "fail" {
                Err(ToolPortError::InvocationFailed("mock failure".to_string()))
            } else {
                Ok(self.response.clone())
            }
        }

        async fn discover_tools(&self) -> Vec<String> {
            vec!["test_tool".to_string()]
        }

        async fn get_tool_info(&self, tool_name: &str) -> Option<hkask_types::ports::ToolInfo> {
            Some(hkask_types::ports::ToolInfo {
                name: tool_name.to_string(),
                description: "mock tool".to_string(),
                input_schema: serde_json::json!({}),
                server_id: "mock".to_string(),
                required_capability: None,
            })
        }
    }

    fn test_system_token() -> DelegationToken {
        DelegationTokenBuilder::new(
            DelegationResource::Tool,
            "test_server".to_string(),
            DelegationAction::Execute,
            WebID::new(),
            WebID::new(),
        )
        .sign(b"test_secret")
    }

    fn test_dispatch_tx() -> tokio::sync::mpsc::UnboundedSender<LoopMessage> {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        tx
    }

    #[tokio::test]
    async fn tool_dispatch_id_is_communication_worker_kind_is_tool_dispatch() {
        let mock_port = Arc::new(MockToolPort::new(serde_json::json!({"ok": true})));
        let (dispatch, _inbox_tx) =
            LoopRoutedToolDispatch::new(mock_port, test_system_token(), test_dispatch_tx());
        assert_eq!(dispatch.id(), LoopId::Communication);
        assert_eq!(dispatch.worker_kind(), Some(WorkerKind::ToolDispatch));
    }

    #[tokio::test]
    async fn tool_dispatch_processes_invocation() {
        let mock_port = Arc::new(MockToolPort::new(serde_json::json!({"result": 42})));
        let (dispatch_tx, mut dispatch_rx) = tokio::sync::mpsc::unbounded_channel::<LoopMessage>();
        let (dispatch, inbox_tx) =
            LoopRoutedToolDispatch::new(mock_port, test_system_token(), dispatch_tx.clone());

        // Send a ToolInvocation message to the dispatch's inbox
        let invocation = LoopMessage::new(
            MessagePriority::Info,
            LoopId::Inference,
            LoopPayload::ToolInvocation {
                trace_id: TraceId::new(),
                server: "test_server".to_string(),
                tool: "test_tool".to_string(),
                args: serde_json::json!({}),
                agent: WebID::new(),
            },
        )
        .with_target(WorkerKind::ToolDispatch);

        inbox_tx.send(invocation).expect("send should succeed");

        // Process invocations
        let processed = dispatch.process_invocations().await;
        assert_eq!(processed, 1);

        // Check that a ToolResult was dispatched
        let result_msg = dispatch_rx.try_recv().expect("should receive ToolResult");
        match result_msg.payload {
            LoopPayload::ToolResult { tool, success, .. } => {
                assert_eq!(tool, "test_tool");
                assert!(success);
            }
            other => panic!("Expected ToolResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn tool_dispatch_handles_failure() {
        let mock_port = Arc::new(MockToolPort::new(serde_json::json!({"ok": true})));
        let (dispatch_tx, mut dispatch_rx) = tokio::sync::mpsc::unbounded_channel::<LoopMessage>();
        let (dispatch, inbox_tx) =
            LoopRoutedToolDispatch::new(mock_port, test_system_token(), dispatch_tx.clone());

        let invocation = LoopMessage::new(
            MessagePriority::Warning,
            LoopId::Inference,
            LoopPayload::ToolInvocation {
                trace_id: TraceId::new(),
                server: "test_server".to_string(),
                tool: "fail".to_string(),
                args: serde_json::json!({}),
                agent: WebID::new(),
            },
        )
        .with_target(WorkerKind::ToolDispatch);

        inbox_tx.send(invocation).expect("send should succeed");

        let processed = dispatch.process_invocations().await;
        assert_eq!(processed, 1);

        let result_msg = dispatch_rx.try_recv().expect("should receive ToolResult");
        match result_msg.payload {
            LoopPayload::ToolResult {
                tool,
                success,
                result,
                ..
            } => {
                assert_eq!(tool, "fail");
                assert!(!success);
                assert!(result.get("error").is_some());
            }
            other => panic!("Expected ToolResult, got {:?}", other),
        }
    }
}
