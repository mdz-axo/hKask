//! Bot Agent Implementation
//!
//! Bots are process-execution agents that operate autonomously via A2A communication.
//! They handle machine-to-machine (M2M) tasks with OCAP-gated capabilities.

use hkask_types::{BotID, WebID};
use serde::{Deserialize, Serialize};

/// Bot agent
///
/// Bots are public/shared agents focused on process execution.
/// They operate autonomously within their capability boundaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bot {
    /// Bot unique identifier
    pub id: BotID,
    /// Bot WebID for ACP registration
    pub webid: WebID,
    /// Bot display name
    pub name: String,
    /// Bot description/persona
    pub description: String,
    /// Whether bot is active
    pub active: bool,
    /// Bot capabilities
    pub capabilities: BotCapabilities,
}

/// Bot capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BotCapabilities {
    /// Can invoke MCP tools
    pub can_invoke_tools: bool,
    /// Can access memory
    pub can_access_memory: bool,
    /// Can dispatch templates
    pub can_dispatch_templates: bool,
    /// Can escalate to curator
    pub can_escalate: bool,
}

impl Bot {
    /// Create new bot
    pub fn new(name: String, description: String) -> Self {
        let webid = WebID::new();
        Self {
            id: BotID::new(),
            webid,
            name,
            description,
            active: false,
            capabilities: BotCapabilities::default(),
        }
    }

    /// Activate bot
    pub fn activate(&mut self) {
        self.active = true;
    }

    /// Deactivate bot
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Check if bot is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Enable tool invocation
    pub fn enable_tools(&mut self) {
        self.capabilities.can_invoke_tools = true;
    }

    /// Enable memory access
    pub fn enable_memory(&mut self) {
        self.capabilities.can_access_memory = true;
    }

    /// Enable template dispatch
    pub fn enable_templates(&mut self) {
        self.capabilities.can_dispatch_templates = true;
    }

    /// Enable escalation
    pub fn enable_escalation(&mut self) {
        self.capabilities.can_escalate = true;
    }
}

impl Default for Bot {
    fn default() -> Self {
        Self::new("unnamed-bot".to_string(), "A bot agent".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bot_creation() {
        let bot = Bot::new("test-bot".to_string(), "Test bot".to_string());
        assert!(!bot.is_active());
        assert_eq!(bot.name, "test-bot");
    }

    #[test]
    fn test_bot_activation() {
        let mut bot = Bot::default();
        bot.activate();
        assert!(bot.is_active());
        
        bot.deactivate();
        assert!(!bot.is_active());
    }

    #[test]
    fn test_bot_capabilities() {
        let mut bot = Bot::default();
        
        bot.enable_tools();
        assert!(bot.capabilities.can_invoke_tools);
        
        bot.enable_memory();
        assert!(bot.capabilities.can_access_memory);
        
        bot.enable_templates();
        assert!(bot.capabilities.can_dispatch_templates);
        
        bot.enable_escalation();
        assert!(bot.capabilities.can_escalate);
    }
}
