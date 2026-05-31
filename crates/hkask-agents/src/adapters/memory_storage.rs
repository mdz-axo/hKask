//! Memory Storage Adapter
//!
//! Concrete implementations of memory storage ports using hkask-storage crate.
//! Implements `EpisodicStoragePort` and `SemanticStoragePort` traits.

use crate::error::MemoryError;
use crate::ports::{EpisodicStoragePort, SemanticStoragePort};
use hkask_storage::{Database, Embedding, EmbeddingStore, Triple, TripleStore};
use hkask_types::{CapabilityToken, ExperienceClassification, TripleID, Visibility, WebID};
use serde_json::Value;
use uuid::Uuid;

/// Memory Storage Adapter — Concrete implementation for artifact persistence
pub struct MemoryStorageAdapter {
    triple_store: TripleStore,
    embedding_store: EmbeddingStore,
}

impl MemoryStorageAdapter {
    /// Create new memory storage adapter
    pub fn new(db: Database) -> Result<Self, MemoryError> {
        let conn = db.conn_arc();
        Ok(Self {
            triple_store: TripleStore::new(conn.clone()),
            embedding_store: EmbeddingStore::new(conn),
        })
    }

    /// Create from database path and passphrase
    pub fn from_path(path: &str, passphrase: &str) -> Result<Self, MemoryError> {
        let db =
            Database::open(path, passphrase).map_err(|e| MemoryError::Storage(e.to_string()))?;
        Self::new(db)
    }

    /// Create in-memory database for testing
    pub fn in_memory() -> Result<Self, MemoryError> {
        let db = Database::in_memory().map_err(|e| MemoryError::Storage(e.to_string()))?;
        Self::new(db)
    }
}

// =============================================================================
// Episodic Storage Port Implementation
// =============================================================================

impl EpisodicStoragePort for MemoryStorageAdapter {
    fn store_episodic(
        &self,
        producer_webid: WebID,
        entity: &str,
        attribute: &str,
        value: Value,
        confidence: f64,
        token: &CapabilityToken,
    ) -> Result<String, MemoryError> {
        // Validate capability token allows storage operations
        if token.action == hkask_types::CapabilityAction::Read {
            return Err(MemoryError::CapabilityDenied(
                "Token has read-only action, write required for episodic storage".to_string(),
            ));
        }

        let triple = Triple::new(entity, attribute, value, producer_webid)
            .with_visibility(Visibility::Private)
            .with_perspective(producer_webid)
            .with_confidence(confidence);

        self.triple_store
            .insert(&triple)
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        Ok(triple.id.to_string())
    }

    fn recall_episodic(
        &self,
        query: &str,
        owner: &WebID,
        token: &CapabilityToken,
    ) -> Result<Vec<Value>, MemoryError> {
        // Validate capability token allows read operations
        match token.action {
            hkask_types::CapabilityAction::Read
            | hkask_types::CapabilityAction::Execute
            | hkask_types::CapabilityAction::Validate => {}
            _ => {
                return Err(MemoryError::CapabilityDenied(
                    "Token does not grant read access for episodic recall".to_string(),
                ));
            }
        }

        // Query by entity and filter to only the owner's perspective
        let triples = self
            .triple_store
            .query_by_entity(query)
            .map_err(|e| MemoryError::Query(e.to_string()))?;

        let results: Vec<Value> = triples
            .into_iter()
            .filter(|t| t.perspective == Some(*owner))
            .filter(|t| t.is_episodic())
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
            "Episodic recall"
        );

        Ok(results)
    }

    fn episodic_storage_usage(&self, perspective: &WebID) -> Result<usize, MemoryError> {
        let triples = self
            .triple_store
            .query_by_perspective(perspective)
            .map_err(|e| MemoryError::Query(e.to_string()))?;

        let count = triples.iter().filter(|t| t.is_episodic()).count();

        tracing::debug!(
            target: "cns.memory.budget",
            perspective = %perspective,
            count = count,
            "Episodic storage usage checked"
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
        token: &CapabilityToken,
    ) -> Result<String, MemoryError> {
        // Validate capability token allows storage operations
        if token.action == hkask_types::CapabilityAction::Read {
            return Err(MemoryError::CapabilityDenied(
                "Token has read-only action, write required for episodic storage".to_string(),
            ));
        }

        let confidence = confidence_override.unwrap_or_else(|| classification.default_confidence());

        let triple = Triple::new(entity, attribute, value, producer_webid)
            .with_visibility(Visibility::Private)
            .with_perspective(producer_webid)
            .with_confidence(confidence);

        // Store the classification in the attribute namespace for traceability
        // e.g., "outcome" → "observation:entity:attribute"
        // The classification is reflected in the confidence (default from class)
        // and emitted as a cns.memory.encode span.

        tracing::info!(
            target: "cns.memory.encode",
            classification = %classification,
            confidence = confidence,
            entity = entity,
            attribute = attribute,
            "Episodic experience encoded"
        );

        self.triple_store
            .insert(&triple)
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        Ok(triple.id.to_string())
    }
}

// =============================================================================
// Semantic Storage Port Implementation
// =============================================================================

impl SemanticStoragePort for MemoryStorageAdapter {
    fn store_semantic(
        &self,
        producer_webid: WebID,
        entity: &str,
        attribute: &str,
        value: Value,
        confidence: f64,
        token: &CapabilityToken,
    ) -> Result<String, MemoryError> {
        // Validate capability token allows storage operations
        if token.action == hkask_types::CapabilityAction::Read {
            return Err(MemoryError::CapabilityDenied(
                "Token has read-only action, write required for semantic storage".to_string(),
            ));
        }

        // Semantic triples are shared (not private) and have no perspective
        let triple = Triple::new(entity, attribute, value, producer_webid)
            .with_visibility(Visibility::Shared)
            .with_confidence(confidence);

        self.triple_store
            .insert(&triple)
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        Ok(triple.id.to_string())
    }

    fn recall_semantic(
        &self,
        query: &str,
        token: &CapabilityToken,
    ) -> Result<Vec<Value>, MemoryError> {
        // Validate capability token allows read operations
        match token.action {
            hkask_types::CapabilityAction::Read
            | hkask_types::CapabilityAction::Execute
            | hkask_types::CapabilityAction::Validate => {}
            _ => {
                return Err(MemoryError::CapabilityDenied(
                    "Token does not grant read access for semantic recall".to_string(),
                ));
            }
        }

        // Query by entity and filter to only semantic triples
        let triples = self
            .triple_store
            .query_by_entity(query)
            .map_err(|e| MemoryError::Query(e.to_string()))?;

        let results: Vec<Value> = triples
            .into_iter()
            .filter(|t| t.is_semantic())
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
            "Semantic recall"
        );

        Ok(results)
    }

    fn semantic_storage_usage(&self, entity: &str) -> Result<usize, MemoryError> {
        let triples = self
            .triple_store
            .query_by_entity(entity)
            .map_err(|e| MemoryError::Query(e.to_string()))?;

        let count = triples.iter().filter(|t| t.is_semantic()).count();

        tracing::debug!(
            target: "cns.memory.budget",
            entity = %entity,
            count = count,
            "Semantic storage usage checked"
        );

        Ok(count)
    }
}

// =============================================================================
// Legacy MemoryStoragePort Implementation (DEPRECATED)
// =============================================================================

#[allow(deprecated)]
impl MemoryStoragePort for MemoryStorageAdapter {
    fn store_artifact(
        &self,
        producer_webid: WebID,
        artifact_type: &str,
        content: Value,
        visibility: &str,
        token: &CapabilityToken,
    ) -> Result<String, MemoryError> {
        // Validate capability token allows storage operations
        if token.action == hkask_types::CapabilityAction::Read {
            return Err(MemoryError::CapabilityDenied(
                "Token has read-only action, write required for storage".to_string(),
            ));
        }
        let visibility = match visibility.to_lowercase().as_str() {
            "public" => Visibility::Public,
            "shared" => Visibility::Shared,
            _ => Visibility::Private,
        };

        match artifact_type {
            "episodic_triple" | "semantic_triple" => {
                let entity = content["entity"].as_str().ok_or_else(|| {
                    MemoryError::Serialization("Missing 'entity' field".to_string())
                })?;
                let attribute = content["attribute"].as_str().ok_or_else(|| {
                    MemoryError::Serialization("Missing 'attribute' field".to_string())
                })?;
                let value = content["value"].clone();

                let mut triple = Triple::new(entity, attribute, value, producer_webid)
                    .with_visibility(visibility);

                if artifact_type == "episodic_triple" {
                    triple = triple.with_perspective(producer_webid);
                }

                self.triple_store
                    .insert(&triple)
                    .map_err(|e| MemoryError::Storage(e.to_string()))?;

                Ok(triple.id.to_string())
            }

            "embedding" => {
                let vector: Vec<f32> = content["vector"]
                    .as_array()
                    .ok_or_else(|| {
                        MemoryError::Serialization("Missing 'vector' field".to_string())
                    })?
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect();

                let model = content["model"].as_str().unwrap_or("default");
                let mut embedding = Embedding::new(vector, model);

                if let Some(entity_ref_str) = content["entity_ref"].as_str()
                    && let Ok(entity_ref_uuid) = Uuid::parse_str(entity_ref_str)
                {
                    let entity_ref = TripleID(entity_ref_uuid);
                    embedding = embedding.with_entity_ref(entity_ref);
                }

                self.embedding_store
                    .insert(&embedding)
                    .map_err(|e| MemoryError::Storage(e.to_string()))?;

                Ok(embedding.id.clone())
            }

            _ => Err(MemoryError::InvalidArtifactType(artifact_type.to_string())),
        }
    }

    fn recall(&self, query: &str, token: &CapabilityToken) -> Result<Vec<Value>, MemoryError> {
        // Validate capability token allows read operations
        match token.action {
            hkask_types::CapabilityAction::Read
            | hkask_types::CapabilityAction::Execute
            | hkask_types::CapabilityAction::Validate => {}
            _ => {
                return Err(MemoryError::CapabilityDenied(
                    "Token does not grant read access for recall".to_string(),
                ));
            }
        }

        let triples = self
            .triple_store
            .query_by_entity(query)
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
            target: "hkask.memory",
            query = %query,
            results = results.len(),
            "Memory recall"
        );

        Ok(results)
    }
}
