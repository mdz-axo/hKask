//! MCP Runtime Adapter
//
//! Concrete implementation of MCPRuntimePort.
//! Routes tool invocations through `McpRuntime`'s live MCP server
//! connections when available. Falls back to capability-only verification
//! when no runtime is wired (e.g., in tests).

use crate::error::McpError;
use crate::ports::MCPRuntimePort;
use hkask_mcp::runtime::McpRuntime;
use hkask_types::{
    CapabilityChecker, DelegationAction, DelegationResource, DelegationToken, VerificationOutcome,
    verify_delegation_token,
};
use std::sync::Arc;

/// MCP Runtime Adapter — Concrete implementation for tool access
///
/// When wired with an `McpRuntime`, routes tool invocations through
/// live MCP server connections (spawned via `McpRuntime::start_server()`).
/// When no runtime is provided (e.g., in tests), returns an error on
/// invocation.
#[derive(Default, Clone)]
pub struct McpRuntimeAdapter {
    /// Optional capability checker for HMAC verification
    capability_checker: Option<Arc<CapabilityChecker>>,
    /// MCP runtime for live tool dispatch
    mcp_runtime: Option<Arc<McpRuntime>>,
    /// Tokio runtime handle for bridging sync→async
    handle: Option<tokio::runtime::Handle>,
}

impl McpRuntimeAdapter {
    /// Create new MCP runtime adapter (no runtime, capability-only).
    ///
    /// Tool invocations will fail with `McpError::NoRuntime`. Use
    /// `with_runtime()` for a working adapter.
    pub fn new() -> Self {
        Self {
            capability_checker: None,
            mcp_runtime: None,
            handle: None,
        }
    }

    /// Set the capability checker for cryptographic OCAP verification
    pub fn with_capability_checker(mut self, checker: CapabilityChecker) -> Self {
        self.capability_checker = Some(Arc::new(checker));
        self
    }

    /// Wire the adapter through a live `McpRuntime` for actual MCP dispatch.
    ///
    /// The `handle` is used to bridge synchronous trait methods to the
    /// async `McpRuntime` calls. Obtain it from `tokio::runtime::Handle::current()`
    /// or from a `Runtime::handle()`.
    pub fn with_runtime(
        mut self,
        runtime: Arc<McpRuntime>,
        handle: tokio::runtime::Handle,
    ) -> Self {
        self.mcp_runtime = Some(runtime);
        self.handle = Some(handle);
        self
    }
}

impl MCPRuntimePort for McpRuntimeAdapter {
    fn grant_tool_access(&self, token: DelegationToken) -> Result<(), McpError> {
        let token_id = token.id.clone();

        if token_id.is_empty() {
            return Err(McpError::InvalidToken("Token ID is empty".to_string()));
        }

        // P1.1: Use unified verification instead of inline HMAC check
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        match verify_delegation_token(
            self.capability_checker.as_ref().map(|a| a.as_ref()),
            &token,
            &token.delegated_to,
            DelegationResource::Tool,
            &token.resource_id,
            DelegationAction::Execute,
            current_time,
        ) {
            VerificationOutcome::Valid => Ok(()),
            VerificationOutcome::InvalidSignature => Err(McpError::InvalidToken(
                "Token signature verification failed".to_string(),
            )),
            VerificationOutcome::Expired => {
                Err(McpError::CapabilityDenied("Token is expired".to_string()))
            }
            VerificationOutcome::InsufficientAccess { .. } => Err(McpError::CapabilityDenied(
                format!("Token does not authorize tool: {}", token.resource_id),
            )),
            VerificationOutcome::NoChecker => Err(McpError::CapabilityDenied(
                "No capability checker configured — tool access denied".to_string(),
            )),
        }
    }

    fn invoke_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
        token: &DelegationToken,
    ) -> Result<serde_json::Value, McpError> {
        // P1.1: Use unified verification instead of duplicated inline HMAC check
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        match verify_delegation_token(
            self.capability_checker.as_ref().map(|a| a.as_ref()),
            token,
            &token.delegated_to,
            DelegationResource::Tool,
            tool_name,
            DelegationAction::Execute,
            current_time,
        ) {
            VerificationOutcome::Valid => {}
            VerificationOutcome::InvalidSignature => {
                return Err(McpError::CapabilityDenied(
                    "Token signature verification failed".to_string(),
                ));
            }
            VerificationOutcome::Expired => {
                return Err(McpError::CapabilityDenied("Token is expired".to_string()));
            }
            VerificationOutcome::InsufficientAccess { resource_id, .. } => {
                return Err(McpError::CapabilityDenied(format!(
                    "Token does not authorize tool: {}",
                    resource_id
                )));
            }
            VerificationOutcome::NoChecker => {
                return Err(McpError::CapabilityDenied(
                    "No capability checker configured — tool invocation denied".to_string(),
                ));
            }
        }

        // Route through McpRuntime if available
        let (runtime, handle) = match (&self.mcp_runtime, &self.handle) {
            (Some(r), Some(h)) => (r, h),
            _ => {
                return Err(McpError::NoRuntime(
                    "No McpRuntime wired — call McpRuntimeAdapter::with_runtime() first"
                        .to_string(),
                ));
            }
        };

        // Resolve server_id for the tool, then invoke through RawMcpToolPort
        let server_id = handle
            .block_on(runtime.get_tool_info(tool_name))
            .map(|info| info.server_id)
            .unwrap_or_else(|| "unknown".to_string());

        let raw_port = hkask_mcp::RawMcpToolPort::new(runtime.as_ref().clone());
        match handle.block_on(hkask_types::ports::ToolPort::invoke(
            &raw_port, &server_id, tool_name, input, token,
        )) {
            Ok(value) => Ok(value),
            Err(hkask_types::ports::ToolPortError::NotFound(msg)) => {
                Err(McpError::ToolNotFound(msg))
            }
            Err(hkask_types::ports::ToolPortError::InvocationFailed(msg)) => {
                Err(McpError::InvocationFailed(msg))
            }
            Err(hkask_types::ports::ToolPortError::CapabilityDenied(msg)) => {
                Err(McpError::CapabilityDenied(msg))
            }
            Err(hkask_types::ports::ToolPortError::GasBudgetExceeded(msg)) => Err(
                McpError::InvocationFailed(format!("Energy budget exceeded: {}", msg)),
            ),
        }
    }

    fn resolve_tool_server(&self, tool_name: &str) -> Option<String> {
        let (runtime, handle) = (&self.mcp_runtime, &self.handle);
        match (runtime, handle) {
            (Some(r), Some(h)) => h
                .block_on(r.get_tool_info(tool_name))
                .map(|info| info.server_id),
            _ => None,
        }
    }
}
