//! Domain channel message types — direct typed channels replacing LoopMessage routing.
//!
//! Each pathway gets its own typed channel instead of routing through a generic
//! Communication Loop. Channel identity replaces both LoopId and DispatchTarget.
//!
//! These types are introduced alongside the legacy `LoopMessage`/`LoopPayload` types
//! (strangler fig pattern). Once all producers and consumers are migrated, the legacy
//! types and the Communication Loop are removed.

use crate::WebID;

// ── Alerts channel: Cybernetics → Curation ──────────────────────────────────

/// Runtime alert from Cybernetics to Curation when variety deficit exceeds threshold.
///
/// Replaces `LoopPayload::AlgedonicAlert`. Sent on a dedicated
/// `tokio::sync::mpsc::Sender<RuntimeAlert>` channel directly from
/// CyberneticsLoop to CurationLoop's inbox.
///
/// This pathway is a Prohibition-level constraint — it must survive unbroken
/// because Curation depends on the algedonic signal to detect regulation failure.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RuntimeAlert {
    pub current: u64,
    pub threshold: u64,
    pub deficit: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

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

/// Messages CurationLoop receives from its unified inbox channel.
///
/// Multiple producers send into a single channel that Curation drains during
/// its sense phase. This replaces the old pattern of matching on
/// `LoopPayload` variants in the CurationLoop inbox.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CurationInput {
    /// Algedonic alert from Cybernetics (variety deficit escalation)
    Alert(RuntimeAlert),
    /// Spec drift alert from DefaultSpecCurator
    SpecDrift(SpecEvent),
    /// Goal state transition from GoalStore
    GoalTransition(GoalTransitionEvent),
}
