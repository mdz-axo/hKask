//! Memory Storage Adapter
//!
//! Concrete implementation of MemoryStoragePort using hkask-storage crate.

use crate::error::MemoryError;
use crate::ports::MemoryStoragePort;
use hkask_storage::{Database, Embedding, EmbeddingStore, Triple, TripleStore};
use hkask_types::{CapabilityToken, TripleID, Visibility, WebID};
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

impl MemoryStoragePort for MemoryStorageAdapter {
    fn store_artifact(
        &self,
        producer_webid: WebID,
        artifact_type: &str,
        content: Value,
        visibility: &str,
        _token: &CapabilityToken,
    ) -> Result<String, MemoryError> {
        let visibility = match visibility.to_lowercase().as_str() {
            "public" => Visibility::Public,
            "shared" => Visibility::Shared,
            _ => Visibility::Private,
        };

        match artifact_type {
            "episodic_triple" | "semantic_triple" => {
<<<<<<< HEAD
                let entity = content["entity"].as_str().ok_or_else(|| {
                    MemoryError::Serialization("Missing 'entity' field".to_string())
                })?;
                let attribute = content["attribute"].as_str().ok_or_else(|| {
                    MemoryError::Serialization("Missing 'attribute' field".to_string())
                })?;
=======
                // Extract entity, attribute, value from content
                let entity = content["entity"].as_str().ok_or("Missing 'entity' field")?;
                let attribute = content["attribute"]
                    .as_str()
                    .ok_or("Missing 'attribute' field")?;
>>>>>>> origin/main
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

<<<<<<< HEAD
    fn recall(&self, query: &str, _token: &CapabilityToken) -> Result<Vec<Value>, MemoryError> {
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

=======
    fn recall(&self, query: &str, _token: &CapabilityToken) -> Result<Vec<Value>, String> {
        // For now, return empty results
        // TODO: Implement actual search using sqlite-vec
>>>>>>> origin/main
        tracing::debug!(
            target: "hkask.memory",
            query = %query,
            results = results.len(),
            "Memory recall"
        );
<<<<<<< HEAD

        Ok(results)
=======
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::CapabilityToken;
    use serde_json::json;

    #[test]
    fn test_memory_storage_in_memory() {
        let adapter = MemoryStorageAdapter::in_memory().unwrap();
        assert!(true);
    }

    #[test]
    fn test_store_semantic_triple() {
        let adapter = MemoryStorageAdapter::in_memory().unwrap();
        let producer_webid = WebID::new();
        let content = json!({
            "entity": "test-entity",
            "attribute": "test-attribute",
            "value": "test-value"
        });

        let token = CapabilityToken::new(
            hkask_types::CapabilityResource::Tool,
            "test".to_string(),
            hkask_types::CapabilityAction::Execute,
            WebID::new(),
            producer_webid,
            b"test-secret",
        );

        let result =
            adapter.store_artifact(producer_webid, "semantic_triple", content, "public", &token);

        assert!(result.is_ok());
    }

    #[test]
    fn test_store_episodic_triple() {
        let adapter = MemoryStorageAdapter::in_memory().unwrap();
        let producer_webid = WebID::new();
        let content = json!({
            "entity": "test-entity",
            "attribute": "test-attribute",
            "value": "test-value"
        });

        let token = CapabilityToken::new(
            hkask_types::CapabilityResource::Tool,
            "test".to_string(),
            hkask_types::CapabilityAction::Execute,
            WebID::new(),
            producer_webid,
            b"test-secret",
        );

        let result = adapter.store_artifact(
            producer_webid,
            "episodic_triple",
            content,
            "private",
            &token,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_store_embedding() {
        let adapter = MemoryStorageAdapter::in_memory().unwrap();
        let producer_webid = WebID::new();
        let content = json!({
            "vector": [0.1, 0.2, 0.3, 0.4, 0.5],
            "model": "test-model"
        });

        let token = CapabilityToken::new(
            hkask_types::CapabilityResource::Tool,
            "test".to_string(),
            hkask_types::CapabilityAction::Execute,
            WebID::new(),
            producer_webid,
            b"test-secret",
        );

        let result = adapter.store_artifact(producer_webid, "embedding", content, "public", &token);

        assert!(result.is_ok());
    }

    #[test]
    fn test_recall_memory() {
        let adapter = MemoryStorageAdapter::in_memory().unwrap();
        let token = CapabilityToken::new(
            hkask_types::CapabilityResource::Tool,
            "test".to_string(),
            hkask_types::CapabilityAction::Execute,
            WebID::new(),
            WebID::new(),
            b"test-secret",
        );

        let result = adapter.recall("test query", &token);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
>>>>>>> origin/main
    }
}
