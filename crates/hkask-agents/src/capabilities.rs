//! Unified Agent Capabilities
//!
//! Collapsed from the former `BotCapabilities` and `ReplicantCapabilities` into
//! a single `AgentCapabilities` type with a structured `MemoryAccess` sub-struct.
//!
//! Bots previously had a single `can_access_memory` flag; this now maps to
//! `MemoryAccess { can_access_episodic: true, can_access_semantic: true }`.
//! Replicants already had separate episodic/semantic flags, which map directly.

use serde::{Deserialize, Serialize};

/// Memory access permissions for an agent.
///
/// Separates episodic (private, session-scoped) from semantic (public, long-term)
/// memory access, replacing the former undifferentiated `can_access_memory` flag.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryAccess {
    /// Can access episodic memory (private, session-scoped)
    pub can_access_episodic: bool,
    /// Can access semantic memory (public, long-term)
    pub can_access_semantic: bool,
}

impl MemoryAccess {
    /// Grant access to both episodic and semantic memory.
    pub fn full() -> Self {
        Self {
            can_access_episodic: true,
            can_access_semantic: true,
        }
    }

    /// Grant episodic memory access only.
    pub fn episodic_only() -> Self {
        Self {
            can_access_episodic: true,
            can_access_semantic: false,
        }
    }

    /// Grant semantic memory access only.
    pub fn semantic_only() -> Self {
        Self {
            can_access_episodic: false,
            can_access_semantic: true,
        }
    }

    /// Check if any memory access is granted.
    pub fn any(&self) -> bool {
        self.can_access_episodic || self.can_access_semantic
    }
}

/// Unified capabilities for all agent types (bots and replicants).
///
/// Replaces the former `BotCapabilities` and `ReplicantCapabilities` with a
/// single type that uses `MemoryAccess` to differentiate episodic vs semantic
/// memory permissions.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentCapabilities {
    /// Can invoke MCP tools
    pub can_invoke_tools: bool,
    /// Memory access permissions (episodic vs semantic)
    pub memory_access: MemoryAccess,
    /// Can dispatch templates
    pub can_dispatch_templates: bool,
    /// Can escalate to curator
    pub can_escalate: bool,
}

impl AgentCapabilities {
    /// Create capabilities with full memory access (both episodic and semantic).
    /// Maps to the former `BotCapabilities.can_access_memory = true`.
    pub fn with_full_memory() -> Self {
        Self {
            can_invoke_tools: false,
            memory_access: MemoryAccess::full(),
            can_dispatch_templates: false,
            can_escalate: false,
        }
    }

    /// Create capabilities with episodic memory access only.
    pub fn with_episodic_memory() -> Self {
        Self {
            can_invoke_tools: false,
            memory_access: MemoryAccess::episodic_only(),
            can_dispatch_templates: false,
            can_escalate: false,
        }
    }

    /// Create capabilities with semantic memory access only.
    pub fn with_semantic_memory() -> Self {
        Self {
            can_invoke_tools: false,
            memory_access: MemoryAccess::semantic_only(),
            can_dispatch_templates: false,
            can_escalate: false,
        }
    }
}
