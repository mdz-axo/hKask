//! Memory Storage Ports — Episodic and Semantic boundaries
//!
//! Split from the monolithic `MemoryStoragePort` into episodic (private,
//! agent-scoped) and semantic (shared, public) access patterns.
//!
//! # OCAP Discipline
//!
//! - `EpisodicStoragePort` — store/recall episodic triples (private, agent-scoped)
//!   Only the owning agent can store or read their own episodic triples.
//! - `SemanticStoragePort` — store/recall semantic triples (shared, public)
//!   Any agent with a capability token can read semantic triples.
//!   Only agents with consolidation capability can store semantic triples.
//! - `MemoryStoragePort` — legacy monolithic port (deprecated, use split ports)

use hkask_types::{CapabilityToken, ExperienceClassification, WebID};

// =============================================================================
// Episodic Storage Port — Private, agent-scoped memory
// =============================================================================

/// Port trait for episodic memory storage operations.
///
/// Episodic memory is private to the owning agent. Only the agent whose
/// WebID matches the `perspective` field can store or read their own
/// episodic triples. This enforces the OCAP boundary:
/// `EpisodicReadHandle` can only read own-perspective triples.
/// `EpisodicWriteHandle` can only write own-perspective triples.
pub trait EpisodicStoragePort: Send + Sync {
    /// Store an episodic triple (private, agent-scoped).
    ///
    /// # Requires
    /// - `producer_webid` must match the agent storing the triple
    /// - `token` must grant Write action on the Manifest resource
    /// - The triple is stored with the agent's perspective (WebID)
    fn store_episodic(
        &self,
        producer_webid: WebID,
        entity: &str,
        attribute: &str,
        value: serde_json::Value,
        confidence: f64,
        token: &CapabilityToken,
    ) -> Result<String, crate::error::MemoryError>;

    /// Recall episodic triples for the agent's own perspective.
    ///
    /// # Requires
    /// - `token` must grant Read action on the Manifest resource
    /// - Returns only triples matching the agent's perspective
    fn recall_episodic(
        &self,
        query: &str,
        owner: &WebID,
        token: &CapabilityToken,
    ) -> Result<Vec<serde_json::Value>, crate::error::MemoryError>;

    /// Check episodic storage budget for an agent.
    ///
    /// Returns the number of triples currently stored for the given perspective.
    /// Used by Loop 2a.5 (Storage Budget) to enforce per-agent limits.
    fn episodic_storage_usage(
        &self,
        perspective: &WebID,
    ) -> Result<usize, crate::error::MemoryError>;

    /// Store an episodic triple with experience classification (Loop 2a.1).
    ///
    /// This is the enhanced store method that accepts an experience
    /// classification. The classification determines the default confidence
    /// if `confidence_override` is `None`:
    ///
    /// - `Success` → 0.9
    /// - `Failure` → 0.3
    /// - `Observation` → 0.7
    /// - `Inference` → 0.5
    /// - `Instruction` → 0.8
    ///
    /// # Requires
    /// - `producer_webid` must match the agent storing the triple
    /// - `token` must grant Write action on the Manifest resource
    #[allow(clippy::too_many_arguments)]
    fn store_episodic_classified(
        &self,
        producer_webid: WebID,
        entity: &str,
        attribute: &str,
        value: serde_json::Value,
        classification: ExperienceClassification,
        confidence_override: Option<f64>,
        token: &CapabilityToken,
    ) -> Result<String, crate::error::MemoryError>;
}

// =============================================================================
// Semantic Storage Port — Shared, public knowledge
// =============================================================================

/// Port trait for semantic memory storage operations.
///
/// Semantic memory is shared across agents. Any agent with a valid
/// capability token can read semantic triples. Only agents with
/// consolidation capability (Curator, or agents performing
/// consolidation from episodic to semantic) can store new semantic triples.
pub trait SemanticStoragePort: Send + Sync {
    /// Store a semantic triple (shared, public knowledge).
    ///
    /// # Requires
    /// - `token` must grant Write action on the Manifest resource
    /// - The triple is stored without perspective (consolidated from episodic)
    fn store_semantic(
        &self,
        producer_webid: WebID,
        entity: &str,
        attribute: &str,
        value: serde_json::Value,
        confidence: f64,
        token: &CapabilityToken,
    ) -> Result<String, crate::error::MemoryError>;

    /// Recall semantic triples (shared, deduplicated knowledge).
    ///
    /// # Requires
    /// - `token` must grant Read action on the Manifest resource
    /// - Returns all triples matching the query (no perspective filter)
    fn recall_semantic(
        &self,
        query: &str,
        token: &CapabilityToken,
    ) -> Result<Vec<serde_json::Value>, crate::error::MemoryError>;
}

// =============================================================================
// Legacy monolithic port — DEPRECATED, use split ports above
// =============================================================================

/// Port trait for memory storage operations (legacy monolithic).
///
/// **Deprecated:** Use `EpisodicStoragePort` and `SemanticStoragePort` instead.
/// These enforce the OCAP boundary between private episodic memory
/// and shared semantic memory at the type level.
#[deprecated(note = "Use EpisodicStoragePort and SemanticStoragePort instead for OCAP discipline")]
pub trait MemoryStoragePort: Send + Sync {
    fn store_artifact(
        &self,
        producer_webid: WebID,
        artifact_type: &str,
        content: serde_json::Value,
        visibility: &str,
        token: &CapabilityToken,
    ) -> Result<String, crate::error::MemoryError>;

    fn recall(
        &self,
        query: &str,
        token: &CapabilityToken,
    ) -> Result<Vec<serde_json::Value>, crate::error::MemoryError>;
}
