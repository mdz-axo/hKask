//! MCP governance — Cybernetics loop concerns
//!
//! OCP capability verification, token lifecycle, and revocation.
//! These are Cybernetics (meta) concerns: governing who can invoke what.
//!
//! Split from `McpDispatcher` to enforce the authority DAG:
//! Cybernetics governs Communication. The governor holds the capability
//! checker and revoked token set; the dispatcher holds the transport runtime.
//!
//! **Architectural note (V3):** This struct lives in `hkask-mcp` (Communication)
//! but performs Cybernetics concerns. The correct location is `hkask-cns`
//! or a shared governance module. Moving it requires refactoring crate
//! dependencies — `McpDispatcher` would need to take a `Box<dyn CapabilityGovernor>`
//! trait object instead of owning `McpGovernor` directly. This is deferred
//! until a second governor implementation exists that justifies the trait split.

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
