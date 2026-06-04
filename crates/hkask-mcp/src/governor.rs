//! MCP governance — Cybernetics loop concerns
//!
//! OCP capability verification, token lifecycle, and revocation.
//! These are Cybernetics (meta) concerns: governing who can invoke what.
//!
//! Split from `McpDispatcher` to enforce the authority DAG:
//! Cybernetics governs Communication. The governor holds the capability
//! checker and revoked token set; the dispatcher holds the transport runtime.

use hkask_types::{CapabilityChecker, DelegationToken, WebID};
use std::sync::Arc;

/// Cybernetics governor for MCP capability governance.
///
/// Owns the capability checker and revoked token set.
/// All governance decisions flow through this struct.
pub(crate) struct McpGovernor {
    /// Capability checker for OCP
    capability_checker: Arc<CapabilityChecker>,
}

impl McpGovernor {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            capability_checker: Arc::new(CapabilityChecker::new(secret)),
        }
    }

    /// Issue capability token to a bot
    pub fn issue_capability(&self, tool_name: String, from: WebID, to: WebID) -> DelegationToken {
        self.capability_checker.grant_tool(tool_name, from, to)
    }
}
