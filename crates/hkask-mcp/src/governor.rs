//! MCP governance — Cybernetics loop concerns
//!
//! OCP capability verification, token lifecycle, and bot capability registry.
//! These are Cybernetics (meta) concerns: governing who can invoke what.
//!
//! Split from `McpDispatcher` to enforce the authority DAG:
//! Cybernetics governs Communication. The governor holds the capability
//! registry; the dispatcher holds the transport runtime.

use hkask_types::{
    BotCapabilities, CapabilityAction, CapabilityChecker, CapabilityResource, CapabilityToken,
    WebID,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

/// Cybernetics governor for MCP capability governance.
///
/// Owns the capability checker, bot capabilities registry, and revoked
/// token set. All governance decisions flow through this struct.
pub struct McpGovernor {
    /// Capability checker for OCP
    capability_checker: Arc<CapabilityChecker>,
    /// Bot capabilities registry
    bot_capabilities: Arc<RwLock<std::collections::HashMap<WebID, BotCapabilities>>>,
    /// Revoked token IDs
    revoked_tokens: Arc<RwLock<std::collections::HashSet<String>>>,
}

impl McpGovernor {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            capability_checker: Arc::new(CapabilityChecker::new(secret)),
            bot_capabilities: Arc::new(RwLock::new(std::collections::HashMap::new())),
            revoked_tokens: Arc::new(RwLock::new(std::collections::HashSet::new())),
        }
    }

    /// Register bot capabilities
    pub async fn register_bot_capabilities(&self, caps: BotCapabilities) {
        let mut capabilities = self.bot_capabilities.write().await;
        capabilities.insert(caps.bot_id, caps);
    }

    /// Issue capability token to a bot
    pub fn issue_capability(&self, tool_name: String, from: WebID, to: WebID) -> CapabilityToken {
        self.capability_checker.grant_tool(tool_name, from, to)
    }

    /// Revoke a capability token by ID
    pub async fn revoke_token(&self, token_id: String) {
        let mut revoked = self.revoked_tokens.write().await;
        revoked.insert(token_id);
    }

    /// Check if a token has been revoked
    pub async fn is_token_revoked(&self, token_id: &str) -> bool {
        let revoked = self.revoked_tokens.read().await;
        revoked.contains(token_id)
    }

    /// Cryptographically verify a capability token
    pub fn verify_token(&self, token: &CapabilityToken) -> bool {
        self.capability_checker.verify(token)
    }

    /// Check if a token authorizes a specific tool/action
    pub fn token_is_valid_for(&self, token: &CapabilityToken, tool_name: &str) -> bool {
        token.is_valid_for(
            CapabilityResource::Tool,
            tool_name,
            CapabilityAction::Execute,
        )
    }

    /// Check if a bot has a string-based capability (legacy fallback)
    pub async fn check_bot_capability(&self, bot_id: &WebID, tool_name: &str) -> bool {
        let capabilities = self.bot_capabilities.read().await;
        if let Some(caps) = capabilities.get(bot_id) {
            caps.has_capability(tool_name)
        } else {
            false
        }
    }

    /// Full governance check for a tool invocation with a capability token.
    ///
    /// Returns `Ok(())` if the invocation is authorized, `Err(reason)` otherwise.
    pub async fn authorize(&self, token: &CapabilityToken, tool_name: &str) -> Result<(), String> {
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

    /// Legacy authorization check (no token, bot-capabilities string match).
    pub async fn authorize_legacy(&self, bot_id: &WebID, tool_name: &str) -> Result<(), String> {
        warn!(
            target: "hkask.ocap",
            bot_id = ?bot_id,
            tool_name = %tool_name,
            "No capability token provided; falling back to bot-capabilities check"
        );

        if self.check_bot_capability(bot_id, tool_name).await {
            Ok(())
        } else {
            Err(format!(
                "Bot {:?} lacks capability for tool: {}",
                bot_id, tool_name
            ))
        }
    }

    /// Get the capability checker (for token minting from CLI)
    pub fn capability_checker(&self) -> &Arc<CapabilityChecker> {
        &self.capability_checker
    }
}
