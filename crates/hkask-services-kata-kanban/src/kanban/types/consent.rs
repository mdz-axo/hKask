use super::*;

// ── Consent Proof ──────────────────────────────────────────────────────────

/// ConsentProof — evidence that an agent has consented to a task assignment.
///
/// P1 (User Sovereignty) §4: "No agent is assigned work without consent."
/// This type is deliberately opaque — the service layer validates it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentProof {
    /// The WebID of the consenting agent.
    pub agent: WebID,
    /// The task being consented to.
    pub task_id: TaskId,
    /// When consent was given.
    pub consented_at: DateTime<Utc>,
}

impl ConsentProof {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  agent and task_id are valid
    /// post: returns ConsentProof with consented_at=now
    pub fn new(agent: WebID, task_id: TaskId) -> Self {
        Self {
            agent,
            task_id,
            consented_at: Utc::now(),
        }
    }
}
