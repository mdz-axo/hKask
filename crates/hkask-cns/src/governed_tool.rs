//! GovernedTool — Capability-gated, gas-accounted, observability-emitting membrane
//!
//! Wraps a `ToolPort` and implements `ToolPort` itself. Before delegating
//! to the inner tool, it checks:
//! 1. Authority (OCAP) — two-path DelegationToken verification:
//!    - Path 1 (exact): exact-match on tool name (ad-hoc invocation tokens)
//!    - Path 2 (domain): capability-domain matching via `capabilities_match()`
//!      (agent capability tokens use domain shorthand like "cns" not "cns_health")
//! 2. Budget (Cybernetics) — reserve gas
//! 3. Emits span (CNS) — cns.tool.invoked
//! 4. Delegates to inner tool
//! 5. Settles energy cost (Cybernetics) — settle actual vs. reserved
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
use crate::energy::EnergyCost;
use crate::types::loops::ToolConsumptionEvent;
use hkask_capability::{DelegationAction, DelegationResource, DelegationToken, capabilities_match};
use hkask_ports::{ToolInfo, ToolPort, ToolPortError};
use hkask_types::NuEventSink;
use hkask_types::WebID;
use hkask_types::cns::CnsSpan;
use hkask_types::event::{CyclePhase, NuEvent, Span, SpanKind, SpanNamespace};

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info, warn};

/// Gas estimator trait for GovernedTool.
///
/// Gas is a dimensionless cost unit analogous to Ethereum gas: it prevents
/// infinite loops by making resource exhaustion explicit. Different tool
/// categories have different cost models:
/// - Inference: estimated by token count (InferenceEnergyEstimator)
/// - All other tools: flat costs from TableEnergyEstimator
///
/// Use `TableEnergyEstimator` for production (per-server energy costs) or
/// `InferenceEnergyEstimator` for inference-specific token-based estimation.
pub trait EnergyEstimator: Send + Sync {
    /// Estimate the energy cost of a tool invocation before it happens.
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
///     Arc::new(TableEnergyEstimator::new()),
///     agent_webid,
///     dispatch_tx,
/// );
/// // governed implements ToolPort — use it anywhere ToolPort is expected
/// ```
pub struct GovernedTool<P: ToolPort> {
    inner: Arc<P>,
    cybernetics: Arc<RwLock<CyberneticsLoop>>,
    event_sink: Arc<dyn NuEventSink>,
    estimator: Arc<dyn EnergyEstimator>,
    agent: WebID,
    /// Direct tool consumption channel: GovernedTool → Cybernetics
    tool_consumption_tx: Option<mpsc::UnboundedSender<ToolConsumptionEvent>>,
}

impl<P: ToolPort> GovernedTool<P> {
    /// Create a new GovernedTool membrane wrapping an inner ToolPort.
    ///
    /// expect: "The system creates a governed tool membrane that gates execution behind energy and OCAP checks"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — tool governance enables feedback loops
    /// \[P4\] Constraining: Clear Boundaries — cybernetics binding enforces OCAP boundary
    /// pre:  inner is valid, cns is valid
    /// post: returns GovernedTool
    ///
    /// Per P4: the Cybernetics binding here is the OCAP enforcement point —
    /// every tool invocation flows through this membrane.
    pub fn new(
        inner: Arc<P>,
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
            tool_consumption_tx: None,
        }
    }

    /// Wire the direct tool consumption channel: GovernedTool → Cybernetics.
    #[must_use = "builder methods must be chained or assigned"]
    /// Set the tool consumption channel.
    ///
    /// expect: "The system wires tool consumption events back to the cybernetics loop"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — consumption channel closes the cybernetic feedback loop
    /// \[P4\] Constraining: Clear Boundaries — channel ownership tracks consumer identity
    /// @must_use because builder methods must be chained or assigned
    /// post: returns Self with channel set (builder pattern)
    pub fn with_tool_consumption_channel(
        mut self,
        tx: mpsc::UnboundedSender<ToolConsumptionEvent>,
    ) -> Self {
        self.tool_consumption_tx = Some(tx);
        self
    }

    /// Builder: change the agent for this membrane.
    /// Set the agent WebID for attribution.
    ///
    /// expect: "I can bind an agent identity to the governance membrane for attribution"
    /// \[P12\] Motivating: Affirmative Consent — agent identity is the consent anchor
    /// \[P4\] Constraining: Clear Boundaries — OCAP gate enforces boundary per invocation
    /// @must_use because builder methods must be chained or assigned
    /// post: returns Self with agent set (builder pattern)
    pub fn with_agent(mut self, agent: WebID) -> Self {
        self.agent = agent;
        self
    }

    /// Verify OCAP authority via exact-match (ad-hoc invocation tokens).
    ///
    /// Ad-hoc tokens are minted with the exact tool name as `resource_id`
    /// (e.g., token for `cns_health`).
    fn verify_capability_exact(token: &DelegationToken, tool_name: &str) -> bool {
        token.is_valid_for(
            DelegationResource::Tool,
            tool_name,
            DelegationAction::Execute,
        )
    }

    /// Verify OCAP authority via domain-based capability matching.
    ///
    /// Agent capability tokens use domain shorthand (e.g., `cns` not `cns_health`).
    /// The tool's `required_capability` declares its domain (e.g., `tool:cns:execute`).
    /// If the token's capability covers the tool's required domain, access is granted.
    fn verify_capability_domain(token: &DelegationToken, required_capability: &str) -> bool {
        let token_capability = format!("tool:{}:{}", token.resource_id, token.action.as_str());
        capabilities_match(&token_capability, required_capability)
    }

    /// Async fallback: look up tool metadata and try domain-based matching.
    ///
    /// Called when exact-match fails. Returns `true` if the tool has a
    /// `required_capability` and the token covers it.
    async fn verify_capability_domain_fallback(
        &self,
        token: &DelegationToken,
        tool_name: &str,
    ) -> bool {
        match self.inner.get_tool_info(tool_name).await {
            Some(ref info) => match info.required_capability {
                Some(ref required) => Self::verify_capability_domain(token, required),
                None => false,
            },
            None => false,
        }
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

        // Step 0: Verify cryptographic authenticity of the delegation token
        if !token.verify() {
            warn!(
                target: "cns.tool",
                agent = ?self.agent,
                tool = %tool,
                "Tool invocation rejected — token signature verification failed"
            );
            return Err(ToolPortError::CapabilityDenied(
                "Token failed cryptographic verification".to_string(),
            ));
        }

        // Step 1: Verify OCAP authority
        // Exact-match (ad-hoc tokens) or domain-based (agent tokens)
        let authorized = Self::verify_capability_exact(token, tool)
            || self.verify_capability_domain_fallback(token, tool).await;
        if !authorized {
            warn!(
                target: "cns.tool",
                agent = ?self.agent,
                tool = %tool,
                "Tool invocation rejected — capability denied"
            );
            return Err(ToolPortError::CapabilityDenied(format!(
                "Token does not authorize tool: {}",
                tool
            )));
        }

        // Step 2: Reserve energy budget (hold-settle pattern)
        let estimated_cost = EnergyCost(estimated_cost);
        let loop6 = self.cybernetics.read().await;
        if !loop6.can_proceed(&self.agent, estimated_cost).await {
            // Emit cns.gas.depleted span
            let depleted_span = Span::from_kind(SpanKind::GasDepleted);
            let depleted_event = NuEvent::new(
                self.agent,
                depleted_span,
                CyclePhase::Sense,
                serde_json::json!({
                    "server": server,
                    "tool": tool,
                    "estimated_cost": estimated_cost.0,
                }),
                0,
            );
            let _ = self.event_sink.persist(&depleted_event);

            debug!(
                target: "cns.tool",
                agent = ?self.agent,
                tool = %tool,
                estimated_cost = estimated_cost.0,
                "Tool invocation rejected — energy budget exceeded"
            );
            return Err(ToolPortError::EnergyBudgetExceeded(format!(
                "Gas budget exceeded for agent {:?}, tool {}, estimated cost {}",
                self.agent, tool, estimated_cost.0
            )));
        }
        // Reserve the gas
        if let Err(e) = loop6.reserve_gas(&self.agent, estimated_cost).await {
            warn!(
                target: "cns.tool",
                agent = ?self.agent,
                tool = %tool,
                error = %e,
                estimated_cost = estimated_cost.0,
                "Failed to reserve gas for tool invocation"
            );
            return Err(ToolPortError::EnergyBudgetExceeded(format!(
                "Gas reservation failed for agent {:?}, tool {}, estimated cost {}",
                self.agent, tool, estimated_cost.0
            )));
        }
        drop(loop6);

        // Emit cns.gas.reserved span
        let reserved_span = Span::from_kind(SpanKind::GasReserved);
        let reserved_event = NuEvent::new(
            self.agent,
            reserved_span,
            CyclePhase::Act,
            serde_json::json!({
                "server": server,
                "tool": tool,
                "estimated_cost": estimated_cost.0,
            }),
            0,
        );
        let _ = self.event_sink.persist(&reserved_event);

        // Step 3: Emit invoked span
        let invoked_span = Span::new(
            SpanNamespace::from(CnsSpan::Tool {
                subsystem: hkask_types::cns::ToolSubsystem::from_server_name(server),
            }),
            "invoked",
        );
        let invoked_event = NuEvent::new(
            self.agent,
            invoked_span,
            CyclePhase::Sense,
            serde_json::json!({
                "server": server,
                "tool": tool,
                "estimated_cost": estimated_cost.0,
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
            estimated_cost = estimated_cost.0,
            "Delegating tool invocation (gas reserved)"
        );
        let result = self.inner.invoke(server, tool, args, token).await;

        // Step 5: Settle energy cost (hold-settle)
        let actual_cost = match &result {
            Ok(_) => estimated_cost.0,      // Full cost on success
            Err(_) => estimated_cost.0 / 2, // Half cost on failure
        };
        let loop6 = self.cybernetics.read().await;
        if let Err(e) = loop6
            .settle_gas(&self.agent, estimated_cost, EnergyCost(actual_cost))
            .await
        {
            warn!(
                target: "cns.tool",
                agent = ?self.agent,
                tool = %tool,
                error = %e,
                reserved = estimated_cost.0,
                actual = actual_cost,
                "Failed to settle gas after tool invocation"
            );
        } else {
            info!(
                target: "cns.tool",
                agent = ?self.agent,
                tool = %tool,
                reserved = estimated_cost.0,
                actual = actual_cost,
                refunded = estimated_cost.0.saturating_sub(actual_cost),
                "Gas settled after tool invocation"
            );
        }
        drop(loop6);

        // Emit cns.gas.settled span
        let settled_span = Span::from_kind(SpanKind::GasSettled);
        let settled_event = NuEvent::new(
            self.agent,
            settled_span,
            CyclePhase::Act,
            serde_json::json!({
                "server": server,
                "tool": tool,
                "reserved": estimated_cost.0,
                "actual": actual_cost,
                "refunded": estimated_cost.0.saturating_sub(actual_cost),
            }),
            0,
        );
        let _ = self.event_sink.persist(&settled_event);

        // Step 5b: Emit gas-consumed signal to Cybernetics Loop via direct channel.
        let success = result.is_ok();
        if let Some(ref tx) = self.tool_consumption_tx {
            let event = ToolConsumptionEvent {
                tool_name: tool.to_string(),
                agent: self.agent,
                gas_cost: actual_cost,
                success,
            };
            if let Err(e) = tx.send(event) {
                warn!(
                    target: "cns.tool",
                    agent = ?self.agent,
                    tool = %tool,
                    error = %e,
                    "Failed to send ToolConsumptionEvent on direct channel"
                );
            }
        }

        // Step 6: Emit outcome span
        let (outcome_phase, outcome_obs) = match &result {
            Ok(_value) => (
                CyclePhase::Act,
                serde_json::json!({
                    "server": server,
                    "tool": tool,
                    "estimated_cost": estimated_cost.0,
                    "actual_cost": actual_cost,
                    "status": "success",
                    "settled": true,
                }),
            ),
            Err(e) => (
                CyclePhase::Act,
                serde_json::json!({
                    "server": server,
                    "tool": tool,
                    "estimated_cost": estimated_cost.0,
                    "actual_cost": actual_cost,
                    "status": "failure",
                    "error": e.to_string(),
                    "settled": true,
                }),
            ),
        };
        let completed_span = Span::new(
            SpanNamespace::from(CnsSpan::Tool {
                subsystem: hkask_types::cns::ToolSubsystem::from_server_name(server),
            }),
            "completed",
        );
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

        // Step 7: Record outcome for quality tracking (success rate per domain)
        {
            let cybernetics = self.cybernetics.read().await;
            let error_kind: Option<String> = match &result {
                Err(e) => {
                    // Extract the error kind — use the ToolPortError variant name
                    let err_str = e.to_string();
                    // Take first line or first 64 chars as the error kind
                    let kind = err_str.lines().next().unwrap_or(&err_str);
                    Some(kind.chars().take(64).collect())
                }
                Ok(_) => None,
            };
            cybernetics
                .record_outcome(server, success, error_kind.as_deref())
                .await;
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

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_capability::{
        DelegationAction, DelegationResource, DelegationToken, DelegationTokenBuilder,
        derive_signing_key,
    };
    use hkask_types::WebID;

    fn make_token(resource_id: &str) -> DelegationToken {
        let sk = derive_signing_key(b"test-secret-32-bytes-long!!");
        DelegationTokenBuilder::new(
            DelegationResource::Tool,
            resource_id.into(),
            DelegationAction::Execute,
            WebID::new(),
            WebID::new(),
            &sk,
        )
        .sign()
    }

    //
    // OCAP Path 1: a DelegationToken minted for a specific tool name
    // must grant access when the tool name matches exactly.
    #[test]
    fn exact_match_grants_correct_tool() {
        let token = make_token("cns_health");

        assert!(GovernedTool::<NoOpToolPort>::verify_capability_exact(
            &token,
            "cns_health"
        ));
    }

    //
    // OCAP Path 1: a token for one tool must not grant access to another.
    #[test]
    fn exact_match_denies_wrong_tool() {
        let token = make_token("cns_health");

        assert!(!GovernedTool::<NoOpToolPort>::verify_capability_exact(
            &token,
            "prompt_invoke"
        ));
    }

    //
    // OCAP Path 2: an agent capability token with domain "cns" and action
    // "execute" must grant access to a tool with required_capability
    // "tool:cns:execute" via capabilities_match().
    #[test]
    fn domain_capability_matches_mcp_tool_domain() {
        let token = make_token("cns");

        assert!(GovernedTool::<NoOpToolPort>::verify_capability_domain(
            &token,
            "tool:cns:execute"
        ));
    }

    //
    // A token for domain "cns" must not grant access to a tool
    // requiring "tool:memory:write".
    #[test]
    fn domain_capability_denies_different_domain() {
        let token = make_token("cns");

        assert!(!GovernedTool::<NoOpToolPort>::verify_capability_domain(
            &token,
            "tool:memory:write"
        ));
    }

    // No-op ToolPort for testing static verification methods.
    // The verification_functions are pure (no ToolPort dispatch needed).
    struct NoOpToolPort;
    impl ToolPort for NoOpToolPort {
        async fn invoke(
            &self,
            _server: &str,
            _tool: &str,
            _args: serde_json::Value,
            _token: &DelegationToken,
        ) -> Result<serde_json::Value, ToolPortError> {
            Ok(serde_json::Value::Null)
        }
        async fn discover_tools(&self) -> Vec<String> {
            vec![]
        }
        async fn get_tool_info(&self, _tool_name: &str) -> Option<ToolInfo> {
            None
        }
    }
}
