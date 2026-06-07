//! SnapshotWriter — Thin serialization layer over [`GitCASPort`]
//!
//! Each `put_*` method serializes a domain type to JSON and writes it through
//! the CAS port to the appropriate [`RepoId`]. The mapping is:
//!
//! | Domain type       | RepoId         |
//! |-------------------|----------------|
//! | RegisteredAgent   | Registry       |
//! | TripleEntry       | Memory         |
//! | NuEvent           | CnsAudit       |
//! | StoredConsentRec  | Sovereignty    |
//! | Goal              | GoalsSpecs     |
//! | StoredSession     | Sessions       |

use hkask_storage::StoredConsentRecord;
use hkask_storage::StoredSession;
use hkask_types::agent_def::RegisteredAgent;
use hkask_types::event::NuEvent;
use hkask_types::goal::Goal;
use hkask_types::ports::git_cas::{ContentHash, GitCASPort, GitCasError, RepoId, TripleEntry};
use std::sync::Arc;

/// Thin serialization layer that writes domain types through a [`GitCASPort`].
///
/// Each method knows which [`RepoId`] to target. Callers don't need to
/// know the repo mapping — they just call `put_registry_entry(agent)`.
#[allow(dead_code)]
pub(crate) struct SnapshotWriter {
    port: Arc<dyn GitCASPort>,
}

#[allow(dead_code)]
impl SnapshotWriter {
    /// Create a new SnapshotWriter wrapping the given port.
    pub(crate) fn new(port: Arc<dyn GitCASPort>) -> Self {
        Self { port }
    }

    /// Serialize and store a [`RegisteredAgent`] in the Registry repo.
    pub(crate) async fn put_registry_entry(
        &self,
        agent: &RegisteredAgent,
    ) -> Result<ContentHash, GitCasError> {
        let bytes = serde_json::to_vec(agent)
            .map_err(|e| GitCasError::Io(format!("Failed to serialize RegisteredAgent: {e}")))?;
        self.port.put_blob(&RepoId::Registry, &bytes).await
    }

    /// Serialize and store a [`TripleEntry`] in the Memory repo.
    pub(crate) async fn put_triple(
        &self,
        triple: &TripleEntry,
    ) -> Result<ContentHash, GitCasError> {
        let bytes = serde_json::to_vec(triple)
            .map_err(|e| GitCasError::Io(format!("Failed to serialize TripleEntry: {e}")))?;
        self.port.put_blob(&RepoId::Memory, &bytes).await
    }

    /// Serialize and store a [`NuEvent`] in the CnsAudit repo.
    pub(crate) async fn put_nu_event(&self, event: &NuEvent) -> Result<ContentHash, GitCasError> {
        let bytes = serde_json::to_vec(event)
            .map_err(|e| GitCasError::Io(format!("Failed to serialize NuEvent: {e}")))?;
        self.port.put_blob(&RepoId::CnsAudit, &bytes).await
    }

    /// Serialize and store a [`StoredConsentRecord`] in the Sovereignty repo.
    pub(crate) async fn put_consent_record(
        &self,
        record: &StoredConsentRecord,
    ) -> Result<ContentHash, GitCasError> {
        let bytes = serde_json::to_vec(record).map_err(|e| {
            GitCasError::Io(format!("Failed to serialize StoredConsentRecord: {e}"))
        })?;
        self.port.put_blob(&RepoId::Sovereignty, &bytes).await
    }

    /// Serialize and store a [`Goal`] in the GoalsSpecs repo.
    pub(crate) async fn put_goal(&self, goal: &Goal) -> Result<ContentHash, GitCasError> {
        let bytes = serde_json::to_vec(goal)
            .map_err(|e| GitCasError::Io(format!("Failed to serialize Goal: {e}")))?;
        self.port.put_blob(&RepoId::GoalsSpecs, &bytes).await
    }

    /// Serialize and store a [`StoredSession`] in the Sessions repo.
    pub(crate) async fn put_session(
        &self,
        session: &StoredSession,
    ) -> Result<ContentHash, GitCasError> {
        let bytes = serde_json::to_vec(session)
            .map_err(|e| GitCasError::Io(format!("Failed to serialize StoredSession: {e}")))?;
        self.port.put_blob(&RepoId::Sessions, &bytes).await
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────
