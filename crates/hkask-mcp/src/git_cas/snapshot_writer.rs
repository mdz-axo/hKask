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

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::agent_def::{AgentDefinition, AgentKind};
    use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};
    use hkask_types::goal::Goal;
    use hkask_types::id::WebID;
    use hkask_types::ports::git_cas::MockGitCas;
    use hkask_types::visibility::Visibility;
    use std::collections::HashSet;

    /// Tracer bullet 15: SnapshotWriter.put_registry_entry writes to Registry repo.
    #[tokio::test]
    async fn put_registry_entry_writes_to_registry_repo() {
        let mock = Arc::new(MockGitCas::new());
        let writer = SnapshotWriter::new(mock.clone());

        let agent = RegisteredAgent {
            definition: AgentDefinition {
                name: "test-agent".to_string(),
                agent_kind: AgentKind::Bot,
                charter: None,
                capabilities: vec![],
                rights: vec![],
                responsibilities: vec![],
                persona: None,
                depends_on: vec![],
                process_manifest: None,
            },
            token_hash: "abc123".to_string(),
            registered_at: "2025-01-01T00:00:00Z".to_string(),
            source_yaml: "test".to_string(),
        };

        let hash = writer
            .put_registry_entry(&agent)
            .await
            .expect("put should succeed");
        assert!(!hash.to_string().is_empty(), "hash should be non-empty");

        // Verify round-trip: retrieve the blob from the Registry repo
        let retrieved = mock
            .get_blob(&RepoId::Registry, &hash)
            .await
            .expect("get_blob should succeed");
        let roundtrip: RegisteredAgent =
            serde_json::from_slice(&retrieved).expect("deserialization should succeed");
        assert_eq!(roundtrip.token_hash, "abc123");
    }

    /// Tracer bullet 16: SnapshotWriter.put_nu_event writes to CnsAudit repo.
    #[tokio::test]
    async fn put_nu_event_writes_to_cns_audit_repo() {
        let mock = Arc::new(MockGitCas::new());
        let writer = SnapshotWriter::new(mock.clone());

        let event = NuEvent::new(
            WebID::new(),
            Span::new(SpanNamespace::new("cns.test"), "invoked"),
            Phase::Sense,
            serde_json::json!({"key": "value"}),
            0,
        );

        let hash = writer
            .put_nu_event(&event)
            .await
            .expect("put should succeed");
        assert!(!hash.to_string().is_empty());

        let retrieved = mock
            .get_blob(&RepoId::CnsAudit, &hash)
            .await
            .expect("get_blob should succeed");
        assert!(!retrieved.is_empty(), "retrieved blob should have content");
    }

    /// Tracer bullet 17: SnapshotWriter.put_goal writes to GoalsSpecs repo.
    #[tokio::test]
    async fn put_goal_writes_to_goals_specs_repo() {
        let mock = Arc::new(MockGitCas::new());
        let writer = SnapshotWriter::new(mock.clone());

        let goal = Goal::new(WebID::new(), "test goal", Visibility::Public);

        let hash = writer.put_goal(&goal).await.expect("put should succeed");
        assert!(!hash.to_string().is_empty());

        let retrieved = mock
            .get_blob(&RepoId::GoalsSpecs, &hash)
            .await
            .expect("get_blob should succeed");
        let roundtrip: Goal =
            serde_json::from_slice(&retrieved).expect("deserialization should succeed");
        assert_eq!(roundtrip.text, "test goal");
    }

    /// Tracer bullet 18: SnapshotWriter.put_consent_record writes to Sovereignty repo.
    #[tokio::test]
    async fn put_consent_record_writes_to_sovereignty_repo() {
        let mock = Arc::new(MockGitCas::new());
        let writer = SnapshotWriter::new(mock.clone());

        let record = StoredConsentRecord {
            id: "consent-1".to_string(),
            webid: "did:web:user".to_string(),
            granted_categories: HashSet::from(["inference".to_string()]),
            granted_at: 1700000000,
            revoked_at: None,
            active: true,
        };

        let hash = writer
            .put_consent_record(&record)
            .await
            .expect("put should succeed");
        assert!(!hash.to_string().is_empty());

        let retrieved = mock
            .get_blob(&RepoId::Sovereignty, &hash)
            .await
            .expect("get_blob should succeed");
        let roundtrip: StoredConsentRecord =
            serde_json::from_slice(&retrieved).expect("deserialization should succeed");
        assert_eq!(roundtrip.webid, "did:web:user");
    }

    /// Tracer bullet 19: SnapshotWriter.put_session writes to Sessions repo.
    #[tokio::test]
    async fn put_session_writes_to_sessions_repo() {
        let mock = Arc::new(MockGitCas::new());
        let writer = SnapshotWriter::new(mock.clone());

        let session = StoredSession {
            session_id: "sess-1".to_string(),
            config_yaml: "agent: test".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            last_active: "2025-01-01T00:00:00Z".to_string(),
            key_version: 1,
            sealed: false,
        };

        let hash = writer
            .put_session(&session)
            .await
            .expect("put should succeed");
        assert!(!hash.to_string().is_empty());

        let retrieved = mock
            .get_blob(&RepoId::Sessions, &hash)
            .await
            .expect("get_blob should succeed");
        let roundtrip: StoredSession =
            serde_json::from_slice(&retrieved).expect("deserialization should succeed");
        assert_eq!(roundtrip.session_id, "sess-1");
    }

    /// Tracer bullet 20: TripleEntry from_triple preserves fields.
    #[tokio::test]
    async fn put_triple_entry_preserves_fields() {
        let mock = Arc::new(MockGitCas::new());
        let writer = SnapshotWriter::new(mock.clone());

        let entry = TripleEntry {
            id: "triple-1".to_string(),
            entity: "alice".to_string(),
            attribute: "knows".to_string(),
            value: serde_json::json!("bob"),
            valid_from: "2025-01-01T00:00:00Z".to_string(),
            valid_to: None,
            confidence: 1.0,
            perspective: "did:web:alice".to_string(),
            visibility: "Public".to_string(),
        };

        let hash = writer.put_triple(&entry).await.expect("put should succeed");
        assert!(!hash.to_string().is_empty());

        let retrieved = mock
            .get_blob(&RepoId::Memory, &hash)
            .await
            .expect("get_blob should succeed");
        let roundtrip: TripleEntry =
            serde_json::from_slice(&retrieved).expect("deserialization should succeed");
        assert_eq!(roundtrip.entity, "alice");
        assert_eq!(roundtrip.attribute, "knows");
    }
}
