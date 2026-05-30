//! Bot Agent Implementation
//!
//! Bots are process-execution agents that operate autonomously via A2A communication.
//! They handle machine-to-machine (M2M) tasks with OCAP-gated capabilities.

use hkask_types::{BotID, WebID};
use serde::{Deserialize, Serialize};

use crate::capabilities::AgentCapabilities;

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
    pub capabilities: AgentCapabilities,
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
            capabilities: AgentCapabilities::default(),
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

    /// Enable full memory access (both episodic and semantic).
    ///
    /// For bots, memory access grants both episodic and semantic memory,
    /// consistent with the former `can_access_memory` flag.
    pub fn enable_memory(&mut self) {
        self.capabilities.memory_access = crate::capabilities::MemoryAccess::full();
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
