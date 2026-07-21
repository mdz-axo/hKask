//! Pod value types — PodLifecycleState, PodID, persona types, template types

pub use hkask_types::PodID;
use serde::{Deserialize, Serialize};


/// Agent operating mode — how the agent is currently interacting with the world.
///
/// Initially mutually exclusive: an agent can be in Chat mode OR Server mode,
/// not both. Concurrency support planned for future release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentMode {
    /// Conversational mode: chatting with users/agents, calling tools.
    Chat,
    /// Server mode: presenting as MCP server(s), handling incoming tool calls.
    Server,
}

impl std::fmt::Display for AgentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentMode::Chat => write!(f, "chat"),
            AgentMode::Server => write!(f, "server"),
        }
    }
}

/// Pod tier — determines isolation model and filename convention.
///
/// - Curator: singleton system daemon, owns SemanticIndex, CNS aggregation
/// - UserPod: per-user sovereign pod (1:1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PodKind {
    Curator,
    #[default]
    UserPod,
}

impl std::fmt::Display for PodKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PodKind::Curator => write!(f, "curator"),
            PodKind::UserPod => write!(f, "userpod"),
        }
    }
}

/// Pod lifecycle state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PodLifecycleState {
    /// Pod instantiated from template crate, not yet registered
    Populated,
    /// Registered with A2A runtime, capability token minted
    Registered,
    /// Activated for A2A communication, MCP access granted
    Activated,
    /// Deactivated, capabilities revoked
    Deactivated,
}

impl PodLifecycleState {
    /// Whether a transition from `self` to `next` is legal.
    ///
    /// The lifecycle is a linear progression:
    /// `Populated → Registered → Activated → Deactivated`
    ///
    /// \[DECLARATIVE\] Re-stating the current state is a no-op and always permitted. (P7 — Evolutionary Architecture).
    /// Terminal state `Deactivated` admits no further transitions.
    ///
    /// expect: "Agent interactions are gated by OCAP boundaries"
    /// \[P4\] Motivating: Clear Boundaries — lifecycle state machine enforces transitions
    /// \[P7\] Constraining: Evolutionary Architecture — linear model + idempotent restate
    /// pre:  `self` and `next` are valid `PodLifecycleState` variants.
    /// post: Returns `true` if `self == next` (idempotent) or if the
    ///       transition follows the linear progression; `false` for all
    ///       other transitions (including from `Deactivated`).
    pub fn can_transition_to(&self, next: PodLifecycleState) -> bool {
        if *self == next {
            return true;
        }
        match (self, next) {
            (PodLifecycleState::Populated, PodLifecycleState::Registered)
            | (PodLifecycleState::Registered, PodLifecycleState::Activated)
            | (PodLifecycleState::Activated, PodLifecycleState::Deactivated) => true,
            // Deactivated is terminal; all other moves illegal.
            _ => false,
        }
    }
}

impl std::fmt::Display for PodLifecycleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PodLifecycleState::Populated => write!(f, "populated"),
            PodLifecycleState::Registered => write!(f, "registered"),
            PodLifecycleState::Activated => write!(f, "activated"),
            PodLifecycleState::Deactivated => write!(f, "deactivated"),
        }
    }
}

// ── Communication Accommodation Theory (CAT) — curator convergence posture ─────
// (Curator-only; userpods have no persona. Retained for the curator daemon's
// Matrix engagement posture.)

/// Communication posture — governs whether and how the curator engages via Matrix.
///
/// Grounded in Communication Accommodation Theory (Giles): convergence is the
/// single dimension along which the curator decides to speak or remain silent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationPosture {
    /// Convergence bias: 0.0 (silent, divergent) to 1.0 (fully convergent).
    /// Default: 0.5 — balanced, responds to direct engagement.
    #[serde(default = "default_convergence_bias")]
    pub convergence_bias: f64,

    /// Core traits never compromised by accommodation (consistency anchor).
    #[serde(default)]
    pub invariant_traits: Vec<String>,
}

fn default_convergence_bias() -> f64 {
    0.5
}

impl Default for CommunicationPosture {
    fn default() -> Self {
        Self {
            convergence_bias: 0.5,
            invariant_traits: vec![],
        }
    }
}
