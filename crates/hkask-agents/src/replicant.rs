//! Replicant Agent Implementation
//!
//! Replicants are human-assistance agents that operate via H2A (Human-to-Agent) communication.
//! They assist users with tasks while maintaining strict OCAP boundaries.

use hkask_types::WebID;
use serde::{Deserialize, Serialize};

use crate::capabilities::AgentCapabilities;

/// Replicant agent
///
/// Replicants are private/episodic agents focused on human assistance.
/// They operate under direct user supervision with OCAP-gated capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replicant {
    /// Replicant WebID for ACP registration
    pub webid: WebID,
    /// Replicant display name
    pub name: String,
    /// Replicant description/persona
    pub description: String,
    /// Owner WebID (user who created this replicant)
    pub owner: WebID,
    /// Whether replicant is active
    pub active: bool,
    /// Replicant capabilities
    pub capabilities: AgentCapabilities,
}

/// Replicant operational capabilities (deprecated — use [`AgentCapabilities`]).
///
/// Migrated to `AgentCapabilities` which uses `MemoryAccess` for structured
/// episodic/semantic memory permissions. The former `can_access_episodic` and
/// `can_access_semantic` fields now live in `MemoryAccess`.
#[deprecated(
    since = "0.21.0",
    note = "Use `AgentCapabilities` with `MemoryAccess` instead. `can_access_episodic` maps to `memory_access.can_access_episodic`, `can_access_semantic` maps to `memory_access.can_access_semantic`."
)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplicantCapabilities {
    /// Can invoke MCP tools
    pub can_invoke_tools: bool,
    /// Can access episodic memory (private)
    pub can_access_episodic: bool,
    /// Can access semantic memory (public)
    pub can_access_semantic: bool,
    /// Can dispatch templates
    pub can_dispatch_templates: bool,
    /// Can escalate to curator
    pub can_escalate: bool,
}

impl Replicant {
    /// Create new replicant
    pub fn new(name: String, description: String, owner: WebID) -> Self {
        Self {
            webid: WebID::new(),
            name,
            description,
            owner,
            active: false,
            capabilities: AgentCapabilities::default(),
        }
    }

    /// Activate replicant
    pub fn activate(&mut self) {
        self.active = true;
    }

    /// Deactivate replicant
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Check if replicant is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Enable tool invocation
    pub fn enable_tools(&mut self) {
        self.capabilities.can_invoke_tools = true;
    }

    /// Enable episodic memory access (private)
    pub fn enable_episodic_memory(&mut self) {
        self.capabilities.memory_access.can_access_episodic = true;
    }

    /// Enable semantic memory access (public)
    pub fn enable_semantic_memory(&mut self) {
        self.capabilities.memory_access.can_access_semantic = true;
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

impl Default for Replicant {
    fn default() -> Self {
        let owner = WebID::new();
        Self::new(
            "unnamed-replicant".to_string(),
            "A replicant agent".to_string(),
            owner,
        )
    }
}

#[allow(deprecated)]
impl From<ReplicantCapabilities> for AgentCapabilities {
    fn from(caps: ReplicantCapabilities) -> Self {
        Self {
            can_invoke_tools: caps.can_invoke_tools,
            memory_access: crate::capabilities::MemoryAccess {
                can_access_episodic: caps.can_access_episodic,
                can_access_semantic: caps.can_access_semantic,
            },
            can_dispatch_templates: caps.can_dispatch_templates,
            can_escalate: caps.can_escalate,
        }
    }
}

#[allow(deprecated)]
impl From<AgentCapabilities> for ReplicantCapabilities {
    fn from(caps: AgentCapabilities) -> Self {
        Self {
            can_invoke_tools: caps.can_invoke_tools,
            can_access_episodic: caps.memory_access.can_access_episodic,
            can_access_semantic: caps.memory_access.can_access_semantic,
            can_dispatch_templates: caps.can_dispatch_templates,
            can_escalate: caps.can_escalate,
        }
    }
}
