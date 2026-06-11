//! MCP Runtime Adapters — Concrete implementations of MCPRuntimePort
//
//! Two adapter types enforce the "make impossible states unrepresentable" principle:
//!
//! - `CapabilityOnlyAdapter` — can verify and grant capabilities but cannot invoke tools.
//!   Requires a `CapabilityChecker` at construction; tool invocation always returns
//!   `McpError::NoRuntime`.
//!
//! - `FullMcpAdapter` — can verify capabilities *and* dispatch tool invocations through
//!   a live `McpRuntime`. Requires `CapabilityChecker`, `McpRuntime`, *and* a tokio
//!   `Handle` at construction.

use crate::error::McpError;
use crate::ports::MCPRuntimePort;
use hkask_mcp::runtime::McpRuntime;
use hkask_types::{
    CapabilityChecker, DelegationAction, DelegationResource, DelegationToken, TOKEN_ERR_EXPIRED,
    TOKEN_ERR_INVALID_SIGNATURE, TOKEN_ERR_NO_CHECKER, VerificationOutcome,
    verify_delegation_token_now,
};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Capability-only adapter
// ---------------------------------------------------------------------------

/// Capability-only adapter for ACP token verification.
///
/// Can verify and grant capabilities but cannot invoke tools —
/// `invoke_tool` and `resolve_tool_server` always return errors.
///
/// Use this when you need token verification gate logic but no live
/// MCP server connections (e.g., in tests or lightweight embeds).
pub struct CapabilityOnlyAdapter {
    capability_checker: Arc<CapabilityChecker>,
}

impl CapabilityOnlyAdapter {
    /// Create a capability-only adapter with the given checker.
    ///
    /// Token verification (`grant_tool_access`, verification gate in
    /// `invoke_tool`) will use this checker. Tool invocation always
    /// fails with `McpError::NoRuntime`.
    pub fn new(checker: Arc<CapabilityChecker>) -> Self {
        Self {
            capability_checker: checker,
        }
    }
}

impl MCPRuntimePort for CapabilityOnlyAdapter {
    fn grant_tool_access(&self, token: DelegationToken) -> Result<(), McpError> {
        let token_id = token.id.clone();

        if token_id.is_empty() {
            return Err(McpError::InvalidToken("Token ID is empty".to_string()));
        }

        match verify_delegation_token_now(
            Some(self.capability_checker.as_ref()),
            &token,
            &token.delegated_to,
            DelegationResource::Tool,
            &token.resource_id,
            DelegationAction::Execute,
        ) {
            VerificationOutcome::Valid => Ok(()),
            VerificationOutcome::InvalidSignature => Err(McpError::InvalidToken(
                TOKEN_ERR_INVALID_SIGNATURE.to_string(),
            )),
            VerificationOutcome::Expired => Err(McpError::CapabilityDenied {
                resource: "token".to_string(),
                action: TOKEN_ERR_EXPIRED.to_string(),
            }),
            VerificationOutcome::InsufficientAccess { .. } => Err(McpError::CapabilityDenied {
                resource: token.resource_id.clone(),
                action: "execute".to_string(),
            }),
            // CapabilityOnlyAdapter always carries a checker; NoChecker
            // should never fire, but we handle it defensively.
            VerificationOutcome::NoChecker => Err(McpError::CapabilityDenied {
                resource: "token".to_string(),
                action: format!("{TOKEN_ERR_NO_CHECKER} — tool access denied"),
            }),
        }
    }

    fn invoke_tool(
        &self,
        _tool_name: &str,
        _input: serde_json::Value,
        _token: &DelegationToken,
    ) -> Result<serde_json::Value, McpError> {
        Err(McpError::NoRuntime(
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
    fn grant_tool_access(&self, token: DelegationToken) -> Result<(), McpError> {
        let token_id = token.id.clone();

        if token_id.is_empty() {
            return Err(McpError::InvalidToken("Token ID is empty".to_string()));
        }

        match verify_delegation_token_now(
            Some(self.capability_checker.as_ref()),
            &token,
            &token.delegated_to,
            DelegationResource::Tool,
            &token.resource_id,
            DelegationAction::Execute,
        ) {
            VerificationOutcome::Valid => Ok(()),
            VerificationOutcome::InvalidSignature => Err(McpError::InvalidToken(
                TOKEN_ERR_INVALID_SIGNATURE.to_string(),
            )),
            VerificationOutcome::Expired => Err(McpError::CapabilityDenied {
                resource: "token".to_string(),
                action: TOKEN_ERR_EXPIRED.to_string(),
            }),
            VerificationOutcome::InsufficientAccess { .. } => Err(McpError::CapabilityDenied {
                resource: token.resource_id.clone(),
                action: "execute".to_string(),
            }),
            VerificationOutcome::NoChecker => Err(McpError::CapabilityDenied {
                resource: "token".to_string(),
                action: format!("{TOKEN_ERR_NO_CHECKER} — tool access denied"),
            }),
        }
    }

    fn invoke_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
        token: &DelegationToken,
    ) -> Result<serde_json::Value, McpError> {
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
                return Err(McpError::CapabilityDenied {
                    resource: "token".to_string(),
                    action: TOKEN_ERR_INVALID_SIGNATURE.to_string(),
                });
            }
            VerificationOutcome::Expired => {
                return Err(McpError::CapabilityDenied {
                    resource: "token".to_string(),
                    action: TOKEN_ERR_EXPIRED.to_string(),
                });
            }
            VerificationOutcome::InsufficientAccess { resource_id, .. } => {
                return Err(McpError::CapabilityDenied {
                    resource: resource_id,
                    action: "execute".to_string(),
                });
            }
            VerificationOutcome::NoChecker => {
                return Err(McpError::CapabilityDenied {
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
            .unwrap_or_else(|| "unknown".to_string());

        let raw_port = hkask_mcp::RawMcpToolPort::new(self.mcp_runtime.as_ref().clone());
        match self.handle.block_on(hkask_types::ports::ToolPort::invoke(
            &raw_port, &server_id, tool_name, input, token,
        )) {
            Ok(value) => Ok(value),
            Err(hkask_types::ports::ToolPortError::NotFound(msg)) => {
                Err(McpError::ToolNotFound(msg))
            }
            Err(hkask_types::ports::ToolPortError::InvocationFailed(msg)) => {
                Err(McpError::InvocationFailed(Box::new(
                    hkask_types::ports::ToolPortError::InvocationFailed(msg),
                )))
            }
            Err(hkask_types::ports::ToolPortError::CapabilityDenied(msg)) => {
                Err(McpError::CapabilityDenied {
                    resource: "tool".to_string(),
                    action: msg,
                })
            }
            Err(hkask_types::ports::ToolPortError::EnergyBudgetExceeded(msg)) => {
                Err(McpError::InvocationFailed(Box::new(
                    hkask_types::ports::ToolPortError::EnergyBudgetExceeded(msg),
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

// ---------------------------------------------------------------------------
// Backward-compatible type alias (deprecated)
// ---------------------------------------------------------------------------
