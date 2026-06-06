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
// WorkerKind — Non-governing worker identifiers
// =============================================================================

/// Worker kinds — specialized message handlers that are NOT governing loops.
///
/// Workers operate within a parent loop. They have no authority rank
/// and cannot be targeted by regulatory actions (Throttle, CircuitBreak, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerKind {
    /// Metacognition worker within Curation (Loop 5)
    Metacognition,
    /// Tool dispatch worker within Communication (Loop 4)
    ToolDispatch,
}

impl fmt::Display for WorkerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkerKind::Metacognition => write!(f, "metacognition"),
            WorkerKind::ToolDispatch => write!(f, "tool_dispatch"),
        }
    }
}

// =============================================================================
// DispatchTarget — Loop or Worker target
// =============================================================================

/// Target for inter-loop/worker messages.
///
/// Loops (LoopId) are governing entities with authority rank.
/// Workers (WorkerKind) are specialized handlers without authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchTarget {
    Loop(LoopId),
    Worker(WorkerKind),
}

impl fmt::Display for DispatchTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DispatchTarget::Loop(id) => write!(f, "{}", id),
            DispatchTarget::Worker(w) => write!(f, "{}", w),
        }
    }
}

impl From<LoopId> for DispatchTarget {
    fn from(id: LoopId) -> Self {
        DispatchTarget::Loop(id)
    }
}

impl From<WorkerKind> for DispatchTarget {
    fn from(w: WorkerKind) -> Self {
        DispatchTarget::Worker(w)
    }
}

// =============================================================================
// TraceId — Cross-loop correlation identifier
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TraceId(pub uuid::Uuid);

impl TraceId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    pub fn from_uuid(id: uuid::Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl std::str::FromStr for TraceId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        uuid::Uuid::parse_str(s).map(TraceId)
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
// LoopPayload — Message content
// =============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopPayload {
    /// Algedonic alert from Cybernetics (variety deficit escalation).
    AlgedonicAlert {
        current: u64,
        threshold: u64,
        deficit: u64,
    },
    /// Directive from Curation to Cybernetics.
    ///
    /// Origin: Curation (Loop 5). Consumed by: Cybernetics (Loop 6).
    /// Per the authority DAG: Curation → Cybernetics.
    CurationDirective {
        directive_type: String,
        target: WebID,
        parameters: serde_json::Value,
    },
    /// Regulation action from Cybernetics to a domain loop.
    ///
    /// Origin: Cybernetics (Loop 6). Consumed by: domain loops (1, 2a, 2b, 4).
    /// Per the authority DAG: Cybernetics → {Inference, Episodic, Semantic, Communication}.
    CyberneticsRegulation {
        regulation_type: String,
        target: WebID,
        parameters: serde_json::Value,
    },
    /// Tool gas consumption report from GovernedTool to Cybernetics.
    ///
    /// Origin: GovernedTool (within Cybernetics membrane). Consumed by: Cybernetics (Loop 6).
    /// Enables per-tool and per-agent energy consumption tracking in the sense phase.
    ToolConsumption {
        tool_name: String,
        agent: WebID,
        gas_cost: u64,
        success: bool,
    },
    /// Goal state transition notification for Curation Loop.
    ///
    /// Origin: GoalStore (domain). Consumed by: Curation (Loop 5).
    /// Enables Curation to detect goal stalemate, priority inversion, capability expiry.
    GoalTransition {
        goal_id: String,
        from_state: String,
        to_state: String,
        agent: WebID,
    },
    /// Tool invocation request routed through the Communication Loop.
    ///
    /// Origin: GovernedTool (within Cybernetics membrane). Consumed by: tool worker.
    /// When loop-routed tool dispatch is enabled, GovernedTool sends the invocation
    /// as a LoopMessage through the Communication Loop rather than calling
    /// ToolPort::invoke() directly. This provides cross-loop traceability (TraceId),
    /// priority-aware ordering, and delivery confirmation.
    ToolInvocation {
        trace_id: TraceId,
        server: String,
        tool: String,
        args: serde_json::Value,
        agent: WebID,
    },
    /// Tool invocation result returned through the Communication Loop.
    ///
    /// Origin: tool worker. Consumed by: the loop that issued the ToolInvocation.
    /// Carries the result (or error) of a loop-routed tool invocation back to the caller.
    ToolResult {
        trace_id: TraceId,
        server: String,
        tool: String,
        result: serde_json::Value,
        success: bool,
        gas_cost: u64,
        agent: WebID,
    },
    /// Spec drift alert from DefaultSpecCurator when drift exceeds threshold.
    ///
    /// Origin: DefaultSpecCurator (domain). Consumed by: Curation (Loop 5).
    /// Enables Curation to sense spec-tool coherence failures without
    /// relying on the NuEvent store as the sole pathway.
    SpecDriftAlert {
        spec_id: String,
        drift_magnitude: f64,
        drift_threshold: f64,
        missing_verbs: Vec<String>,
    },
}

// =============================================================================
// LoopMessage — Inter-loop communication unit
// =============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoopMessage {
    pub trace_id: TraceId,
    pub priority: MessagePriority,
    pub origin: LoopId,
    pub payload: LoopPayload,
    pub target_loop: Option<DispatchTarget>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub sender: Option<WebID>,
}

impl LoopMessage {
    pub fn new(priority: MessagePriority, origin: LoopId, payload: LoopPayload) -> Self {
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

    pub fn critical(origin: LoopId, payload: LoopPayload) -> Self {
        Self::new(MessagePriority::Critical, origin, payload)
    }

    pub fn warning(origin: LoopId, payload: LoopPayload) -> Self {
        Self::new(MessagePriority::Warning, origin, payload)
    }

    #[must_use]
    pub fn with_target(mut self, target: impl Into<DispatchTarget>) -> Self {
        self.target_loop = Some(target.into());
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
