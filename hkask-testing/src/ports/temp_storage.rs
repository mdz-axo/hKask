//! Temporary Storage Port - Inbound port for testing
//!
//! Implements storage port traits with in-memory/temporary storage for testing.
//! Provides isolated, disposable storage for test scenarios.

use hkask_storage::{Blob, BlobStore, Embedding, EmbeddingStore, Triple, TripleStore};
use hkask_types::WebID;
use rusqlite::Connection;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Temporary blob store for testing
pub struct TempBlobStore {
    blobs: RefCell<HashMap<String, Blob>>,
}

impl TempBlobStore {
    pub fn new() -> Self {
        Self {
            blobs: RefCell::new(HashMap::new()),
        }
    }

    pub fn store(&self, blob: Blob) {
        let id = blob.id.clone();
        self.blobs.borrow_mut().insert(id, blob);
    }

    pub fn get(&self, id: &str) -> Option<Blob> {
        self.blobs.borrow().get(id).cloned()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.blobs.borrow().contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.blobs.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.blobs.borrow().is_empty()
    }

    pub fn clear(&self) {
        self.blobs.borrow_mut().clear();
    }

    pub fn all_blobs(&self) -> Vec<Blob> {
        self.blobs.borrow().values().cloned().collect()
    }
}

impl Default for TempBlobStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Temporary triple store for testing
pub struct TempTripleStore {
    triples: RefCell<Vec<Triple>>,
}

impl TempTripleStore {
    pub fn new() -> Self {
        Self {
            triples: RefCell::new(Vec::new()),
        }
    }

    pub fn insert(&self, triple: Triple) {
        self.triples.borrow_mut().push(triple);
    }

    pub fn get_by_entity(&self, entity: &str) -> Vec<Triple> {
        self.triples
            .borrow()
            .iter()
            .filter(|t| t.entity == entity)
            .cloned()
            .collect()
    }

    pub fn get_by_attribute(&self, attribute: &str) -> Vec<Triple> {
        self.triples
            .borrow()
            .iter()
            .filter(|t| t.attribute == attribute)
            .cloned()
            .collect()
    }

    pub fn get_by_entity_and_attribute(&self, entity: &str, attribute: &str) -> Vec<Triple> {
        self.triples
            .borrow()
            .iter()
            .filter(|t| t.entity == entity && t.attribute == attribute)
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.triples.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.triples.borrow().is_empty()
    }

    pub fn clear(&self) {
        self.triples.borrow_mut().clear();
    }

    pub fn all_triples(&self) -> Vec<Triple> {
        self.triples.borrow().clone()
    }
}

impl Default for TempTripleStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Temporary embedding store for testing
pub struct TempEmbeddingStore {
    embeddings: RefCell<HashMap<String, Embedding>>,
}

impl TempEmbeddingStore {
    pub fn new() -> Self {
        Self {
            embeddings: RefCell::new(HashMap::new()),
        }
    }

    pub fn insert(&self, embedding: Embedding) {
        let id = embedding.id.clone();
        self.embeddings.borrow_mut().insert(id, embedding);
    }

    pub fn get(&self, id: &str) -> Option<Embedding> {
        self.embeddings.borrow().get(id).cloned()
    }

    pub fn len(&self) -> usize {
        self.embeddings.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.embeddings.borrow().is_empty()
    }

    pub fn clear(&self) {
        self.embeddings.borrow_mut().clear();
    }

    pub fn all_embeddings(&self) -> Vec<Embedding> {
        self.embeddings.borrow().values().cloned().collect()
    }

    /// Simple cosine similarity search (stub implementation)
    pub fn similarity_search(&self, query: &[f32], k: usize) -> Vec<(Embedding, f32)> {
        let mut scores: Vec<_> = self
            .embeddings
            .borrow()
            .values()
            .map(|e| {
                let similarity = cosine_similarity(query, &e.vector);
                (e.clone(), similarity)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.into_iter().take(k).collect()
    }
}

impl Default for TempEmbeddingStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

/// Helper function for testing cosine similarity
pub fn cosine_similarity_helper(a: &[f32], b: &[f32]) -> f32 {
    cosine_similarity(a, b)
}

impl TempEmbeddingStore {
    /// Helper for tests to calculate similarity directly
    pub fn similarity_search_helper(a: &[f32], b: &[f32]) -> f32 {
        cosine_similarity(a, b)
    }
}

/// Temporary database connection for testing
pub struct TempDatabase {
    pub conn: Rc<Connection>,
}

impl TempDatabase {
    pub fn new() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        Ok(Self {
            conn: Rc::new(conn),
        })
    }

    pub fn blob_store(&self) -> BlobStore {
        BlobStore::new(Rc::clone(&self.conn))
    }

    pub fn triple_store(&self) -> TripleStore {
        TripleStore::new(Rc::clone(&self.conn))
    }

    pub fn embedding_store(&self) -> EmbeddingStore {
        EmbeddingStore::new(Rc::clone(&self.conn))
    }
}

impl Default for TempDatabase {
    fn default() -> Self {
        Self::new().expect("Failed to create in-memory database")
    }
}

/// Test fixture for storage operations
pub struct StorageTestFixture {
    pub blob_store: TempBlobStore,
    pub triple_store: TempTripleStore,
    pub embedding_store: TempEmbeddingStore,
    pub database: TempDatabase,
}

impl StorageTestFixture {
    pub fn new() -> Result<Self, rusqlite::Error> {
        Ok(Self {
            blob_store: TempBlobStore::new(),
            triple_store: TempTripleStore::new(),
            embedding_store: TempEmbeddingStore::new(),
            database: TempDatabase::new()?,
        })
    }

    pub fn create_test_blob(&self, content: &str, content_type: &str) -> Blob {
        let owner = WebID::new();
        Blob::new(content.as_bytes().to_vec(), content_type, owner)
    }

    pub fn create_test_triple(
        &self,
        entity: &str,
        attribute: &str,
        value: serde_json::Value,
    ) -> Triple {
        let owner = WebID::new();
        Triple::new(entity, attribute, value, owner)
    }

    pub fn create_test_embedding(&self, vector: Vec<f32>, model: &str) -> Embedding {
        Embedding::new(vector, model)
    }
}

impl Default for StorageTestFixture {
    fn default() -> Self {
        Self::new().expect("Failed to create storage test fixture")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_temp_blob_store_new() {
        let store = TempBlobStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_temp_blob_store_store_and_get() {
        let store = TempBlobStore::new();
        let owner = WebID::new();
        let blob = Blob::new(b"test data".to_vec(), "text/plain", owner);
        let id = blob.id.clone();

        store.store(blob);
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);
        assert!(store.contains(&id));

        let retrieved = store.get(&id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data, b"test data".to_vec());
    }

    #[test]
    fn test_temp_blob_store_clear() {
        let store = TempBlobStore::new();
        let owner = WebID::new();
        let blob = Blob::new(b"test".to_vec(), "text/plain", owner);
        let id = blob.id.clone();

        store.store(blob);
        assert!(store.contains(&id));

        store.clear();
        assert!(!store.contains(&id));
        assert!(store.is_empty());
    }

    #[test]
    fn test_temp_triple_store_insert() {
        let store = TempTripleStore::new();
        let owner = WebID::new();
        let triple = Triple::new("entity1", "attr1", json!("value1"), owner);

        store.insert(triple);
        assert_eq!(store.len(), 1);
        assert!(!store.is_empty());
    }

    #[test]
    fn test_temp_triple_store_query_by_entity() {
        let store = TempTripleStore::new();
        let owner = WebID::new();

        store.insert(Triple::new(
            "entity1",
            "attr1",
            json!("value1"),
            owner.clone(),
        ));
        store.insert(Triple::new(
            "entity1",
            "attr2",
            json!("value2"),
            owner.clone(),
        ));
        store.insert(Triple::new("entity2", "attr1", json!("value3"), owner));

        let results = store.get_by_entity("entity1");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_temp_triple_store_query_by_attribute() {
        let store = TempTripleStore::new();
        let owner = WebID::new();

        store.insert(Triple::new(
            "entity1",
            "attr1",
            json!("value1"),
            owner.clone(),
        ));
        store.insert(Triple::new(
            "entity2",
            "attr1",
            json!("value2"),
            owner.clone(),
        ));
        store.insert(Triple::new("entity3", "attr2", json!("value3"), owner));

        let results = store.get_by_attribute("attr1");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_temp_embedding_store_insert() {
        let store = TempEmbeddingStore::new();
        let embedding = Embedding::new(vec![0.1, 0.2, 0.3], "test-model");
        let id = embedding.id.clone();

        store.insert(embedding);
        assert_eq!(store.len(), 1);
        assert!(store.get(&id).is_some());
    }

    #[test]
    fn test_temp_embedding_store_similarity_search() {
        let store = TempEmbeddingStore::new();

        // Create embeddings with known vectors
        let emb1 = Embedding::new(vec![1.0, 0.0, 0.0], "test");
        let emb2 = Embedding::new(vec![0.0, 1.0, 0.0], "test");
        let emb3 = Embedding::new(vec![0.0, 0.0, 1.0], "test");

        store.insert(emb1);
        store.insert(emb2);
        store.insert(emb3);

        // Query vector similar to emb1
        let query = vec![0.9, 0.1, 0.0];
        let results = store.similarity_search(&query, 2);

        assert_eq!(results.len(), 2);
        // First result should be most similar (emb1)
        assert!(results[0].1 > results[1].1);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let similarity = cosine_similarity(&a, &b);
        assert!((similarity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let similarity = cosine_similarity(&a, &b);
        assert!(similarity.abs() < 0.001);
    }

    #[test]
    fn test_temp_database_new() {
        let db = TempDatabase::new();
        assert!(db.is_ok());
    }

    #[test]
    fn test_storage_test_fixture_new() {
        let fixture = StorageTestFixture::new();
        assert!(fixture.is_ok());
    }

    #[test]
    fn test_storage_test_fixture_create_test_blob() {
        let fixture = StorageTestFixture::new().unwrap();
        let blob = fixture.create_test_blob("test content", "text/plain");
        assert_eq!(blob.content_type, "text/plain");
        assert_eq!(blob.size, 12);
    }

    #[test]
    fn test_storage_test_fixture_create_test_triple() {
        let fixture = StorageTestFixture::new().unwrap();
        let triple = fixture.create_test_triple("entity", "attr", json!("value"));
        assert_eq!(triple.entity, "entity");
        assert_eq!(triple.attribute, "attr");
    }

    #[test]
    fn test_storage_test_fixture_create_test_embedding() {
        let fixture = StorageTestFixture::new().unwrap();
        let embedding = fixture.create_test_embedding(vec![0.1, 0.2], "test-model");
        assert_eq!(embedding.dimensions, 2);
        assert_eq!(embedding.model, "test-model");
    }
}
