//! MCP governance — Cybernetics loop concerns
//!
//! OCP capability verification, token lifecycle, and revocation.
//! These are Cybernetics (meta) concerns: governing who can invoke what.
//!
//! Split from `McpDispatcher` to enforce the authority DAG:
//! Cybernetics governs Communication. The governor holds the capability
//! checker and revoked token set; the dispatcher holds the transport runtime.

use hkask_types::{
    CapabilityChecker, DelegationAction, DelegationResource, DelegationToken, WebID,
};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cybernetics governor for MCP capability governance.
///
/// Owns the capability checker and revoked token set.
/// All governance decisions flow through this struct.
pub(crate) struct McpGovernor {
    /// Capability checker for OCP
    capability_checker: Arc<CapabilityChecker>,
    /// Revoked token IDs
    revoked_tokens: Arc<RwLock<HashSet<String>>>,
}

impl McpGovernor {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            capability_checker: Arc::new(CapabilityChecker::new(secret)),
            revoked_tokens: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Issue capability token to a bot
    pub fn issue_capability(&self, tool_name: String, from: WebID, to: WebID) -> DelegationToken {
        self.capability_checker.grant_tool(tool_name, from, to)
    }

    /// Check if a token has been revoked
    pub async fn is_token_revoked(&self, token_id: &str) -> bool {
        let revoked = self.revoked_tokens.read().await;
        revoked.contains(token_id)
    }

    /// Cryptographically verify a capability token
    pub fn verify_token(&self, token: &DelegationToken) -> bool {
        self.capability_checker.verify(token)
    }

    /// Check if a token authorizes a specific tool/action
    pub fn token_is_valid_for(&self, token: &DelegationToken, tool_name: &str) -> bool {
        token.is_valid_for(
            DelegationResource::Tool,
            tool_name,
            DelegationAction::Execute,
        )
    }

    /// Full governance check for a tool invocation with a capability token.
    ///
    /// Returns `Ok(())` if the invocation is authorized, `Err(reason)` otherwise.
    pub async fn authorize(&self, token: &DelegationToken, tool_name: &str) -> Result<(), String> {
        if !self.verify_token(token) {
            return Err(format!(
                "Invalid capability token signature for tool: {}",
                tool_name
            ));
        }

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        if token.is_expired(current_time) {
            return Err(format!("Expired capability token for tool: {}", tool_name));
        }

        if !self.token_is_valid_for(token, tool_name) {
            return Err(format!(
                "Capability token does not authorize tool: {}",
                tool_name
            ));
        }

        if self.is_token_revoked(&token.id).await {
            return Err(format!("Revoked capability token for tool: {}", tool_name));
        }

        Ok(())
    }
}
