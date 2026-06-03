//! GovernedTool — Capability-gated, energy-accounted, observability-emitting membrane
//!
//! Wraps a `dyn ToolPort` and implements `ToolPort` itself. Before delegating
//! to the inner tool, it checks:
//! 1. Authority (OCAP) — DelegationToken verification
//! 2. Budget (Cybernetics) — can_proceed / acquire_budget
//! 3. Emits span (CNS) — cns.tool.invoked
//! 4. Delegates to inner tool
//! 5. Accounts energy cost (Cybernetics) — acquire_budget
//! 6. Emits outcome span (CNS) — cns.tool.completed
//!
//! This is the membrane where Cybernetics governs all tool invocations.
//! The membrane IS the security property (Miller). GovernedTool subsumes:
//! - GovernedInference (for tool-style inference calls)
//! - SecurityGateway dispatch-time OCAP checks
//! - check_throttle (energy accounting replaces rate limiting)
//! - ToolSpanGuard (span emission is now built-in)

use crate::cybernetics_loop::CyberneticsLoop;
use hkask_types::NuEventSink;
use hkask_types::WebID;
use hkask_types::capability::{DelegationAction, DelegationResource, DelegationToken};
use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};
use hkask_types::id::EventID;
use hkask_types::ports::{ToolInfo, ToolPort, ToolPortError};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Energy estimator trait for GovernedTool.
///
/// Different tool categories have different cost models:
/// - Inference: estimated by token count
/// - Storage: estimated by byte count
/// - Compute: estimated by wall-clock time
///
/// The default implementation uses a flat cost of 1 energy unit per invocation.
pub trait EnergyEstimator: Send + Sync {
    /// Estimate the energy cost of a tool invocation before it happens.
    fn estimate_cost(&self, server: &str, tool: &str, args: &Value) -> u64;
}

/// Flat energy estimator — 1 unit per invocation.
/// Use this as a baseline; replace with domain-specific estimators
/// (e.g., inference token estimation) for production use.
pub struct FlatEnergyEstimator;

impl EnergyEstimator for FlatEnergyEstimator {
    fn estimate_cost(&self, _server: &str, _tool: &str, _args: &Value) -> u64 {
        1
    }
}

/// GovernedTool — the singular membrane through which all tool invocations pass.
///
/// This struct wraps a `dyn ToolPort` and enforces OCAP authority, energy
/// budgets, and CNS observability. It implements `ToolPort` itself — the
/// membrane IS a ToolPort (Miller's membrane object pattern).
///
/// # Composition
///
/// ```ignore
/// let inner: Arc<dyn ToolPort> = Arc::new(McpDispatcher::new(...));
/// let governed = GovernedTool::new(
///     inner,
///     cybernetics_loop,
///     event_sink,
///     Arc::new(FlatEnergyEstimator),
///     agent_webid,
/// );
/// // governed implements ToolPort — use it anywhere ToolPort is expected
/// ```
pub struct GovernedTool {
    inner: Arc<dyn ToolPort>,
    cybernetics: Arc<RwLock<CyberneticsLoop>>,
    event_sink: Arc<dyn NuEventSink>,
    estimator: Arc<dyn EnergyEstimator>,
    agent: WebID,
}

impl GovernedTool {
    /// Create a new GovernedTool membrane wrapping an inner ToolPort.
    pub fn new(
        inner: Arc<dyn ToolPort>,
        cybernetics: Arc<RwLock<CyberneticsLoop>>,
        event_sink: Arc<dyn NuEventSink>,
        estimator: Arc<dyn EnergyEstimator>,
        agent: WebID,
    ) -> Self {
        Self {
            inner,
            cybernetics,
            event_sink,
            estimator,
            agent,
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

#[async_trait::async_trait]
impl ToolPort for GovernedTool {
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

        // Step 2: Check energy budget
        let loop6 = self.cybernetics.read().await;
        if !loop6.can_proceed(&self.agent, estimated_cost).await {
            debug!(
                target: "cns.tool",
                agent = ?self.agent,
                tool = %tool,
                estimated_cost = estimated_cost,
                "Tool invocation rejected — energy budget exceeded"
            );
            return Err(ToolPortError::EnergyBudgetExceeded(format!(
                "Energy budget exceeded for agent {:?}, tool {}, estimated cost {}",
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
            "Delegating tool invocation"
        );
        let result = self.inner.invoke(server, tool, args, token).await;

        // Step 5: Account energy cost
        let actual_cost = match &result {
            Ok(_) => estimated_cost,
            Err(_) => estimated_cost / 2, // Charge half cost on failure
        };
        let loop6 = self.cybernetics.read().await;
        if let Err(e) = loop6.acquire_budget(&self.agent, actual_cost).await {
            warn!(
                target: "cns.tool",
                agent = ?self.agent,
                tool = %tool,
                error = %e,
                actual_cost = actual_cost,
                "Failed to account energy cost after tool invocation"
            );
        }
        drop(loop6);

        // Step 6: Emit outcome span
        let (outcome_phase, outcome_obs) = match &result {
            Ok(value) => (
                Phase::Act,
                serde_json::json!({
                    "server": server,
                    "tool": tool,
                    "actual_cost": actual_cost,
                    "status": "success",
                }),
            ),
            Err(e) => (
                Phase::Act,
                serde_json::json!({
                    "server": server,
                    "tool": tool,
                    "actual_cost": actual_cost,
                    "status": "failure",
                    "error": e.to_string(),
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
    use crate::energy::EnergyBudget;
    use crate::runtime::CnsRuntime;
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

    #[async_trait::async_trait]
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

    /// Helper: create a CyberneticsLoop wired for tests.
    fn test_cybernetics_loop() -> Arc<RwLock<CyberneticsLoop>> {
        let cns = Arc::new(RwLock::new(CnsRuntime::default()));
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel::<LoopMessage>();
        Arc::new(RwLock::new(CyberneticsLoop::new(cns, tx)))
    }

    #[tokio::test]
    async fn governed_tool_rejects_when_capability_denied() {
        let loop6 = test_cybernetics_loop();
        let agent = WebID::new();

        // Register a large budget so budget checks pass
        let budget = EnergyBudget::new(100_000).with_cost_per_token(1.0);
        loop6
            .read()
            .await
            .register_energy_budget(agent, budget)
            .await;

        let inner = Arc::new(MockToolPort::new());
        let event_sink = Arc::new(MockNuEventSink::new());
        let governed = GovernedTool::new(
            inner,
            loop6,
            event_sink,
            Arc::new(FlatEnergyEstimator),
            agent,
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
    async fn governed_tool_rejects_when_energy_budget_exhausted() {
        let loop6 = test_cybernetics_loop();
        let agent = WebID::new();

        // EnergyBudget with cost_per_token=1.0 so that flat cost=1 produces real cost=1.
        // cap=1, remaining=1, so can_proceed(1) → cost=1 ≤ 1 → passes (budget OK).
        // But after one invocation, remaining=0, so second call fails.
        // Instead, use cap=0 which means remaining=0, so can_proceed(1) → cost=1 > 0 → false.
        // Actually, with cost_per_token=1.0, calculate_cost(1) = 1, so can_proceed(1) with remaining=0 → false.
        let budget = EnergyBudget::new(0).with_cost_per_token(1.0);
        loop6
            .read()
            .await
            .register_energy_budget(agent, budget)
            .await;

        let inner = Arc::new(MockToolPort::new());
        let event_sink = Arc::new(MockNuEventSink::new());
        let governed = GovernedTool::new(
            inner,
            loop6,
            event_sink,
            Arc::new(FlatEnergyEstimator),
            agent,
        );

        let token = test_tool_token();
        let result = governed
            .invoke("test_server", "test_tool", serde_json::json!({}), &token)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolPortError::EnergyBudgetExceeded(msg) => {
                assert!(
                    msg.contains("Energy budget exceeded"),
                    "Expected budget error, got: {msg}"
                );
            }
            other => panic!("Expected EnergyBudgetExceeded, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn governed_tool_allows_when_authorized_and_budget_sufficient() {
        let loop6 = test_cybernetics_loop();
        let agent = WebID::new();

        let budget = EnergyBudget::new(100_000).with_cost_per_token(1.0);
        loop6
            .read()
            .await
            .register_energy_budget(agent, budget)
            .await;

        let inner = Arc::new(MockToolPort::new());
        let event_sink = Arc::new(MockNuEventSink::new());
        let governed = GovernedTool::new(
            inner,
            loop6.clone(),
            event_sink.clone(),
            Arc::new(FlatEnergyEstimator),
            agent,
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
        let loop6 = test_cybernetics_loop();
        let agent = WebID::new();

        let budget = EnergyBudget::new(100_000).with_cost_per_token(1.0);
        loop6
            .read()
            .await
            .register_energy_budget(agent, budget)
            .await;

        let inner = Arc::new(MockToolPort::new());
        let event_sink = Arc::new(MockNuEventSink::new());
        let governed = GovernedTool::new(
            inner,
            loop6,
            event_sink.clone(),
            Arc::new(FlatEnergyEstimator),
            agent,
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
        let loop6 = test_cybernetics_loop();
        let agent = WebID::new();

        let budget = EnergyBudget::new(100_000).with_cost_per_token(1.0);
        loop6
            .read()
            .await
            .register_energy_budget(agent, budget)
            .await;

        let inner = Arc::new(MockToolPort::failing());
        let event_sink = Arc::new(MockNuEventSink::new());
        let governed = GovernedTool::new(
            inner,
            loop6,
            event_sink.clone(),
            Arc::new(FlatEnergyEstimator),
            agent,
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

    #[tokio::test]
    async fn governed_tool_deducts_energy_on_success() {
        let loop6 = test_cybernetics_loop();
        let agent = WebID::new();

        // EnergyBudget with cost_per_token=1.0 so that FlatEnergyEstimator's cost=1 maps to real cost=1.
        // cap=100, remaining=100. After one call: acquire_budget(1) → cost=1 consumed, remaining=99.
        // can_proceed(100) → cost=100 > 99 → should fail.
        let budget = EnergyBudget::new(100).with_cost_per_token(1.0);
        loop6
            .read()
            .await
            .register_energy_budget(agent, budget)
            .await;

        let inner = Arc::new(MockToolPort::new());
        let event_sink = Arc::new(MockNuEventSink::new());
        let governed = GovernedTool::new(
            inner,
            loop6.clone(),
            event_sink,
            Arc::new(FlatEnergyEstimator),
            agent,
        );

        let token = test_tool_token();
        let result = governed
            .invoke("test_server", "test_tool", serde_json::json!({}), &token)
            .await;
        assert!(result.is_ok());

        // After a successful call, the budget should have been deducted.
        // can_proceed(100) → cost=100 > 99 → should fail.
        let loop6_guard = loop6.read().await;
        assert!(!loop6_guard.can_proceed(&agent, 100).await);
    }
}
