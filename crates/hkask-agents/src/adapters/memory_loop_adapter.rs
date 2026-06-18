//! Memory Loop Adapter — routes through hkask-memory's domain logic
//
//! Wraps `EpisodicMemory` and `SemanticMemory` so that pods get
//! domain-logic-enriched storage (dedup, Bayesian confidence decay,
//! temporal attention weighting) through the loop membrane.

use hkask_rsolidity as rs;
use crate::error::MemoryError;
use crate::ports::{
    EpisodicStoragePort, RecallRequest, RecalledEpisode, RecalledSemantic, SemanticStoragePort,
    StorageRequest,
};
use hkask_memory::{EpisodicMemory, SemanticMemory};
use hkask_storage::{Database, EmbeddingStore, Triple, TripleStore};
use hkask_types::{
    Confidence, DelegationToken, ExperienceClassification, require_read_access,
    require_write_access,
};
use std::sync::Arc;

// ── Template Method helpers (P2.4) ──────────────────────────────────────

/// Convert a `Triple` into a typed `RecalledEpisode`.
///
/// Used by `recall_episodic` to produce domain-typed DTOs instead of
/// untyped `serde_json::Value`. `recall_semantic` uses
/// `triple_to_recalled_semantic` (separate type, no perspective).
fn triple_to_recalled_episode(t: Triple) -> RecalledEpisode {
    RecalledEpisode {
        id: t.id.to_string(),
        entity: t.entity,
        attribute: t.attribute,
        value: t.value,
        confidence: t.confidence,
        perspective: t.access.perspective,
        visibility: t.access.visibility,
        valid_from: t.temporal.valid_from.to_rfc3339(),
    }
}

/// Convert a `Triple` into a typed `RecalledSemantic`.
///
/// Used by `recall_semantic` to produce domain-typed DTOs instead of
/// untyped `serde_json::Value`. Semantic triples are perspective-free,
/// so this struct omits the `perspective` field that `RecalledEpisode` carries.
fn triple_to_recalled_semantic(t: Triple) -> RecalledSemantic {
    RecalledSemantic {
        id: t.id.to_string(),
        entity: t.entity,
        attribute: t.attribute,
        value: t.value,
        confidence: t.confidence,
        visibility: t.access.visibility,
        valid_from: t.temporal.valid_from.to_rfc3339(),
    }
}

/// Build a `Triple` from a `StorageRequest`.
///
/// The `StorageRequest` capture-common-parameters (P2.4/P1.5), and this
/// helper converts it into the `Triple` value object that the storage layer
/// expects — eliminating per-method inline construction.
fn request_to_triple(req: &StorageRequest) -> Triple {
    let mut triple = Triple::new(
        &req.entity,
        &req.attribute,
        req.value.clone(),
        req.access.owner_webid,
    )
    .with_visibility(req.access.visibility)
    .with_confidence(req.confidence);
    if let Some(p) = req.access.perspective {
        triple = triple.with_perspective(p);
    }
    triple
}

/// Verify write access to the given store type.
///
/// Wraps `require_write_access` with `MemoryError::CapabilityDenied`
/// mapping, eliminating the `.map_err()` repetition across store methods.
fn check_write_access(token: &DelegationToken, store_type: &str) -> Result<(), MemoryError> {
    require_write_access(token, store_type).map_err(|msg| MemoryError::CapabilityDenied {
        resource: store_type.to_string(),
        action: msg,
    })
}

/// Verify read access to the given store type.
///
/// Wraps `require_read_access` with `MemoryError::CapabilityDenied`
/// mapping, eliminating the `.map_err()` repetition across recall methods.
fn check_read_access(token: &DelegationToken, store_type: &str) -> Result<(), MemoryError> {
    require_read_access(token, store_type).map_err(|msg| MemoryError::CapabilityDenied {
        resource: store_type.to_string(),
        action: msg,
    })
}

/// Store a triple via a backend, with capability check and entity return.
///
/// Extracts the common pattern shared by `store_episodic`, `store_semantic`,
/// and `store_episodic_classified`: check write access, convert request to
/// triple, delegate to the storage backend, return the entity string.
///
/// The `store_fn` closure receives the constructed `Triple` and is expected
/// to call the appropriate backend's `store` method, converting its error
/// type to `MemoryError` via the `?` operator (which relies on existing
/// `From` impls).
fn store_via<E>(
    request: StorageRequest,
    token: &DelegationToken,
    store_type: &str,
    store_fn: impl FnOnce(Triple) -> Result<(), E>,
) -> Result<String, MemoryError>
where
    MemoryError: From<E>,
{
    check_write_access(token, store_type)?;
    let entity = request.entity.clone();
    store_fn(request_to_triple(&request))?;
    Ok(entity)
}

/// Memory Loop Forwarder — wraps EpisodicMemory and SemanticMemory
///
/// F-SYN-015: the type was previously named `MemoryLoopAdapter`. The
/// rename to `MemoryLoopForwarder` reflects what the type actually
/// does: it *forwards* pod storage requests through `hkask-memory`'s
/// domain logic (dedup, Bayesian confidence decay, temporal
/// attention weighting) without owning the underlying loops. The
/// old name `MemoryLoopAdapter` is preserved as a type alias for
/// source compatibility; the rename is the fix, not a deprecation.
///
/// Routes pod storage requests through `hkask-memory`'s domain
/// logic instead of directly hitting `TripleStore`.
pub struct MemoryLoopForwarder {
    episodic: EpisodicMemory,
    semantic: SemanticMemory,
}

/// F-SYN-015: type alias for source compatibility. New code should
/// use `MemoryLoopForwarder` directly.
pub type MemoryLoopAdapter = MemoryLoopForwarder;

impl MemoryLoopForwarder {
    /// Create a new adapter wrapping EpisodicMemory and SemanticMemory.
    ///
    /// expect: "The system loads and adapts agent registries for generative use" [P3]
    /// \[P3\] Motivating: Generative Space — MemoryLoopForwarder wires episodic + semantic
    /// pre:  `episodic` is a valid `EpisodicMemory`; `semantic` is a valid
    ///       `SemanticMemory`.
    /// post: Returns a `MemoryLoopForwarder` holding both memory instances.
    #[rs::contract(id = "P3-agt-memory-adapter-new", principle = "P3")]
    #[rs::contract(id = "P3-agt-memory-adapter-new", principle = "P3")]
    pub fn new(episodic: EpisodicMemory, semantic: SemanticMemory) -> Self {
        Self { episodic, semantic }
    }

    /// Create with in-memory storage for testing.
    ///
    /// expect: "The system loads and adapts agent registries for generative use" [P3]
    /// \[P3\] Motivating: Generative Space — in-memory SQLite adapter for tests
    /// pre:  (none).
    /// post: Returns `Ok(Self)` with an in-memory SQLite database;
    ///       returns `Err(MemoryError)` if database creation fails.
    #[rs::contract(id = "P3-agt-memory-adapter-in-memory", principle = "P3")]
    #[rs::contract(id = "P3-agt-memory-adapter-in-memory", principle = "P3")]
    pub fn in_memory() -> Result<Self, MemoryError> {
        let db = Database::in_memory()?;
        Self::from_database(db)
    }

    /// Create with in-memory storage, panicking on failure.
    ///
    /// Use this in builder patterns and test fixtures where an in-memory DB
    /// \[DECLARATIVE\] failure is always a bug, never a recoverable condition. For recoverable (P5 — Essentialism).
    /// contexts, use `in_memory()` and propagate the error with `?`.
    ///
    /// expect: "The system loads and adapts agent registries for generative use" [P3]
    /// \[P3\] Motivating: Generative Space — infallible in-memory constructor for tests
    /// pre:  (none).
    /// post: Returns `Self` with an in-memory database; panics if
    ///       database creation fails (considered a bug).
    #[rs::contract(id = "P3-agt-memory-adapter-in-memory-unwrap", principle = "P3")]
    #[rs::contract(id = "P3-agt-memory-adapter-in-memory-unwrap", principle = "P3")]
    pub fn in_memory_unchecked() -> Self {
        Self::in_memory().expect("In-memory storage initialization should never fail")
    }

    /// Create from database path and passphrase (encrypted).
    ///
    /// expect: "The system loads and adapts agent registries for generative use" [P3]
    /// \[P1\] Motivating: User Sovereignty — encrypted on-disk memory adapter
    /// \[P4\] Constraining: Clear Boundaries — passphrase protects the store
    /// pre:  `path` is a valid filesystem path; `passphrase` is a
    ///       non-empty string.
    /// post: Returns `Ok(Self)` with an encrypted SQLite database at
    ///       `path`; returns `Err(MemoryError)` if opening fails.
    #[rs::contract(id = "P3-agt-memory-adapter-encrypted", principle = "P3")]
    #[rs::contract(id = "P3-agt-memory-adapter-encrypted", principle = "P3")]
    pub fn from_path(path: &str, passphrase: &str) -> Result<Self, MemoryError> {
        let db = Database::open(path, passphrase)?;
        Self::from_database(db)
    }

    fn from_database(db: Database) -> Result<Self, MemoryError> {
        let conn = db.conn_arc();
        let triple_store = TripleStore::new(Arc::clone(&conn));
        let episodic = EpisodicMemory::new(triple_store);
        let triple_store2 = TripleStore::new(Arc::clone(&conn));
        let embedding_store = EmbeddingStore::new(conn);
        let semantic = SemanticMemory::new(triple_store2, embedding_store);
        Ok(Self::new(episodic, semantic))
    }
}

// Episodic Storage Port — routed through EpisodicMemory

impl EpisodicStoragePort for MemoryLoopAdapter {
    fn store_episodic(
        &self,
        request: StorageRequest,
        token: &DelegationToken,
    ) -> Result<String, MemoryError> {
        store_via(request, token, "episodic", |triple| {
            self.episodic.store(triple)
        })
    }

    fn recall_episodic(
        &self,
        request: &RecallRequest,
    ) -> Result<Vec<RecalledEpisode>, MemoryError> {
        check_read_access(&request.token, "episodic")?;

        // P4.1: Replaced `.expect(...)` with a typed error. Episodic memory
        // is owner-scoped (OCAP), so a missing `perspective` is a capability
        // constraint violation, not a panic-worthy condition.
        let owner = request
            .perspective
            .ok_or_else(|| MemoryError::CapabilityDenied {
                resource: "episodic_memory".into(),
                action: "recall requires a perspective (owner WebID)".into(),
            })?;

        // Route through EpisodicMemory's deduped+decayed query
        let triples = self.episodic.query_for_deduped(&request.query, owner)?;

        let results: Vec<RecalledEpisode> = triples
            .into_iter()
            .map(triple_to_recalled_episode)
            .collect();

        tracing::debug!(
            target: "hkask.memory.episodic",
            query = %request.query,
            owner = %owner,
            results = results.len(),
            "Episodic recall (via loop membrane)"
        );

        Ok(results)
    }

    fn episodic_storage_usage(
        &self,
        perspective: &hkask_types::WebID,
    ) -> Result<usize, MemoryError> {
        let count = self.episodic.storage_usage(perspective)?;

        // F-SYN-013: `cns.memory.budget` is observed by the
        // cybernetics loop's `CyberneticsLoop` (consumes via
        // `tracing-subscriber` layer). The expected consumer is
        // the `hkask-cns` crate's CNS runtime, not any in-source
        // subscriber. The consumer boundary is the tracing
        // registry, configured at startup via
        // `hkask_cli::bootstrap::install_tracing_subscriber`.
        tracing::debug!(
            target: "cns.memory.budget",
            perspective = %perspective,
            count = count,
            "Episodic storage usage checked (via loop membrane); consumer: hkask-cns"
        );

        Ok(count)
    }

    fn episodic_storage_budget(&self) -> usize {
        self.episodic.storage_budget()
    }

    fn store_episodic_classified(
        &self,
        request: StorageRequest,
        classification: ExperienceClassification,
        confidence_override: Option<Confidence>,
        token: &DelegationToken,
    ) -> Result<String, MemoryError> {
        // Resolve confidence: override takes precedence, otherwise classification default
        let confidence = confidence_override
            .unwrap_or_else(|| Confidence::new(classification.default_confidence()));

        tracing::info!(
            target: "cns.memory.encode",
            classification = %classification,
            confidence = %confidence,
            entity = %request.entity,
            attribute = %request.attribute,
            "Episodic experience encoded (via loop membrane)"
        );

        store_via(
            StorageRequest {
                confidence,
                ..request
            },
            token,
            "episodic",
            |triple| self.episodic.store(triple),
        )
    }
}

// Semantic Storage Port — routed through SemanticMemory

impl SemanticStoragePort for MemoryLoopAdapter {
    fn store_semantic(
        &self,
        request: StorageRequest,
        token: &DelegationToken,
    ) -> Result<String, MemoryError> {
        store_via(request, token, "semantic", |triple| {
            self.semantic.store(triple)
        })
    }

    fn recall_semantic(
        &self,
        request: &RecallRequest,
    ) -> Result<Vec<RecalledSemantic>, MemoryError> {
        check_read_access(&request.token, "semantic")?;

        // Route through SemanticMemory's deduped query
        let triples = self.semantic.query_deduped(&request.query)?;

        let results: Vec<RecalledSemantic> = triples
            .into_iter()
            .map(triple_to_recalled_semantic)
            .collect();

        tracing::debug!(
            target: "hkask.memory.semantic",
            query = %request.query,
            results = results.len(),
            "Semantic recall (via loop membrane)"
        );

        Ok(results)
    }

    fn semantic_storage_usage(&self, entity: &str) -> Result<usize, MemoryError> {
        let count = self.semantic.triple_count_for_entity(entity)?;

        tracing::debug!(
            target: "cns.memory.budget",
            entity = %entity,
            count = count,
            "Semantic storage usage checked (via loop membrane)"
        );

        Ok(count)
    }

    fn search_similar(
        &self,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<RecalledSemantic>, MemoryError> {
        let results = self.semantic.search_similar(query_vector, limit)?;
        let mut triples = Vec::new();
        for r in results {
            if let Ok(matches) = self.semantic.query_deduped(&r.embedding.entity_ref) {
                for t in matches {
                    triples.push(RecalledSemantic {
                        id: t.id.to_string(),
                        entity: t.entity,
                        attribute: t.attribute,
                        value: t.value,
                        confidence: t.confidence,
                        visibility: t.access.visibility,
                        valid_from: t.temporal.valid_from.to_rfc3339(),
                    });
                }
            }
        }
        Ok(triples)
    }
}
