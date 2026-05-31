//! Loop 6: Communication — Messenger types
//!
//! The Communication loop is the second master loop (alongside Curation).
//! It enables all inter-loop communication through messenger functions:
//!
//! - 6.1 DISPATCH (GUARD+ROUTE) — send with priority queuing
//! - 6.2 CORRELATE (SENSE) — observe delivery, correlate traces
//! - 6.3 DAMPEN (FILTER+RECONCILE) — suppress repeated directives within time window
//! - 6.4 Channel CIRCUIT (CIRCUIT) — circuit-break inter-loop channels
//! - 6.5 ACKNOWLEDGE (VALIDATE+ROUTE) — confirm delivery, route response
//!
//! Communication has no subloops because all subloops ARE communication
//! pattern instances. It delivers messenger functions on inter-loop edges.
//!
//! # Design
//!
//! `LoopMessage` is the unit of inter-loop communication. Every message
//! carries a `TraceId` for correlation and a `MessagePriority` for
//! dispatch ordering. The priority system ensures that algedonic alerts
//! and governance directives are processed before routine observations.

use crate::id::WebID;
use std::fmt;

// =============================================================================
// TraceId — Cross-loop correlation identifier
// =============================================================================

/// Trace identifier for correlating messages across loop boundaries.
///
/// Every `LoopMessage` carries a `TraceId` that propagates across all
/// inter-loop calls. This enables:
/// - Correlation of cause and effect across loop boundaries
/// - Debugging of message flow through the 8-loop system
/// - CNS observability of inter-loop communication patterns
///
/// `TraceId` is a UUID-based identifier that is created at the message
/// origin and preserved through all routing and forwarding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TraceId(pub uuid::Uuid);

impl TraceId {
    /// Create a new random trace ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Create a trace ID from a UUID string.
    pub fn from_string(s: &str) -> Self {
        Self(uuid::Uuid::parse_str(s).unwrap_or_else(|_| uuid::Uuid::new_v4()))
    }

    /// Create a trace ID from an existing UUID.
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

/// Priority level for inter-loop messages.
///
/// Messages are dispatched in priority order. Critical messages
/// (algedonic alerts, sovereignty violations) are processed first,
/// followed by warnings (governance directives, threshold breaches),
/// then routine information (observations, metrics).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessagePriority {
    /// Critical: algedonic alerts, sovereignty violations, circuit-breaker trips
    Critical,
    /// Warning: governance directives, threshold breaches, escalation routing
    Warning,
    /// Info: routine observations, metrics, span emission
    Info,
}

impl MessagePriority {
    /// Numeric priority for ordering (lower = higher priority).
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

/// Identifies which loop a message originates from.
///
/// Every message carries its origin loop for routing and observability.
/// This enables CORRELATE (messenger function 6.2) to trace message flow
/// across the 8-loop system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopOrigin {
    /// Loop 1: Inference
    Inference,
    /// Loop 2a: Episodic Memory
    Episodic,
    /// Loop 2b: Semantic Memory
    Semantic,
    /// Loop 3: Governance
    Governance,
    /// Loop 4: Observability (CNS)
    Observability,
    /// Loop 5: Curation (regulator)
    Curation,
    /// Loop 6: Communication (this loop)
    Communication,
    /// Loop 7: Cybernetics (manages Observability→Governance feedback cycle)
    Cybernetics,
    /// External source (CLI, API, MCP)
    External,
}

impl fmt::Display for LoopOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoopOrigin::Inference => write!(f, "inference"),
            LoopOrigin::Episodic => write!(f, "episodic"),
            LoopOrigin::Semantic => write!(f, "semantic"),
            LoopOrigin::Governance => write!(f, "governance"),
            LoopOrigin::Observability => write!(f, "observability"),
            LoopOrigin::Curation => write!(f, "curation"),
            LoopOrigin::Communication => write!(f, "communication"),
            LoopOrigin::Cybernetics => write!(f, "cybernetics"),
            LoopOrigin::External => write!(f, "external"),
        }
    }
}

// =============================================================================
// LoopPayload — Message content
// =============================================================================

/// The content of an inter-loop message.
///
/// `LoopPayload` carries the typed data that flows between loops.
/// Each variant corresponds to a category of inter-loop communication.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopPayload {
    /// Algedonic alert: variety deficit exceeds threshold
    AlgedonicAlert {
        current: u64,
        threshold: u64,
        deficit: u64,
    },
    /// Governance directive: calibrate, update, or adjust
    GovernanceDirective {
        directive_type: String,
        target: WebID,
        parameters: serde_json::Value,
    },
    /// Observability observation: span emission or variety update
    Observation {
        category: String,
        data: serde_json::Value,
    },
    /// Memory operation: store, recall, or consolidate
    MemoryOperation {
        operation: String,
        data_category: String,
        data: serde_json::Value,
    },
    /// Capability change: grant, attenuate, or revoke
    CapabilityChange {
        agent: WebID,
        change_type: String,
        details: serde_json::Value,
    },
    /// Circuit breaker state change
    CircuitStateChange {
        circuit_id: String,
        new_state: String,
    },
    /// Custom payload for extensibility
    Custom {
        tag: String,
        data: serde_json::Value,
    },
}

// =============================================================================
// LoopMessage — Inter-loop communication unit
// =============================================================================

/// A message sent between loops via the Communication master loop.
///
/// `LoopMessage` is the unit of inter-loop communication. It carries:
/// - A `TraceId` for cross-loop correlation
/// - A `MessagePriority` for dispatch ordering
/// - A `LoopOrigin` identifying the source loop
/// - A `LoopPayload` with the typed message content
/// - An optional `target_loop` for direct addressing
///
/// # Messenger Functions
///
/// The Communication loop provides 5 messenger functions that operate
/// on `LoopMessage` instances:
///
/// 1. **DISPATCH** (GUARD+ROUTE): Priority-ordered message queuing
/// 2. **CORRELATE** (SENSE): Trace propagation and delivery tracking
/// 3. **DAMPEN** (FILTER+RECONCILE): Suppress repeated directives within time window
/// 4. **Channel CIRCUIT** (CIRCUIT): Circuit-break inter-loop channels
/// 5. **ACKNOWLEDGE** (VALIDATE+ROUTE): Confirm delivery, route response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoopMessage {
    /// Cross-loop correlation identifier
    pub trace_id: TraceId,
    /// Dispatch priority
    pub priority: MessagePriority,
    /// Source loop
    pub origin: LoopOrigin,
    /// Message content
    pub payload: LoopPayload,
    /// Target loop (None = broadcast to all interested loops)
    pub target_loop: Option<LoopOrigin>,
    /// Timestamp of message creation
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Agent that triggered this message (if applicable)
    pub sender: Option<WebID>,
}

impl LoopMessage {
    /// Create a new loop message with the given priority, origin, and payload.
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

    /// Create a critical-priority message.
    pub fn critical(origin: LoopOrigin, payload: LoopPayload) -> Self {
        Self::new(MessagePriority::Critical, origin, payload)
    }

    /// Create a warning-priority message.
    pub fn warning(origin: LoopOrigin, payload: LoopPayload) -> Self {
        Self::new(MessagePriority::Warning, origin, payload)
    }

    /// Create an info-priority message.
    pub fn info(origin: LoopOrigin, payload: LoopPayload) -> Self {
        Self::new(MessagePriority::Info, origin, payload)
    }

    /// Set the target loop for directed messaging.
    #[must_use]
    pub fn with_target(mut self, target: LoopOrigin) -> Self {
        self.target_loop = Some(target);
        self
    }

    /// Set the sender agent.
    #[must_use]
    pub fn with_sender(mut self, sender: WebID) -> Self {
        self.sender = Some(sender);
        self
    }

    /// Whether this message is a broadcast (no specific target).
    pub fn is_broadcast(&self) -> bool {
        self.target_loop.is_none()
    }

    /// Whether this message is directed at a specific loop.
    pub fn is_directed(&self) -> bool {
        self.target_loop.is_some()
    }
}
