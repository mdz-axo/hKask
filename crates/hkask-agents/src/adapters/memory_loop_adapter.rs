//! Memory Loop Adapter — routes through hkask-memory's domain logic
//
//! Wraps `EpisodicMemory` and `SemanticMemory` so that pods get
//! domain-logic-enriched storage (dedup, Bayesian confidence decay,
//! temporal attention weighting) through the loop membrane.

use crate::error::MemoryError;
use crate::ports::{EpisodicStoragePort, SemanticStoragePort};
use hkask_memory::{EpisodicMemory, SemanticMemory};
use hkask_storage::{Database, EmbeddingStore, Triple, TripleStore};
use hkask_types::{
    DelegationToken, ExperienceClassification, Visibility, WebID, require_read_access,
    require_write_access,
};
use serde_json::Value;
use std::sync::Arc;

/// Capture-common-parameters struct for memory storage operations (P2.4/P1.5).
///
/// Groups the fields that every store call shares (entity, attribute, value,
/// producer, confidence, visibility) so that `store_episodic`,
/// `store_episodic_classified`, and `store_semantic` can delegate to a
/// single private method rather than each building a `Triple` inline.
pub struct StorageRequest {
    pub entity: String,
    pub attribute: String,
    pub value: Value,
    pub producer: WebID,
    pub confidence: f64,
    pub visibility: Visibility,
    pub perspective: Option<WebID>,
}

impl StorageRequest {
    /// Build a `Triple` from this request.
    fn to_triple(&self) -> Triple {
        let mut triple = Triple::new(
            &self.entity,
            &self.attribute,
            self.value.clone(),
            self.producer,
        )
        .with_visibility(self.visibility)
        .with_confidence(self.confidence);
        if let Some(p) = self.perspective {
            triple = triple.with_perspective(p);
        }
        triple
    }
}

/// Capture-common-parameters struct for memory storage operations (P2.4/P1.5).
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
        producer_webid: WebID,
        entity: &str,
        attribute: &str,
        value: Value,
        confidence: f64,
        token: &DelegationToken,
    ) -> Result<String, MemoryError> {
        require_write_access(token, "episodic").map_err(MemoryError::CapabilityDenied)?;

        let req = StorageRequest {
            entity: entity.to_string(),
            attribute: attribute.to_string(),
            value,
            producer: producer_webid,
            confidence,
            visibility: Visibility::Private,
            perspective: Some(producer_webid),
        };
        self.episodic.store(req.to_triple())?;

        Ok(entity.to_string())
    }

    fn recall_episodic(
        &self,
        query: &str,
        owner: &WebID,
        token: &DelegationToken,
    ) -> Result<Vec<Value>, MemoryError> {
        require_read_access(token, "episodic").map_err(MemoryError::CapabilityDenied)?;

        // Route through EpisodicMemory's deduped+decayed query
        let triples = self.episodic.query_for_deduped(query, *owner)?;

        let results: Vec<Value> = triples
            .into_iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id.to_string(),
                    "entity": t.entity,
                    "attribute": t.attribute,
                    "value": t.value,
                    "confidence": t.confidence,
                    "perspective": t.perspective.map(|p| p.to_string()),
                    "visibility": t.visibility.as_str(),
                    "valid_from": t.valid_from.to_rfc3339(),
                })
            })
            .collect();

        tracing::debug!(
            target: "hkask.memory.episodic",
            query = %query,
            owner = %owner,
            results = results.len(),
            "Episodic recall (via loop membrane)"
        );

        Ok(results)
    }

    fn episodic_storage_usage(&self, perspective: &WebID) -> Result<usize, MemoryError> {
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
        producer_webid: WebID,
        entity: &str,
        attribute: &str,
        value: Value,
        classification: ExperienceClassification,
        confidence_override: Option<f64>,
        token: &DelegationToken,
    ) -> Result<String, MemoryError> {
        require_write_access(token, "episodic").map_err(MemoryError::CapabilityDenied)?;

        let confidence = confidence_override.unwrap_or_else(|| classification.default_confidence());

        tracing::info!(
            target: "cns.memory.encode",
            classification = %classification,
            confidence = confidence,
            entity = entity,
            attribute = attribute,
            "Episodic experience encoded (via loop membrane)"
        );

        let req = StorageRequest {
            entity: entity.to_string(),
            attribute: attribute.to_string(),
            value,
            producer: producer_webid,
            confidence,
            visibility: Visibility::Private,
            perspective: Some(producer_webid),
        };
        self.episodic.store(req.to_triple())?;

        Ok(entity.to_string())
    }
}

// Semantic Storage Port — routed through SemanticMemory

impl SemanticStoragePort for MemoryLoopAdapter {
    fn store_semantic(
        &self,
        producer_webid: WebID,
        entity: &str,
        attribute: &str,
        value: Value,
        confidence: f64,
        token: &DelegationToken,
    ) -> Result<String, MemoryError> {
        require_write_access(token, "semantic").map_err(MemoryError::CapabilityDenied)?;

        let req = StorageRequest {
            entity: entity.to_string(),
            attribute: attribute.to_string(),
            value,
            producer: producer_webid,
            confidence,
            visibility: Visibility::Shared,
            perspective: None,
        };
        self.semantic.store(req.to_triple())?;

        Ok(entity.to_string())
    }

    fn recall_semantic(
        &self,
        query: &str,
        token: &DelegationToken,
    ) -> Result<Vec<Value>, MemoryError> {
        require_read_access(token, "semantic").map_err(MemoryError::CapabilityDenied)?;

        // Route through SemanticMemory's deduped query
        let triples = self.semantic.query_deduped(query)?;

        let results: Vec<Value> = triples
            .into_iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id.to_string(),
                    "entity": t.entity,
                    "attribute": t.attribute,
                    "value": t.value,
                    "confidence": t.confidence,
                    "perspective": t.perspective.map(|p| p.to_string()),
                    "visibility": t.visibility.as_str(),
                    "valid_from": t.valid_from.to_rfc3339(),
                })
            })
            .collect();

        tracing::debug!(
            target: "hkask.memory.semantic",
            query = %query,
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
