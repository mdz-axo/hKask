//! Loop 2a: Episodic Memory — private, agent-scoped experience
//!
//! experience → encode → store (private) → recall → temporal weight → context
//!
//! Essential subloops:
//! - 2a.1 Experience Encoding (FILTER) — filter and classify incoming experience
//! - 2a.2 Temporal Attention (ADAPT) — weight by recency: e^(-λ × time_since_storage)
//! - 2a.3 Confidence Decay (RECONCILE) — confidence decreases over time
//! - 2a.4 Episodic Storage Budget (GUARD) — per-agent storage limit, consolidation
//!
//! Cybernetics regulation: storage budget adjustment
//!
//! Episodic memory is PRIVATE to the agent. Only the owning agent can
//! store or read their own episodic triples.
//!
//! OCAP enforcement is via `DelegationToken` + `CapabilityChecker` (HMAC-signed
//! tokens verified at the port membrane). Budget enforcement is via
//! `EpisodicLoop` (cybernetic sense→compute→act cycle with SQL COUNT queries).

// Experience Classification (Loop 2a.1)

/// Classification of an episodic experience for encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperienceClassification {
    Success,
    Failure,
}

impl ExperienceClassification {
    pub fn default_confidence(&self) -> f64 {
        match self {
            ExperienceClassification::Success => 0.9,
            ExperienceClassification::Failure => 0.3,
        }
    }
}

impl std::fmt::Display for ExperienceClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExperienceClassification::Success => write!(f, "success"),
            ExperienceClassification::Failure => write!(f, "failure"),
        }
    }
}
