//! Memory Storage Adapter
//!
//! Concrete implementation of MemoryStoragePort using hkask-storage crate.

use crate::pod::MemoryStoragePort;
use hkask_storage::{Database, Embedding, EmbeddingStore, Triple, TripleStore};
use hkask_types::{CapabilityToken, WebID};
use rusqlite;
use std::rc::Rc;

/// Memory Storage Adapter — Concrete implementation for artifact persistence
pub struct MemoryStorageAdapter {
    db: Database,
}

impl MemoryStorageAdapter {
    /// Create new memory storage adapter
    pub fn new(db: Database) -> Result<Self, String> {
        Ok(Self { db })
    }

    /// Create from database path
    pub fn from_path(path: &str, passphrase: &str) -> Result<Self, String> {
        let db = Database::open(path, passphrase).map_err(|e| e.to_string())?;
        Ok(Self { db })
    }

    /// Create in-memory database for testing
    pub fn in_memory() -> Result<Self, String> {
        let db = Database::in_memory().map_err(|e| e.to_string())?;
        Ok(Self { db })
    }

    /// Get connection for creating stores
    fn conn(&self) -> Rc<rusqlite::Connection> {
        self.db.conn_rc()
    }
}

impl MemoryStoragePort for MemoryStorageAdapter {
    fn store_artifact(
        &self,
        producer_webid: WebID,
        artifact_type: &str,
        content: serde_json::Value,
        _visibility: &str,
        _token: &CapabilityToken,
    ) -> Result<String, String> {
        match artifact_type {
            "episodic_triple" | "semantic_triple" => {
                let entity = content["entity"].as_str().unwrap_or("unknown").to_string();
                let attribute = content["attribute"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                let value = content["value"].clone();
                let confidence = content["confidence"].as_f64().unwrap_or(1.0);

                let mut triple = Triple::new(&entity, &attribute, value, producer_webid);
                triple = triple.with_confidence(confidence);

                if artifact_type == "episodic_triple" {
                    triple = triple.with_perspective(producer_webid);
                }

                let store = TripleStore::new(self.conn());
                store.insert(&triple).map_err(|e| e.to_string())?;

                Ok(triple.id.0.to_string())
            }
            "embedding" => {
                let vector: Vec<f32> = content["vector"]
                    .as_array()
                    .ok_or("Missing vector field")?
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect();

                let model = content["model"].as_str().unwrap_or("default").to_string();

                let mut embedding = Embedding::new(vector, &model);
                embedding = embedding.with_entity_ref(hkask_types::TripleID::new());

                let store = EmbeddingStore::new(self.conn());
                store.insert(&embedding).map_err(|e| e.to_string())?;

                Ok(embedding.id)
            }
            _ => Err(format!("Unknown artifact type: {}", artifact_type)),
        }
    }

    fn recall(
        &self,
        _query: &str,
        _token: &CapabilityToken,
    ) -> Result<Vec<serde_json::Value>, String> {
        // TODO: Implement search when TripleStore and EmbeddingStore have search methods
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::{CapabilityAction, CapabilityResource};

    #[test]
    fn test_memory_storage_adapter_in_memory() {
        let _adapter = MemoryStorageAdapter::in_memory().unwrap();
        assert!(true);
    }

    #[test]
    fn test_store_semantic_triple() {
        let adapter = MemoryStorageAdapter::in_memory().unwrap();
        let producer = WebID::new();
        let token = CapabilityToken::new(
            CapabilityResource::Cascade,
            "memory:store".to_string(),
            CapabilityAction::Write,
            WebID::new(),
            producer,
            b"test-secret",
        );

        let content = serde_json::json!({
            "entity": "test-entity",
            "attribute": "test-attribute",
            "value": "test-value",
            "confidence": 0.95
        });

        let result =
            adapter.store_artifact(producer, "semantic_triple", content, "private", &token);
        assert!(result.is_ok());
    }

    #[test]
    fn test_store_episodic_triple() {
        let adapter = MemoryStorageAdapter::in_memory().unwrap();
        let producer = WebID::new();
        let token = CapabilityToken::new(
            CapabilityResource::Cascade,
            "memory:store".to_string(),
            CapabilityAction::Write,
            WebID::new(),
            producer,
            b"test-secret",
        );

        let content = serde_json::json!({
            "entity": "episodic-entity",
            "attribute": "experienced",
            "value": "test-event",
            "confidence": 1.0
        });

        let result =
            adapter.store_artifact(producer, "episodic_triple", content, "private", &token);
        assert!(result.is_ok());
    }

    #[test]
    fn test_recall() {
        let adapter = MemoryStorageAdapter::in_memory().unwrap();
        let producer = WebID::new();
        let token = CapabilityToken::new(
            CapabilityResource::Cascade,
            "memory:recall".to_string(),
            CapabilityAction::Read,
            WebID::new(),
            producer,
            b"test-secret",
        );

        let result = adapter.recall("test-query", &token);
        assert!(result.is_ok());
    }
}
