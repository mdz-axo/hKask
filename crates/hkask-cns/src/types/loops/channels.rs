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

// ── Curation input enum — what CurationLoop reads from its inbox ─────────────

/// Messages CurationLoop receives from multiple producers via a single channel.
///
/// Cybernetics sends `Alert`, SpecCurator sends `SpecDrift`, GoalStore sends
/// `GoalTransition`. Human resolves spec drift → `SpecDriftResolved`.
/// All flow through one `mpsc::Sender<CurationInput>` channel.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CurationInput {
    /// Algedonic alert from Cybernetics (variety deficit escalation)
    Alert(RuntimeAlert),
    /// Spec drift alert from DefaultSpecCurator
    SpecDrift(SpecEvent),
    /// Goal state transition from GoalStore
    GoalTransition(GoalTransitionEvent),
    /// Spec drift resolved by human (P1: User Sovereignty — human resolves, not machine)
    SpecDriftResolved {
        /// The spec whose drift was resolved.
        spec_id: String,
        /// Resolution timestamp.
        resolved_at: String,
    },
}
