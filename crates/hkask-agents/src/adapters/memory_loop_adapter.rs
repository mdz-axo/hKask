//! Memory Loop Adapter — routes through hkask-memory's domain logic
//
//! Wraps `EpisodicMemory` and `SemanticMemory` so that pods get
//! domain-logic-enriched storage (dedup, Bayesian confidence decay,
//! temporal attention weighting) through the loop membrane.

use crate::error::MemoryError;
use crate::ports::{EpisodicStoragePort, RecallRequest, SemanticStoragePort, StorageRequest};
use hkask_memory::{EpisodicMemory, SemanticMemory};
use hkask_storage::{Database, EmbeddingStore, Triple, TripleStore};
use hkask_types::{
    Confidence, DelegationToken, ExperienceClassification, require_read_access,
    require_write_access,
};
use serde_json::Value;
use std::sync::Arc;

// ── Template Method helpers (P2.4) ──────────────────────────────────────

/// Convert a `Triple` into its canonical JSON representation.
///
/// Both `recall_episodic` and `recall_semantic` produce the same JSON
/// shape — this function eliminates that duplication.
fn triple_to_json(t: Triple) -> Value {
    serde_json::json!({
        "id": t.id.to_string(),
        "entity": t.entity,
        "attribute": t.attribute,
        "value": t.value,
        "confidence": t.confidence.value(),
        "perspective": t.access.perspective.map(|p| p.to_string()),
        "visibility": t.access.visibility.as_str(),
        "valid_from": t.temporal.valid_from.to_rfc3339(),
    })
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
    require_write_access(token, store_type).map_err(MemoryError::CapabilityDenied)
}

/// Verify read access to the given store type.
///
/// Wraps `require_read_access` with `MemoryError::CapabilityDenied`
/// mapping, eliminating the `.map_err()` repetition across recall methods.
fn check_read_access(token: &DelegationToken, store_type: &str) -> Result<(), MemoryError> {
    require_read_access(token, store_type).map_err(MemoryError::CapabilityDenied)
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

/// Memory Loop Adapter — wraps EpisodicMemory and SemanticMemory
///
/// Routes pod storage requests through `hkask-memory`'s domain logic
/// (dedup, Bayesian confidence decay, temporal attention weighting)
/// instead of directly hitting `TripleStore`.
pub struct MemoryLoopAdapter {
    episodic: EpisodicMemory,
    semantic: SemanticMemory,
}

impl MemoryLoopAdapter {
    /// Create a new adapter wrapping EpisodicMemory and SemanticMemory.
    pub fn new(episodic: EpisodicMemory, semantic: SemanticMemory) -> Self {
        Self { episodic, semantic }
    }

    /// Create with in-memory storage for testing.
    pub fn in_memory() -> Result<Self, MemoryError> {
        let db = Database::in_memory()?;
        Self::from_database(db)
    }

    /// Create with in-memory storage, panicking on failure.
    ///
    /// Use this in builder patterns and test fixtures where an in-memory DB
    /// failure is always a bug, never a recoverable condition. For recoverable
    /// contexts, use `in_memory()` and propagate the error with `?`.
    pub fn in_memory_unchecked() -> Self {
        Self::in_memory().expect("In-memory storage initialization should never fail")
    }

    /// Create from database path and passphrase (encrypted).
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

    fn recall_episodic(&self, request: &RecallRequest) -> Result<Vec<Value>, MemoryError> {
        check_read_access(&request.token, "episodic")?;

        let owner = request
            .perspective
            .expect("Episodic recall requires a perspective (owner WebID)");

        // Route through EpisodicMemory's deduped+decayed query
        let triples = self.episodic.query_for_deduped(&request.query, owner)?;

        let results: Vec<Value> = triples.into_iter().map(triple_to_json).collect();

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

        tracing::debug!(
            target: "cns.memory.budget",
            perspective = %perspective,
            count = count,
            "Episodic storage usage checked (via loop membrane)"
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

    fn recall_semantic(&self, request: &RecallRequest) -> Result<Vec<Value>, MemoryError> {
        check_read_access(&request.token, "semantic")?;

        // Route through SemanticMemory's deduped query
        let triples = self.semantic.query_deduped(&request.query)?;

        let results: Vec<Value> = triples.into_iter().map(triple_to_json).collect();

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
}
