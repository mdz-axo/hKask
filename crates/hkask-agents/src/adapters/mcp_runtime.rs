//! MCP Runtime Adapters — Concrete implementations of MCPRuntimePort
//
//! Two adapter types enforce the "make impossible states unrepresentable" principle:
//!
//! - `CapabilityOnlyAdapter` — can verify and grant capabilities but cannot invoke tools.
//!   Requires a `CapabilityChecker` at construction; tool invocation always returns
//!   `AgentMcpError::NoRuntime`.
//!
//! - `FullMcpAdapter` — can verify capabilities *and* dispatch tool invocations through
//!   a live `McpRuntime`. Requires `CapabilityChecker`, `McpRuntime`, *and* a tokio
//!   `Handle` at construction.

use crate::error::AgentMcpError;
use crate::ports::MCPRuntimePort;
use hkask_capability::{
    CapabilityChecker, DelegationAction, DelegationResource, DelegationToken, TOKEN_ERR_EXPIRED,
    TOKEN_ERR_INVALID_SIGNATURE, TOKEN_ERR_NO_CHECKER, VerificationOutcome,
    verify_delegation_token_now,
};
use hkask_mcp::runtime::McpRuntime;
use std::sync::Arc;

/// Verify a delegation token for tool access grant.
///
/// Shared by `CapabilityOnlyAdapter` and `FullMcpAdapter` — eliminates
/// the 35-line duplicate verification block that was identical in both
/// `grant_tool_access` implementations.
fn verify_grant_access(
    checker: &CapabilityChecker,
    token: &DelegationToken,
) -> Result<(), AgentMcpError> {
    if token.id.is_empty() {
        return Err(AgentMcpError::InvalidToken("Token ID is empty".to_string()));
    }

    match verify_delegation_token_now(
        Some(checker),
        token,
        &token.delegated_to,
        DelegationResource::Tool,
        &token.resource_id,
        DelegationAction::Execute,
    ) {
        VerificationOutcome::Valid => Ok(()),
        VerificationOutcome::InvalidSignature => Err(AgentMcpError::InvalidToken(
            TOKEN_ERR_INVALID_SIGNATURE.to_string(),
        )),
        VerificationOutcome::Expired => Err(AgentMcpError::CapabilityDenied {
            resource: "token".to_string(),
            action: TOKEN_ERR_EXPIRED.to_string(),
        }),
        VerificationOutcome::InsufficientAccess { .. } => Err(AgentMcpError::CapabilityDenied {
            resource: token.resource_id.clone(),
            action: "execute".to_string(),
        }),
        VerificationOutcome::NoChecker => Err(AgentMcpError::CapabilityDenied {
            resource: "token".to_string(),
            action: format!("{TOKEN_ERR_NO_CHECKER} — tool access denied"),
        }),
    }
}

// ---------------------------------------------------------------------------
// Capability-only adapter
// ---------------------------------------------------------------------------

/// Capability-only adapter for A2A token verification.
///
/// Can verify and grant capabilities but cannot invoke tools —
/// \[DECLARATIVE\] `invoke_tool` and `resolve_tool_server` always return errors. (P4 — Clear Boundaries).
///
/// Use this when you need token verification gate logic but no live
/// MCP server connections (e.g., in tests or lightweight embeds).
pub struct CapabilityOnlyAdapter {
    capability_checker: Arc<CapabilityChecker>,
}

impl CapabilityOnlyAdapter {
    /// Create a capability-only adapter with the given checker.
    ///
    /// expect: "Agent interactions are gated by OCAP boundaries"
    /// \[P4\] Motivating: Clear Boundaries — capability-only adapter gates tools without runtime
    /// pre:  `checker` is a valid `Arc<CapabilityChecker>`.
    /// post: Returns a `CapabilityOnlyAdapter` with the given checker;
    ///       tool invocation will always fail with `AgentMcpError::NoRuntime`.
    pub fn new(checker: Arc<CapabilityChecker>) -> Self {
        Self {
            capability_checker: checker,
        }
    }
}

impl MCPRuntimePort for CapabilityOnlyAdapter {
    fn grant_tool_access(&self, token: DelegationToken) -> Result<(), AgentMcpError> {
        verify_grant_access(&self.capability_checker, &token)
    }

    fn invoke_tool(
        &self,
        _tool_name: &str,
        _input: serde_json::Value,
        _token: &DelegationToken,
    ) -> Result<serde_json::Value, AgentMcpError> {
        Err(AgentMcpError::NoRuntime(
            "CapabilityOnlyAdapter has no runtime — use FullMcpAdapter for tool invocation"
                .to_string(),
        ))
    }

    fn resolve_tool_server(&self, _tool_name: &str) -> Option<String> {
        None
    }
}

// ---------------------------------------------------------------------------
// Full MCP adapter
// ---------------------------------------------------------------------------

/// Full MCP adapter with both capability checking and tool dispatch.
///
/// Routes tool invocations through `McpRuntime`'s live MCP server
/// connections. Requires a `CapabilityChecker`, `McpRuntime`, and
/// tokio `Handle` at construction — all three are mandatory so that
/// every method can succeed.
pub struct FullMcpAdapter {
    capability_checker: Arc<CapabilityChecker>,
    mcp_runtime: Arc<McpRuntime>,
    handle: tokio::runtime::Handle,
}

impl FullMcpAdapter {
    /// Create a full MCP adapter.
    ///
    /// All three arguments are required: the checker for token
    /// verification, the runtime for MCP dispatch, and a tokio
    /// handle for bridging sync→async calls.
    ///
    /// expect: "Agent interactions are gated by OCAP boundaries"
    /// \[P4\] Motivating: Clear Boundaries — full adapter combines capability checker + MCP runtime
    /// pre:  `checker` is a valid `Arc<CapabilityChecker>`; `runtime` is
    ///       a valid `Arc<McpRuntime>`; `handle` is a valid tokio runtime
    ///       handle.
    /// post: Returns a `FullMcpAdapter` with all three components set.
    pub fn new(
        checker: Arc<CapabilityChecker>,
        runtime: Arc<McpRuntime>,
        handle: tokio::runtime::Handle,
    ) -> Self {
        Self {
            capability_checker: checker,
            mcp_runtime: runtime,
            handle,
        }
    }
}

impl MCPRuntimePort for FullMcpAdapter {
    fn grant_tool_access(&self, token: DelegationToken) -> Result<(), AgentMcpError> {
        verify_grant_access(&self.capability_checker, &token)
    }

    fn invoke_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
        token: &DelegationToken,
    ) -> Result<serde_json::Value, AgentMcpError> {
        // P1.1: Use unified verification instead of duplicated inline HMAC check
        match verify_delegation_token_now(
            Some(self.capability_checker.as_ref()),
            token,
            &token.delegated_to,
            DelegationResource::Tool,
            tool_name,
            DelegationAction::Execute,
        ) {
            VerificationOutcome::Valid => {}
            VerificationOutcome::InvalidSignature => {
                return Err(AgentMcpError::CapabilityDenied {
                    resource: "token".to_string(),
                    action: TOKEN_ERR_INVALID_SIGNATURE.to_string(),
                });
            }
            VerificationOutcome::Expired => {
                return Err(AgentMcpError::CapabilityDenied {
                    resource: "token".to_string(),
                    action: TOKEN_ERR_EXPIRED.to_string(),
                });
            }
            VerificationOutcome::InsufficientAccess { resource_id, .. } => {
                return Err(AgentMcpError::CapabilityDenied {
                    resource: resource_id,
                    action: "execute".to_string(),
                });
            }
            VerificationOutcome::NoChecker => {
                return Err(AgentMcpError::CapabilityDenied {
                    resource: "token".to_string(),
                    action: format!("{TOKEN_ERR_NO_CHECKER} — tool invocation denied"),
                });
            }
        }

        // Resolve server_id for the tool, then invoke through RawMcpToolPort
        let server_id = self
            .handle
            .block_on(self.mcp_runtime.get_tool_info(tool_name))
            .map(|info| info.server_id)
            .ok_or_else(|| AgentMcpError::CapabilityDenied {
                resource: tool_name.to_string(),
                action: format!(
                    "Tool '{}' not registered on any MCP server — invocation denied",
                    tool_name
                ),
            })?;

        let raw_port = hkask_mcp::RawMcpToolPort::new(self.mcp_runtime.as_ref().clone());
        match self.handle.block_on(hkask_ports::ToolPort::invoke(
            &raw_port, &server_id, tool_name, input, token,
        )) {
            Ok(value) => Ok(value),
            Err(hkask_ports::ToolPortError::NotFound(msg)) => Err(AgentMcpError::ToolNotFound(msg)),
            Err(hkask_ports::ToolPortError::InvocationFailed(msg)) => {
                Err(AgentMcpError::InvocationFailed(Box::new(
                    hkask_ports::ToolPortError::InvocationFailed(msg),
                )))
            }
            Err(hkask_ports::ToolPortError::CapabilityDenied(msg)) => {
                Err(AgentMcpError::CapabilityDenied {
                    resource: "tool".to_string(),
                    action: msg,
                })
            }
            Err(hkask_ports::ToolPortError::EnergyBudgetExceeded(msg)) => {
                Err(AgentMcpError::InvocationFailed(Box::new(
                    hkask_ports::ToolPortError::EnergyBudgetExceeded(msg),
                )))
            }
        }
    }

    fn resolve_tool_server(&self, tool_name: &str) -> Option<String> {
        self.handle
            .block_on(self.mcp_runtime.get_tool_info(tool_name))
            .map(|info| info.server_id)
    }
}
