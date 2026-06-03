//! Memory Loop Adapter — routes through hkask-memory's domain logic
//!
//! Wraps `EpisodicMemory` and `SemanticMemory` so that pods get
//! domain-logic-enriched storage (dedup, Bayesian confidence decay,
//! temporal attention weighting) through the loop membrane.

use crate::error::MemoryError;
use crate::ports::{EpisodicStoragePort, SemanticStoragePort};
use hkask_memory::{EpisodicMemory, SemanticMemory};
use hkask_storage::{Database, EmbeddingStore, Triple, TripleStore};
use hkask_types::{DelegationToken, ExperienceClassification, Visibility, WebID};
use serde_json::Value;
use std::sync::Arc;

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
        let db = Database::in_memory().map_err(|e| MemoryError::Storage(e.to_string()))?;
        Self::from_database(db)
    }

    /// Create from database path and passphrase (encrypted).
    pub fn from_path(path: &str, passphrase: &str) -> Result<Self, MemoryError> {
        let db =
            Database::open(path, passphrase).map_err(|e| MemoryError::Storage(e.to_string()))?;
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

// =============================================================================
// Episodic Storage Port — routed through EpisodicMemory
// =============================================================================

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
        if token.action == hkask_types::DelegationAction::Read {
            return Err(MemoryError::CapabilityDenied(
                "Token has read-only action, write required for episodic storage".to_string(),
            ));
        }

        let triple = Triple::new(entity, attribute, value, producer_webid)
            .with_visibility(Visibility::Private)
            .with_perspective(producer_webid)
            .with_confidence(confidence);

        self.episodic
            .store(triple)
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        Ok(entity.to_string())
    }

    fn recall_episodic(
        &self,
        query: &str,
        owner: &WebID,
        token: &DelegationToken,
    ) -> Result<Vec<Value>, MemoryError> {
        match token.action {
            hkask_types::DelegationAction::Read
            | hkask_types::DelegationAction::Execute
            | hkask_types::DelegationAction::Validate => {}
            _ => {
                return Err(MemoryError::CapabilityDenied(
                    "Token does not grant read access for episodic recall".to_string(),
                ));
            }
        }

        // Route through EpisodicMemory's deduped+decayed query
        let triples = self
            .episodic
            .query_for_deduped(query, *owner)
            .map_err(|e| MemoryError::Query(e.to_string()))?;

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
        let count = self
            .episodic
            .storage_usage(perspective)
            .map_err(|e| MemoryError::Query(e.to_string()))?;

        tracing::debug!(
            target: "cns.memory.budget",
            perspective = %perspective,
            count = count,
            "Episodic storage usage checked (via loop membrane)"
        );

        Ok(count)
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
        if token.action == hkask_types::DelegationAction::Read {
            return Err(MemoryError::CapabilityDenied(
                "Token has read-only action, write required for episodic storage".to_string(),
            ));
        }

        let confidence = confidence_override.unwrap_or_else(|| classification.default_confidence());

        let triple = Triple::new(entity, attribute, value, producer_webid)
            .with_visibility(Visibility::Private)
            .with_perspective(producer_webid)
            .with_confidence(confidence);

        tracing::info!(
            target: "cns.memory.encode",
            classification = %classification,
            confidence = confidence,
            entity = entity,
            attribute = attribute,
            "Episodic experience encoded (via loop membrane)"
        );

        self.episodic
            .store(triple)
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        Ok(entity.to_string())
    }
}

// =============================================================================
// Semantic Storage Port — routed through SemanticMemory
// =============================================================================

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
        if token.action == hkask_types::DelegationAction::Read {
            return Err(MemoryError::CapabilityDenied(
                "Token has read-only action, write required for semantic storage".to_string(),
            ));
        }

        let triple = Triple::new(entity, attribute, value, producer_webid)
            .with_visibility(Visibility::Shared)
            .with_confidence(confidence);

        self.episodic
            .store(triple)
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        Ok(entity.to_string())
    }

    fn recall_semantic(
        &self,
        query: &str,
        token: &DelegationToken,
    ) -> Result<Vec<Value>, MemoryError> {
        match token.action {
            hkask_types::DelegationAction::Read
            | hkask_types::DelegationAction::Execute
            | hkask_types::DelegationAction::Validate => {}
            _ => {
                return Err(MemoryError::CapabilityDenied(
                    "Token does not grant read access for semantic recall".to_string(),
                ));
            }
        }

        // Route through SemanticMemory's deduped query
        let triples = self
            .semantic
            .query_deduped(query)
            .map_err(|e| MemoryError::Query(e.to_string()))?;

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
        // SemanticMemory removed storage_usage, so we count via triple_count
        let count = self
            .semantic
            .triple_count()
            .map_err(|e| MemoryError::Query(e.to_string()))?;

        tracing::debug!(
            target: "cns.memory.budget",
            entity = %entity,
            count = count,
            "Semantic storage usage checked (via loop membrane)"
        );

        Ok(count)
    }
}
