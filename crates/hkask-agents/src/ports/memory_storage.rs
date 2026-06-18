//! Memory Storage Ports — Episodic and Semantic boundaries
//!
//! Episodic (private, agent-scoped) and semantic (shared, public) access patterns.
//!
//! # OCAP Discipline
//!
//! - `EpisodicStoragePort` — store/recall episodic triples (private, agent-scoped)
//!   Only the owning agent can store or read their own episodic triples.
//! - `SemanticStoragePort` — store/recall semantic triples (shared, public)
//!   Any agent with a capability token can read semantic triples.
//!   Only agents with consolidation capability can store semantic triples.

use hkask_rsolidity as rs;
use hkask_types::visibility::AccessControl;
use hkask_types::{Confidence, DelegationToken, ExperienceClassification, Visibility, WebID};
use serde_json::Value;

// ── Request value objects (P2.4/P1.5: eliminate data clumps) ───────────────

/// Capture-common-parameters struct for memory store operations (P2.4/P1.5).
///
/// Groups the fields that every store call shares (entity, attribute, value,
/// access, confidence) so that `store_episodic`, `store_episodic_classified`,
/// and `store_semantic` accept a single request object instead of a flat
/// parameter list.
///
/// For classified episodic stores, use [`StorageRequest::classified_episodic`]
/// to attach an experience classification and optional confidence override.
#[derive(Debug, Clone)]
pub struct StorageRequest {
    /// The entity (subject) of the triple.
    pub entity: String,
    /// The attribute (predicate/property) of the triple.
    pub attribute: String,
    /// The value (object) of the triple.
    pub value: Value,
    /// Confidence score (0.0–1.0).
    pub confidence: Confidence,
    /// Access control: visibility, perspective, owner.
    pub access: AccessControl,
}

impl StorageRequest {
    /// Create a new `StorageRequest` with all fields specified.
    ///
    /// expect: "The system loads and adapts agent registries for generative use" [P3]
    /// \[P3\] Motivating: Generative Space — StorageRequest creates a memory triple
    /// \[P1\] Constraining: User Sovereignty — access.owner_webid carries ownership
    /// pre:  `entity` and `attribute` are non-empty strings after `.into()`;
    ///       `confidence` is in range [0.0, 1.0]; `access` carries a valid
    ///       `AccessControl` variant (episodic or semantic).
    /// post: Returns a `StorageRequest` with all fields set to the provided
    ///       values; no validation is performed — the caller is responsible
    ///       for correctness.
    #[rs::contract(id = "P3-agt-memory-request-new", principle = "P3")]
    #[rs::contract(id = "P3-agt-memory-request-new", principle = "P3")]
    pub fn new(
        entity: impl Into<String>,
        attribute: impl Into<String>,
        value: Value,
        confidence: Confidence,
        access: AccessControl,
    ) -> Self {
        Self {
            entity: entity.into(),
            attribute: attribute.into(),
            value,
            confidence,
            access,
        }
    }

    /// Create an episodic (private, perspective-bound) store request.
    ///
    /// Convenience constructor that sets `access` to `AccessControl::episodic`.
    ///
    /// expect: "The system loads and adapts agent registries for generative use" [P3]
    /// \[P3\] Motivating: Generative Space — episodic request binds perspective to owner
    /// pre:  `producer_webid` is a valid WebID; `confidence` in [0.0, 1.0];
    ///       `entity` and `attribute` are non-empty after `.into()`.
    /// post: Returns a `StorageRequest` with `access` set to episodic
    ///       (perspective = owner = `producer_webid`).
    #[rs::contract(id = "P3-agt-memory-request-episodic", principle = "P3")]
    #[rs::contract(id = "P3-agt-memory-request-episodic", principle = "P3")]
    pub fn episodic(
        entity: impl Into<String>,
        attribute: impl Into<String>,
        value: Value,
        confidence: Confidence,
        producer_webid: WebID,
    ) -> Self {
        Self::new(
            entity,
            attribute,
            value,
            confidence,
            AccessControl::episodic(producer_webid, producer_webid),
        )
    }

    /// Create a semantic (shared, perspective-free) store request.
    ///
    /// Convenience constructor that sets `access` to `AccessControl::semantic`.
    ///
    /// expect: "The system loads and adapts agent registries for generative use" [P3]
    /// \[P3\] Motivating: Generative Space — semantic request is perspective-free
    /// pre:  `producer_webid` is a valid WebID; `confidence` in [0.0, 1.0];
    ///       `entity` and `attribute` are non-empty after `.into()`.
    /// post: Returns a `StorageRequest` with `access` set to semantic
    ///       (no perspective, owner = `producer_webid`).
    #[rs::contract(id = "P3-agt-memory-request-semantic", principle = "P3")]
    #[rs::contract(id = "P3-agt-memory-request-semantic", principle = "P3")]
    pub fn semantic(
        entity: impl Into<String>,
        attribute: impl Into<String>,
        value: Value,
        confidence: Confidence,
        producer_webid: WebID,
    ) -> Self {
        Self::new(
            entity,
            attribute,
            value,
            confidence,
            AccessControl::semantic(producer_webid),
        )
    }

    /// Create a classified episodic store request (Loop 2a.1).
    ///
    /// Resolves confidence from the classification if no override is provided:
    /// - `Success` → 0.9
    /// - `Failure` → 0.3
    ///
    /// expect: "The system loads and adapts agent registries for generative use" [P3]
    /// \[P8\] Motivating: Semantic Grounding — classification maps to confidence scalar
    /// pre:  `classification` is a valid `ExperienceClassification` variant;
    ///       `confidence_override`, if `Some`, is in [0.0, 1.0];
    ///       `producer_webid` is a valid WebID.
    /// post: Returns a `StorageRequest` with episodic access and confidence
    ///       resolved from `classification` (or overridden).
    #[rs::contract(id = "P3-agt-memory-confidence-map", principle = "P3")]
    #[rs::contract(id = "P3-agt-memory-confidence-map", principle = "P3")]
    pub fn classified_episodic(
        entity: impl Into<String>,
        attribute: impl Into<String>,
        value: Value,
        classification: ExperienceClassification,
        confidence_override: Option<Confidence>,
        producer_webid: WebID,
    ) -> Self {
        let confidence = confidence_override
            .unwrap_or_else(|| Confidence::new(classification.default_confidence()));
        Self::episodic(entity, attribute, value, confidence, producer_webid)
    }
}

/// Capture-common-parameters struct for memory recall operations (P2.4/P1.5).
///
/// Groups the query string with the access-control token so that recall
/// signatures don't pass flat parameters.
#[derive(Debug, Clone)]
pub struct RecallRequest {
    /// The query string (entity name or search term).
    pub query: String,
    /// The perspective (owner WebID) for episodic recall.
    /// `None` for semantic recall (perspective-free).
    pub perspective: Option<WebID>,
    /// OCAP capability token.
    pub token: DelegationToken,
}

impl RecallRequest {
    /// Create an episodic recall request (perspective-bound).
    ///
    /// expect: "The system loads and adapts agent registries for generative use" [P3]
    /// \[P3\] Motivating: Generative Space — episodic recall requires delegation token
    /// \[P4\] Constraining: Clear Boundaries — token proves capability
    /// pre:  `query` is non-empty after `.into()`; `owner` is a valid WebID;
    ///       `token` is a valid `DelegationToken`.
    /// post: Returns a `RecallRequest` with `perspective = Some(owner)`.
    #[rs::contract(id = "P3-agt-memory-recall-episodic", principle = "P3")]
    #[rs::contract(id = "P3-agt-memory-recall-episodic", principle = "P3")]
    pub fn episodic(query: impl Into<String>, owner: WebID, token: DelegationToken) -> Self {
        Self {
            query: query.into(),
            perspective: Some(owner),
            token,
        }
    }

    /// Create a semantic recall request (perspective-free).
    ///
    /// expect: "The system loads and adapts agent registries for generative use" [P3]
    /// \[P3\] Motivating: Generative Space — semantic recall requires delegation token
    /// \[P4\] Constraining: Clear Boundaries — token proves capability
    /// pre:  `query` is non-empty after `.into()`; `token` is a valid
    ///       `DelegationToken`.
    /// post: Returns a `RecallRequest` with `perspective = None`.
    #[rs::contract(id = "P3-agt-memory-recall-semantic", principle = "P3")]
    #[rs::contract(id = "P3-agt-memory-recall-semantic", principle = "P3")]
    pub fn semantic(query: impl Into<String>, token: DelegationToken) -> Self {
        Self {
            query: query.into(),
            perspective: None,
            token,
        }
    }
}

/// Typed DTO for recalled episodic triples.
///
/// Replaces `Vec<serde_json::Value>` as the return type of
/// `EpisodicStoragePort::recall_episodic`. Uses domain types
/// (`Confidence`, `Visibility`, `WebID`) instead of raw strings
/// so that callers get compile-time field safety rather than
/// fragile `.get("field").and_then(|v| v.as_str())` destructuring.
#[derive(Debug, Clone)]
pub struct RecalledEpisode {
    /// Triple ID (serialized string form).
    pub id: String,
    /// Entity (subject) of the triple.
    pub entity: String,
    /// Attribute (predicate/property) of the triple.
    pub attribute: String,
    /// Value (object) of the triple.
    pub value: Value,
    /// Confidence score (0.0–1.0).
    pub confidence: Confidence,
    /// Perspective (owning agent WebID) for episodic triples.
    pub perspective: Option<WebID>,
    /// Visibility level (Private, Shared, Public).
    pub visibility: Visibility,
    /// Timestamp when the triple became valid (RFC 3339).
    pub valid_from: String,
}

/// Typed DTO for recalled semantic triples.
///
/// Replaces `Vec<serde_json::Value>` as the return type of
/// `SemanticStoragePort::recall_semantic`. Uses domain types
/// (`Confidence`, `Visibility`) instead of raw strings so that
/// callers get compile-time field safety rather than fragile
/// `.get("field").and_then(|v| v.as_str())` destructuring.
///
/// Semantic triples are perspective-free (consolidated from
/// episodic, shared/public knowledge), so this struct omits
/// the `perspective` field that `RecalledEpisode` carries.
#[derive(Debug, Clone)]
pub struct RecalledSemantic {
    /// Triple ID (serialized string form).
    pub id: String,
    /// Entity (subject) of the triple.
    pub entity: String,
    /// Attribute (predicate/property) of the triple.
    pub attribute: String,
    /// Value (object) of the triple.
    pub value: Value,
    /// Confidence score (0.0–1.0).
    pub confidence: Confidence,
    /// Visibility level (Private, Shared, Public).
    pub visibility: Visibility,
    /// Timestamp when the triple became valid (RFC 3339).
    pub valid_from: String,
}

// Episodic Storage Port — Private, agent-scoped memory

/// Port trait for episodic memory storage operations.
///
/// Episodic memory is private to the owning agent. Only the agent whose
/// WebID matches the `perspective` field can store or read their own
/// episodic triples. OCAP enforcement is via `DelegationToken` +
/// `CapabilityChecker` (HMAC-signed tokens verified at the membrane).
pub trait EpisodicStoragePort: Send + Sync {
    /// Store an episodic triple (private, agent-scoped).
    ///
    /// # Requires
    /// - \[NORMATIVE\] `request.access` must carry an episodic access control (perspective-bound) (P4 — Clear Boundaries)
    /// - \[NORMATIVE\] `request.access.owner_webid` must match the agent storing the triple (P1 — User Sovereignty)
    /// - \[NORMATIVE\] `token` must grant Write action on the Manifest resource (P4 — OCAP)
    /// - The triple is stored with the agent's perspective (WebID)
    fn store_episodic(
        &self,
        request: StorageRequest,
        token: &DelegationToken,
    ) -> Result<String, crate::error::MemoryError>;

    /// Recall episodic triples for the agent's own perspective.
    ///
    /// # Requires
    /// - \[NORMATIVE\] `request.token` must grant Read action on the Manifest resource (P4 — OCAP)
    /// - Returns only triples matching the agent's perspective
    fn recall_episodic(
        &self,
        request: &RecallRequest,
    ) -> Result<Vec<RecalledEpisode>, crate::error::MemoryError>;

    /// Check episodic storage budget for an agent.
    ///
    /// Returns the number of triples currently stored for the given perspective.
    /// Used by Loop 2a.4 (Storage Budget) to enforce per-agent limits.
    fn episodic_storage_usage(
        &self,
        perspective: &WebID,
    ) -> Result<usize, crate::error::MemoryError>;

    /// Get the configured per-agent storage budget (max triples).
    ///
    /// Used by the API usage endpoint and budget status reporting.
    fn episodic_storage_budget(&self) -> usize;

    /// Store an episodic triple with experience classification (Loop 2a.1).
    ///
    /// This is the enhanced store method that accepts an experience
    /// classification. The classification determines the default confidence
    /// if no override is provided:
    ///
    /// - `Success` → 0.9
    /// - `Failure` → 0.3
    ///
    /// # Requires
    /// - \[NORMATIVE\] `request.access` must carry an episodic access control (perspective-bound) (P4 — Clear Boundaries)
    /// - \[NORMATIVE\] `request.access.owner_webid` must match the agent storing the triple (P1 — User Sovereignty)
    /// - \[NORMATIVE\] `token` must grant Write action on the Manifest resource (P4 — OCAP)
    fn store_episodic_classified(
        &self,
        request: StorageRequest,
        classification: ExperienceClassification,
        confidence_override: Option<Confidence>,
        token: &DelegationToken,
    ) -> Result<String, crate::error::MemoryError>;
}

// Semantic Storage Port — Shared, public knowledge

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
    /// - \[NORMATIVE\] `request.access` must carry semantic access control (shared, no perspective) (P4 — Clear Boundaries)
    /// - \[NORMATIVE\] `token` must grant Write action on the Manifest resource (P4 — OCAP)
    /// - The triple is stored without perspective (consolidated from episodic)
    fn store_semantic(
        &self,
        request: StorageRequest,
        token: &DelegationToken,
    ) -> Result<String, crate::error::MemoryError>;

    /// Recall semantic triples (shared, deduplicated knowledge).
    ///
    /// # Requires
    /// - \[NORMATIVE\] `request.token` must grant Read action on the Manifest resource (P4 — OCAP)
    /// - Returns all triples matching the query (no perspective filter)
    fn recall_semantic(
        &self,
        request: &RecallRequest,
    ) -> Result<Vec<RecalledSemantic>, crate::error::MemoryError>;

    /// Check semantic storage usage for an entity.
    ///
    /// Returns the number of semantic triples currently stored for the given entity.
    /// Used by Loop 6e (Semantic Storage Budget) to enforce per-entity limits.
    fn semantic_storage_usage(&self, entity: &str) -> Result<usize, crate::error::MemoryError>;

    /// KNN similarity search over semantic embeddings.
    ///
    /// Returns triples whose embeddings are closest to the query vector.
    /// Used for context injection when exact entity-text-match returns nothing.
    /// Default implementation returns empty — override for KNN support.
    fn search_similar(
        &self,
        _query_vector: &[f32],
        _limit: usize,
    ) -> Result<Vec<RecalledSemantic>, crate::error::MemoryError> {
        Ok(Vec::new())
    }
}
