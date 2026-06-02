//! Loop 4: Communication — dumb transport pipe
//!
//! send → observe delivery → confirm
//!
//! Communication moves messages between loops. It does NOT dampen,
//! throttle, or circuit-break — those are Cybernetics regulation
//! actions applied TO communication channels.
//!
//! Essential messenger function:
//! - 4.1 DISPATCH (GUARD+ROUTE) — priority-ordered message queuing

use crate::id::WebID;
use crate::loops::LoopId;
use std::fmt;

// =============================================================================
// TraceId — Cross-loop correlation identifier
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TraceId(pub uuid::Uuid);

impl TraceId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Self {
        Self(uuid::Uuid::parse_str(s).unwrap_or_else(|_| uuid::Uuid::new_v4()))
    }

    pub fn from_uuid(id: uuid::Uuid) -> Self {
        Self(id)
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// =============================================================================
// MessagePriority — Dispatch priority
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessagePriority {
    Critical,
    Warning,
    Info,
}

impl MessagePriority {
    pub fn order(&self) -> u8 {
        match self {
            MessagePriority::Critical => 0,
            MessagePriority::Warning => 1,
            MessagePriority::Info => 2,
        }
    }
}

impl fmt::Display for MessagePriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessagePriority::Critical => write!(f, "CRITICAL"),
            MessagePriority::Warning => write!(f, "WARNING"),
            MessagePriority::Info => write!(f, "INFO"),
        }
    }
}

// =============================================================================
// LoopOrigin — Source loop identification
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopOrigin {
    Inference,
    Episodic,
    Semantic,
    Communication,
    Curation,
    Cybernetics,
    External,
}

impl From<LoopId> for LoopOrigin {
    fn from(id: LoopId) -> Self {
        match id {
            LoopId::Inference => LoopOrigin::Inference,
            LoopId::Episodic => LoopOrigin::Episodic,
            LoopId::Semantic => LoopOrigin::Semantic,
            LoopId::Communication => LoopOrigin::Communication,
            LoopId::Curation => LoopOrigin::Curation,
            LoopId::Cybernetics => LoopOrigin::Cybernetics,
        }
    }
}

impl fmt::Display for LoopOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoopOrigin::Inference => write!(f, "inference"),
            LoopOrigin::Episodic => write!(f, "episodic"),
            LoopOrigin::Semantic => write!(f, "semantic"),
            LoopOrigin::Communication => write!(f, "communication"),
            LoopOrigin::Curation => write!(f, "curation"),
            LoopOrigin::Cybernetics => write!(f, "cybernetics"),
            LoopOrigin::External => write!(f, "external"),
        }
    }
}

// =============================================================================
// LoopPayload — Message content
// =============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopPayload {
    AlgedonicAlert {
        current: u64,
        threshold: u64,
        deficit: u64,
    },
    CyberneticsDirective {
        directive_type: String,
        target: WebID,
        parameters: serde_json::Value,
    },
    MemoryOperation {
        operation: String,
        data_category: String,
        data: serde_json::Value,
    },
    CapabilityChange {
        agent: WebID,
        change_type: String,
        details: serde_json::Value,
    },
}

// =============================================================================
// LoopMessage — Inter-loop communication unit
// =============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoopMessage {
    pub trace_id: TraceId,
    pub priority: MessagePriority,
    pub origin: LoopOrigin,
    pub payload: LoopPayload,
    pub target_loop: Option<LoopOrigin>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub sender: Option<WebID>,
}

impl LoopMessage {
    pub fn new(priority: MessagePriority, origin: LoopOrigin, payload: LoopPayload) -> Self {
        Self {
            trace_id: TraceId::new(),
            priority,
            origin,
            payload,
            target_loop: None,
            timestamp: chrono::Utc::now(),
            sender: None,
        }
    }

    pub fn critical(origin: LoopOrigin, payload: LoopPayload) -> Self {
        Self::new(MessagePriority::Critical, origin, payload)
    }

    pub fn warning(origin: LoopOrigin, payload: LoopPayload) -> Self {
        Self::new(MessagePriority::Warning, origin, payload)
    }

    #[must_use]
    pub fn with_target(mut self, target: LoopOrigin) -> Self {
        self.target_loop = Some(target);
        self
    }

    #[must_use]
    pub fn with_sender(mut self, sender: WebID) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn is_broadcast(&self) -> bool {
        self.target_loop.is_none()
    }

    pub fn is_directed(&self) -> bool {
        self.target_loop.is_some()
    }
}
