//! GovernedTool — Capability-gated, gas-accounted, observability-emitting membrane
//!
//! Wraps a `ToolPort` and implements `ToolPort` itself. Before delegating
//! to the inner tool, it checks:
//! 1. Authority (OCAP) — DelegationToken verification
//! 2. Budget (Cybernetics) — reserve gas
//! 3. Emits span (CNS) — cns.tool.invoked
//! 4. Delegates to inner tool
//! 5. Settles gas cost (Cybernetics) — settle actual vs. reserved
//! 6. Emits outcome span (CNS) — cns.tool.completed
//!
//! This is the membrane where Cybernetics governs all tool invocations.
//! The membrane IS the security property (Miller). GovernedTool subsumes:
//! - SecurityGateway dispatch-time OCAP checks
//! - Gas accounting (replaces the former ThrottleBucket rate limiting)
//! - ToolSpanGuard (span emission is now built-in)
//!
//! Hold-settle pattern: gas is reserved before invocation, then settled after.
//! If actual cost < reserved, the difference is refunded. This prevents
//! gas leaks from over-estimation.

use crate::cybernetics_loop::CyberneticsLoop;
use hkask_types::NuEventSink;
use hkask_types::WebID;
use hkask_types::capability::{DelegationAction, DelegationResource, DelegationToken};
use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};
use hkask_types::loops::{LoopId, LoopMessage, LoopPayload, MessagePriority};
use hkask_types::ports::{ToolInfo, ToolPort, ToolPortError};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info, warn};

/// Gas estimator trait for GovernedTool.
///
/// Gas is a dimensionless cost unit analogous to Ethereum gas: it prevents
/// infinite loops by making resource exhaustion explicit. Different tool
/// categories have different cost models:
/// - Inference: estimated by token count (InferenceGasEstimator)
/// - All other tools: flat costs from TableGasEstimator
///
/// Use `TableGasEstimator` for production (per-server gas costs) or
/// `InferenceGasEstimator` for inference-specific token-based estimation.
pub trait GasEstimator: Send + Sync {
    /// Estimate the gas cost of a tool invocation before it happens.
    fn estimate_cost(&self, server: &str, tool: &str, args: &Value) -> u64;
}

/// GovernedTool — the singular membrane through which all tool invocations pass.
///
/// This struct wraps a `ToolPort` and enforces OCAP authority, gas
/// budgets, and CNS observability. It implements `ToolPort` itself — the
/// membrane IS a ToolPort (Miller's membrane object pattern).
///
/// Hold-settle pattern: gas is reserved before invocation, then settled
/// after with actual cost. If actual cost < reserved, the difference is
/// refunded to the budget.
///
/// # Composition
///
/// ```ignore
/// let inner: Arc<RawMcpToolPort> = Arc::new(RawMcpToolPort::new(runtime));
/// let governed = GovernedTool::new(
///     inner,
///     cybernetics_loop,
///     event_sink,
///     Arc::new(TableGasEstimator::new()),
///     agent_webid,
///     dispatch_tx,
/// );
/// // governed implements ToolPort — use it anywhere ToolPort is expected
/// ```
pub struct GovernedTool<P: ToolPort> {
    inner: Arc<P>,
    cybernetics: Arc<RwLock<CyberneticsLoop>>,
    event_sink: Arc<dyn NuEventSink>,
    estimator: Arc<dyn GasEstimator>,
    agent: WebID,
    dispatch_tx: mpsc::UnboundedSender<LoopMessage>,
}

impl<P: ToolPort> GovernedTool<P> {
    /// Create a new GovernedTool membrane wrapping an inner ToolPort.
    pub fn new(
        inner: Arc<P>,
        cybernetics: Arc<RwLock<CyberneticsLoop>>,
        event_sink: Arc<dyn NuEventSink>,
        estimator: Arc<dyn GasEstimator>,
        agent: WebID,
        dispatch_tx: mpsc::UnboundedSender<LoopMessage>,
    ) -> Self {
        Self {
            inner,
            cybernetics,
            event_sink,
            estimator,
            agent,
            dispatch_tx,
        }
    }

    /// Builder: change the agent for this membrane.
    pub fn with_agent(mut self, agent: WebID) -> Self {
        self.agent = agent;
        self
    }

    /// Verify OCAP authority: check that the token authorizes the tool invocation.
    fn verify_capability(token: &DelegationToken, tool_name: &str) -> Result<(), ToolPortError> {
        // Check that the token is valid for Tool:Execute on this resource
        if !token.is_valid_for(
            DelegationResource::Tool,
            tool_name,
            DelegationAction::Execute,
        ) {
            return Err(ToolPortError::CapabilityDenied(format!(
                "Token does not authorize tool: {}",
                tool_name
            )));
        }
        Ok(())
    }
}

impl<P: ToolPort + 'static> ToolPort for GovernedTool<P> {
    async fn invoke(
        &self,
        server: &str,
        tool: &str,
        args: Value,
        token: &DelegationToken,
    ) -> Result<Value, ToolPortError> {
        let estimated_cost = self.estimator.estimate_cost(server, tool, &args);

        // Step 1: Verify OCAP authority
        if let Err(e) = Self::verify_capability(token, tool) {
            warn!(
                target: "cns.tool",
                agent = ?self.agent,
                tool = %tool,
                error = %e,
                "Tool invocation rejected — capability denied"
            );
            return Err(e);
        }

        // Step 2: Reserve gas budget (hold-settle pattern)
        let loop6 = self.cybernetics.read().await;
        if !loop6.can_proceed(&self.agent, estimated_cost).await {
            debug!(
                target: "cns.tool",
                agent = ?self.agent,
                tool = %tool,
                estimated_cost = estimated_cost,
                "Tool invocation rejected — gas budget exceeded"
            );
            return Err(ToolPortError::GasBudgetExceeded(format!(
                "Gas budget exceeded for agent {:?}, tool {}, estimated cost {}",
                self.agent, tool, estimated_cost
            )));
        }
        // Reserve the gas
        if let Err(e) = loop6.reserve_gas(&self.agent, estimated_cost).await {
            warn!(
                target: "cns.tool",
                agent = ?self.agent,
                tool = %tool,
                error = %e,
                estimated_cost = estimated_cost,
                "Failed to reserve gas for tool invocation"
            );
            return Err(ToolPortError::GasBudgetExceeded(format!(
                "Gas reservation failed for agent {:?}, tool {}, estimated cost {}",
                self.agent, tool, estimated_cost
            )));
        }
        drop(loop6);

        // Step 3: Emit invoked span
        let invoked_span = Span::new(SpanNamespace::new("cns.tool"), "invoked");
        let invoked_event = NuEvent::new(
            self.agent,
            invoked_span,
            Phase::Sense,
            serde_json::json!({
                "server": server,
                "tool": tool,
                "estimated_cost": estimated_cost,
                "settled": false,
            }),
            0,
        );
        if let Err(e) = self.event_sink.persist(&invoked_event) {
            warn!(
                target: "cns.tool",
                error = %e,
                "Failed to persist cns.tool.invoked NuEvent"
            );
        }

        // Step 4: Delegate to inner tool
        info!(
            target: "cns.tool",
            agent = ?self.agent,
            tool = %tool,
            estimated_cost = estimated_cost,
            "Delegating tool invocation (gas reserved)"
        );
        let result = self.inner.invoke(server, tool, args, token).await;

        // Step 5: Settle gas cost (hold-settle)
        let actual_cost = match &result {
            Ok(_) => estimated_cost,      // Full cost on success
            Err(_) => estimated_cost / 2, // Half cost on failure
        };
        let loop6 = self.cybernetics.read().await;
        if let Err(e) = loop6
            .settle_gas(&self.agent, estimated_cost, actual_cost)
            .await
        {
            warn!(
                target: "cns.tool",
                agent = ?self.agent,
                tool = %tool,
                error = %e,
                reserved = estimated_cost,
                actual = actual_cost,
                "Failed to settle gas after tool invocation"
            );
        } else {
            info!(
                target: "cns.tool",
                agent = ?self.agent,
                tool = %tool,
                reserved = estimated_cost,
                actual = actual_cost,
                refunded = estimated_cost.saturating_sub(actual_cost),
                "Gas settled after tool invocation"
            );
        }
        drop(loop6);

        // Step 5b: Emit gas-consumed signal to Cybernetics Loop
        let success = result.is_ok();
        let consumption_msg = LoopMessage::new(
            MessagePriority::Info,
            LoopId::Cybernetics,
            LoopPayload::ToolConsumption {
                tool_name: tool.to_string(),
                agent: self.agent,
                gas_cost: actual_cost,
                success,
            },
        )
        .with_target(LoopId::Cybernetics);
        if let Err(e) = self.dispatch_tx.send(consumption_msg) {
            warn!(
                target: "cns.tool",
                agent = ?self.agent,
                tool = %tool,
                error = %e,
                "Failed to send ToolConsumption signal to Cybernetics Loop"
            );
        }

        // Step 6: Emit outcome span
        let (outcome_phase, outcome_obs) = match &result {
            Ok(_value) => (
                Phase::Act,
                serde_json::json!({
                    "server": server,
                    "tool": tool,
                    "estimated_cost": estimated_cost,
                    "actual_cost": actual_cost,
                    "status": "success",
                    "settled": true,
                }),
            ),
            Err(e) => (
                Phase::Act,
                serde_json::json!({
                    "server": server,
                    "tool": tool,
                    "estimated_cost": estimated_cost,
                    "actual_cost": actual_cost,
                    "status": "failure",
                    "error": e.to_string(),
                    "settled": true,
                }),
            ),
        };
        let completed_span = Span::new(SpanNamespace::new("cns.tool"), "completed");
        let completed_event =
            NuEvent::new(self.agent, completed_span, outcome_phase, outcome_obs, 0)
                .with_parent(invoked_event.id);
        if let Err(e) = self.event_sink.persist(&completed_event) {
            warn!(
                target: "cns.tool",
                error = %e,
                "Failed to persist cns.tool.completed NuEvent"
            );
        }

        result
    }

    async fn discover_tools(&self) -> Vec<String> {
        self.inner.discover_tools().await
    }

    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
        self.inner.get_tool_info(tool_name).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::energy::GasBudget;
    use crate::runtime::CnsRuntime;
    use crate::table_gas_estimator::TableGasEstimator;
    use hkask_types::event::{NuEvent, NuEventSink};
    use hkask_types::loops::LoopMessage;
    use hkask_types::ports::ToolPortError;
    use hkask_types::{InfrastructureError, WebID};
    use std::sync::{Arc, Mutex};
    use tokio::sync::RwLock;

    /// Mock ToolPort for testing GovernedTool.
    struct MockToolPort {
        should_fail: std::sync::atomic::AtomicBool,
    }

    impl MockToolPort {
        fn new() -> Self {
            Self {
                should_fail: std::sync::atomic::AtomicBool::new(false),
            }
        }

        fn failing() -> Self {
            Self {
                should_fail: std::sync::atomic::AtomicBool::new(true),
            }
        }
    }

    impl ToolPort for MockToolPort {
        async fn invoke(
            &self,
            _server: &str,
            _tool: &str,
            _args: Value,
            _token: &DelegationToken,
        ) -> Result<Value, ToolPortError> {
            if self.should_fail.load(std::sync::atomic::Ordering::Relaxed) {
                Err(ToolPortError::InvocationFailed("mock tool failure".into()))
            } else {
                Ok(serde_json::json!({"result": "ok"}))
            }
        }

        async fn discover_tools(&self) -> Vec<String> {
            vec!["test_tool".to_string()]
        }

        async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
            if tool_name == "test_tool" {
                Some(ToolInfo {
                    name: "test_tool".to_string(),
                    description: "A test tool".to_string(),
                    input_schema: serde_json::json!({}),
                    server_id: "test_server".to_string(),
                    required_capability: None,
                })
            } else {
                None
            }
        }
    }

    /// In-memory NuEventSink for testing.
    struct MockNuEventSink {
        events: Mutex<Vec<NuEvent>>,
    }

    impl MockNuEventSink {
        fn new() -> Self {
            Self {
                events: Mutex::new(Vec::new()),
            }
        }

        fn events(&self) -> Vec<NuEvent> {
            self.events.lock().unwrap().clone()
        }
    }

    impl NuEventSink for MockNuEventSink {
        fn persist(&self, event: &NuEvent) -> Result<(), InfrastructureError> {
            self.events.lock().unwrap().push(event.clone());
            Ok(())
        }
    }

    /// Helper: create a test DelegationToken that authorizes a tool.
    fn test_tool_token() -> DelegationToken {
        let secret = b"test-secret-key-for-governed-tool-tests";
        let checker = hkask_types::CapabilityChecker::new(secret);
        let from = WebID::new();
        let to = WebID::new();
        checker.grant_tool("test_tool".to_string(), from, to)
    }

    /// Helper: create a test DelegationToken that authorizes a different tool.
    fn wrong_tool_token() -> DelegationToken {
        let secret = b"test-secret-key-for-governed-tool-tests";
        let checker = hkask_types::CapabilityChecker::new(secret);
        let from = WebID::new();
        let to = WebID::new();
        checker.grant_tool("other_tool".to_string(), from, to)
    }

    /// Helper: create a CyberneticsLoop wired for tests, returning (loop, dispatch_tx).
    fn test_cybernetics_loop() -> (
        Arc<RwLock<CyberneticsLoop>>,
        tokio::sync::mpsc::UnboundedSender<LoopMessage>,
    ) {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<LoopMessage>();
        (
            Arc::new(RwLock::new(CyberneticsLoop::new(cns, tx.clone()))),
            tx,
        )
    }

    #[tokio::test]
    async fn governed_tool_rejects_when_capability_denied() {
        let (loop6, dispatch_tx) = test_cybernetics_loop();
        let agent = WebID::new();

        // Register a large budget so budget checks pass
        let budget = GasBudget::new(100_000);
        loop6.read().await.register_gas_budget(agent, budget).await;

        let inner = Arc::new(MockToolPort::new());
        let event_sink = Arc::new(MockNuEventSink::new());
        let governed = GovernedTool::new(
            inner,
            loop6,
            event_sink,
            Arc::new(TableGasEstimator::new()),
            agent,
            dispatch_tx,
        );

        // Token that authorizes "other_tool", not "test_tool"
        let wrong_token = wrong_tool_token();
        let result = governed
            .invoke(
                "test_server",
                "test_tool",
                serde_json::json!({}),
                &wrong_token,
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolPortError::CapabilityDenied(msg) => {
                assert!(
                    msg.contains("test_tool"),
                    "Expected tool name in error, got: {msg}"
                );
            }
            other => panic!("Expected CapabilityDenied, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn governed_tool_rejects_when_gas_budget_exhausted() {
        let (loop6, dispatch_tx) = test_cybernetics_loop();
        let agent = WebID::new();

        // GasBudget with cap=0 means remaining=0, so can_proceed(1) → gas=1 > 0 → false.
        let budget = GasBudget::new(0);
        loop6.read().await.register_gas_budget(agent, budget).await;

        let inner = Arc::new(MockToolPort::new());
        let event_sink = Arc::new(MockNuEventSink::new());
        let governed = GovernedTool::new(
            inner,
            loop6,
            event_sink,
            Arc::new(TableGasEstimator::new()),
            agent,
            dispatch_tx,
        );

        let token = test_tool_token();
        let result = governed
            .invoke("test_server", "test_tool", serde_json::json!({}), &token)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolPortError::GasBudgetExceeded(msg) => {
                assert!(
                    msg.contains("budget exceeded") || msg.contains("Gas budget"),
                    "Expected budget error, got: {msg}"
                );
            }
            other => panic!("Expected GasBudgetExceeded, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn governed_tool_allows_when_authorized_and_budget_sufficient() {
        let (loop6, dispatch_tx) = test_cybernetics_loop();
        let agent = WebID::new();

        let budget = GasBudget::new(100_000);
        loop6.read().await.register_gas_budget(agent, budget).await;

        let inner = Arc::new(MockToolPort::new());
        let event_sink = Arc::new(MockNuEventSink::new());
        let governed = GovernedTool::new(
            inner,
            loop6.clone(),
            event_sink.clone(),
            Arc::new(TableGasEstimator::new()),
            agent,
            dispatch_tx,
        );

        let token = test_tool_token();
        let result = governed
            .invoke("test_server", "test_tool", serde_json::json!({}), &token)
            .await;

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["result"], "ok");
    }

    #[tokio::test]
    async fn governed_tool_emits_spans_on_success() {
        let (loop6, dispatch_tx) = test_cybernetics_loop();
        let agent = WebID::new();

        let budget = GasBudget::new(100_000);
        loop6.read().await.register_gas_budget(agent, budget).await;

        let inner = Arc::new(MockToolPort::new());
        let event_sink = Arc::new(MockNuEventSink::new());
        let governed = GovernedTool::new(
            inner,
            loop6,
            event_sink.clone(),
            Arc::new(TableGasEstimator::new()),
            agent,
            dispatch_tx,
        );

        let token = test_tool_token();
        let _ = governed
            .invoke("test_server", "test_tool", serde_json::json!({}), &token)
            .await;

        let events = event_sink.events();
        assert_eq!(events.len(), 2, "Expected 2 events (invoked + completed)");
        assert!(
            events[0].span.path.contains("invoked"),
            "First event should be cns.tool.invoked"
        );
        assert!(
            events[1].span.path.contains("completed"),
            "Second event should be cns.tool.completed"
        );
    }

    #[tokio::test]
    async fn governed_tool_emits_spans_on_failure() {
        let (loop6, dispatch_tx) = test_cybernetics_loop();
        let agent = WebID::new();

        let budget = GasBudget::new(100_000);
        loop6.read().await.register_gas_budget(agent, budget).await;

        let inner = Arc::new(MockToolPort::failing());
        let event_sink = Arc::new(MockNuEventSink::new());
        let governed = GovernedTool::new(
            inner,
            loop6,
            event_sink.clone(),
            Arc::new(TableGasEstimator::new()),
            agent,
            dispatch_tx,
        );

        let token = test_tool_token();
        let result = governed
            .invoke("test_server", "test_tool", serde_json::json!({}), &token)
            .await;

        assert!(result.is_err());
        let events = event_sink.events();
        assert_eq!(events.len(), 2, "Expected 2 events even on failure");
        // Outcome should record failure
        let completed_obs = &events[1].observation;
        assert_eq!(completed_obs["status"], "failure");
    }

    /// Helper: create a DelegationToken that authorizes a specific tool name.
    fn token_for_tool(tool_name: &str) -> DelegationToken {
        let secret = b"test-secret-key-for-governed-tool-tests";
        let checker = hkask_types::CapabilityChecker::new(secret);
        let from = WebID::new();
        let to = WebID::new();
        checker.grant_tool(tool_name.to_string(), from, to)
    }

    // Integration: CompositeGasEstimator -> GovernedTool -> budget enforcement

    #[tokio::test]
    async fn governed_tool_with_composite_estimates_inference_gas() {
        let (loop6, dispatch_tx) = test_cybernetics_loop();
        let agent = WebID::new();

        // Register a gas budget large enough for both calls
        loop6
            .read()
            .await
            .register_gas_budget(agent, GasBudget::new(10_000))
            .await;

        // Create GovernedTool with CompositeGasEstimator
        let inner = Arc::new(MockToolPort::new());
        let event_sink = Arc::new(MockNuEventSink::new());
        let estimator = Arc::new(crate::composite_gas_estimator::CompositeGasEstimator::new());
        let governed = GovernedTool::new(inner, loop6, event_sink, estimator, agent, dispatch_tx);

        // Invoke inference tool -- should use token-based estimation
        let token = token_for_tool("generate");
        let args = serde_json::json!({"prompt": "Hello, world!", "max_tokens": 100});
        let result = governed
            .invoke("hkask-mcp-inference", "generate", args, &token)
            .await;
        assert!(result.is_ok(), "Inference invocation should succeed");

        // Invoke web tool -- should use table estimation (50 gas)
        let token = token_for_tool("search");
        let result = governed
            .invoke("hkask-mcp-web", "search", serde_json::json!({}), &token)
            .await;
        assert!(result.is_ok(), "Web invocation should succeed");
    }

    #[tokio::test]
    async fn governed_tool_composite_rejects_when_budget_exhausted() {
        let (loop6, dispatch_tx) = test_cybernetics_loop();
        let agent = WebID::new();

        // Tiny budget: 10 gas -- web tool costs 50 gas via table estimator
        loop6
            .read()
            .await
            .register_gas_budget(agent, GasBudget::new(10))
            .await;

        let inner = Arc::new(MockToolPort::new());
        let event_sink = Arc::new(MockNuEventSink::new());
        let estimator = Arc::new(crate::composite_gas_estimator::CompositeGasEstimator::new());
        let governed = GovernedTool::new(inner, loop6, event_sink, estimator, agent, dispatch_tx);

        let token = token_for_tool("search");
        // Web tool costs 50 gas -- exceeds budget of 10
        let result = governed
            .invoke("hkask-mcp-web", "search", serde_json::json!({}), &token)
            .await;
        assert!(result.is_err(), "Should reject when budget exhausted");
        match result.unwrap_err() {
            ToolPortError::GasBudgetExceeded(msg) => {
                assert!(
                    msg.contains("gas") || msg.contains("budget"),
                    "Expected gas/budget in error message, got: {}",
                    msg
                );
            }
            other => panic!("Expected GasBudgetExceeded, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn governed_tool_settles_gas_on_success() {
        let (loop6, dispatch_tx) = test_cybernetics_loop();
        let agent = WebID::new();

        // GasBudget: cap=100, remaining=100, replenish_rate=10
        // After one call with TableGasEstimator (cost=10 for unknown server):
        // Reserve 10, then settle with actual 10 \u2192 remaining=90
        // can_proceed(100) \u2192 gas=100 > 90 \u2192 should fail.
        let budget = GasBudget::new(100);
        loop6.read().await.register_gas_budget(agent, budget).await;

        let inner = Arc::new(MockToolPort::new());
        let event_sink = Arc::new(MockNuEventSink::new());
        let governed = GovernedTool::new(
            inner,
            loop6.clone(),
            event_sink,
            Arc::new(TableGasEstimator::new()),
            agent,
            dispatch_tx,
        );

        let token = test_tool_token();
        let result = governed
            .invoke("test_server", "test_tool", serde_json::json!({}), &token)
            .await;
        assert!(result.is_ok());

        // After a successful call, the gas should have been settled.
        // can_proceed(100) → gas=100 > 90 → should fail.
        let loop6_guard = loop6.read().await;
        assert!(!loop6_guard.can_proceed(&agent, 100).await);
    }
}
