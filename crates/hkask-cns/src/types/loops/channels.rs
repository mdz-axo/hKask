//! Domain channel message types — direct typed channels for inter-loop communication.
//!
//! Each pathway gets its own typed `tokio::mpsc` channel. Channel identity replaces
//! both the former `LoopId` and `DispatchTarget` routing of the old Communication Loop.

use crate::algedonic::RuntimeAlert;
use hkask_types::WebID;

// ── Alerts channel: Cybernetics → Curation ──────────────────────────────────

// RuntimeAlert is the canonical type in crate::algedonic.
// Re-imported here so CurationInput::Alert(RuntimeAlert) compiles.

// ── Tool consumption channel: GovernedTool → Cybernetics ─────────────────────

/// Per-tool gas consumption report from GovernedTool to Cybernetics.
///
/// Replaces `LoopPayload::ToolConsumption`. Sent on a dedicated
/// `tokio::sync::mpsc::Sender<ToolConsumptionEvent>` channel.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolConsumptionEvent {
    pub tool_name: String,
    pub agent: WebID,
    pub gas_cost: u64,
    pub success: bool,
}

// ── Spec channel: SpecCurator → Curation ────────────────────────────────────

/// Spec drift alert when coherence between specs and tools degrades.
///
/// Replaces `LoopPayload::SpecDriftAlert`. Sent on a dedicated
/// `tokio::sync::mpsc::Sender<SpecEvent>` channel to CurationLoop's inbox.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpecEvent {
    pub spec_id: String,
    pub drift_magnitude: f64,
    pub drift_threshold: f64,
    pub missing_verbs: Vec<String>,
}

// ── Goal channel: GoalStore → Curation ──────────────────────────────────────

/// Goal state transition notification.
///
/// Replaces `LoopPayload::GoalTransition`. Sent on a dedicated
/// `tokio::sync::mpsc::Sender<GoalTransitionEvent>` channel to CurationLoop's inbox.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoalTransitionEvent {
    pub goal_id: String,
    pub from_state: String,
    pub to_state: String,
    pub agent: WebID,
}

// ── Communication channel: CommunicationWatcher → Curation ──────────────────

/// Communication event forwarded from the 7R7 listener through NuEventStore.
///
/// Sent via the curation inbox so the Curator can sense and respond to
/// agent-to-agent or human-to-agent Matrix activity.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommunicationEvent {
    /// Span category (e.g., "communication.message", "communication.thread").
    pub span_category: String,
    /// Span path within the category (e.g., "observed", "created").
    pub span_path: String,
    /// The observation payload from the NuEvent.
    pub observation: serde_json::Value,
    /// ISO 8601 timestamp of the original NuEvent.
    pub observed_at: String,
}

// ── Curation input enum — what CurationLoop reads from its inbox ─────────────

/// Messages CurationLoop receives from multiple producers via a single channel.
///
/// Cybernetics sends `Alert`, SpecCurator sends `SpecDrift`, GoalStore sends
/// `GoalTransition`, CommunicationWatcher sends `Communication`.
/// Human resolves spec drift via `SpecDriftResolved`.
/// All flow through one `mpsc::Sender<CurationInput>` channel.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CurationInput {
    /// Algedonic alert from Cybernetics (variety deficit escalation)
    Alert(RuntimeAlert),
    /// Spec drift alert from DefaultSpecCurator
    SpecDrift(SpecEvent),
    /// Goal state transition from GoalStore
    GoalTransition(GoalTransitionEvent),
    /// Communication event from the Matrix transport (message, thread, agent lifecycle)
    Communication(CommunicationEvent),
    /// Spec drift resolved by human (P1: User Sovereignty — human resolves, not machine)
    SpecDriftResolved {
        /// The spec whose drift was resolved.
        spec_id: String,
        /// Resolution timestamp.
        resolved_at: String,
    },
}
