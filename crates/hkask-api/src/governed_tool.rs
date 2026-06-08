//! Governed MCP tool + dispatcher bundle (P2.2).
//!
//! Extracted from `ApiState::new()`. Wraps the gas estimator, raw tool port,
//! and `GovernedTool` membrane into the `McpDispatcher` that all tool
//! invocations route through. Returns the dispatcher plus a cloned
//! `CyberneticsLoop` handle for downstream gas-governance adapters.

use std::sync::Arc;

use hkask_cns::{CompositeGasEstimator, GasEstimator, GovernedTool};
use hkask_types::WebID;
use hkask_types::event::NuEventSink;

/// Governed MCP tool + dispatcher bundle (P2.2).
///
/// Extracted from `ApiState::new()`. Wraps the gas estimator, raw tool port,
/// and `GovernedTool` membrane into the `McpDispatcher` that all tool
/// invocations route through. Returns the dispatcher plus a cloned
/// `CyberneticsLoop` handle for downstream gas-governance adapters.
pub(crate) struct GovernedMcpTool {
    pub mcp_dispatcher: Arc<hkask_mcp::dispatch::McpDispatcher>,
    /// Cloned before being moved into `GovernedTool`; needed for the
    /// `ApiGasGovernanceAdapter` that the ensemble session manager consumes.
    pub cybernetics_loop_for_gas: Arc<tokio::sync::RwLock<hkask_cns::CyberneticsLoop>>,
}

/// Build the `GovernedTool` membrane and `McpDispatcher` that route every tool
/// invocation through CNS gas governance.
///
/// P2.2 extraction: this block is the largest single section of
/// `ApiState::new()` (after `Stores::init` and `build_loop_system` were
/// already extracted). Isolating it makes the wiring self-documenting and
/// the failure mode (e.g. tokio handle missing) testable in isolation.
pub(crate) fn build_governed_mcp_tool(
    dispatcher_runtime: hkask_mcp::runtime::McpRuntime,
    cybernetics_loop_rwlock: Arc<tokio::sync::RwLock<hkask_cns::CyberneticsLoop>>,
    cns_event_sink: Arc<dyn NuEventSink>,
    loop_system: &hkask_agents::loop_system::LoopSystem,
    system_webid: WebID,
    capability_secret: &[u8],
) -> GovernedMcpTool {
    let raw_tool_port = Arc::new(hkask_mcp::raw_tool_port::RawMcpToolPort::new(
        dispatcher_runtime.clone(),
    ));
    let estimator: Arc<dyn GasEstimator> = Arc::new(CompositeGasEstimator::new());
    let governed_tool = Arc::new(GovernedTool::new(
        raw_tool_port,
        Arc::clone(&cybernetics_loop_rwlock),
        cns_event_sink,
        estimator,
        system_webid,
        loop_system.dispatch_sender(),
    ));
    let mcp_dispatcher = Arc::new(hkask_mcp::dispatch::McpDispatcher::with_governed_tool(
        dispatcher_runtime,
        capability_secret,
        governed_tool,
    ));
    GovernedMcpTool {
        mcp_dispatcher,
        cybernetics_loop_for_gas: cybernetics_loop_rwlock,
    }
}
