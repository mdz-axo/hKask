//! Temporary database for testing

use hkask_storage::{Database, EmbeddingStore, TripleStore};
use std::sync::{Arc, Mutex};

/// Temporary database fixture for testing
pub struct TempDatabase {
    pub conn: Arc<Mutex<rusqlite::Connection>>,
}

impl TempDatabase {
    pub fn new() -> Result<Self, rusqlite::Error> {
        let conn = rusqlite::Connection::open_in_memory()?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn blob_store(&self) -> hkask_storage::BlobStore {
        hkask_storage::BlobStore::new(Arc::clone(&self.conn))
    }

    pub fn triple_store(&self) -> TripleStore {
        TripleStore::new(Arc::clone(&self.conn))
    }

    pub fn embedding_store(&self) -> EmbeddingStore {
        EmbeddingStore::new(Arc::clone(&self.conn))
    }
}

impl Default for TempDatabase {
    fn default() -> Self {
        Self::new().expect("Failed to create in-memory database")
    }
}

/// Storage test fixture with pre-populated data
pub struct StorageTestFixture {
    pub db: TempDatabase,
}

impl StorageTestFixture {
    pub fn new() -> Result<Self, rusqlite::Error> {
        Ok(Self {
            db: TempDatabase::new()?,
        })
    }

    pub fn create_test_triple(&self) -> hkask_types::TripleID {
        use hkask_types::{Triple, Visibility, WebID};
        use serde_json::json;

        let triple = Triple::new("test", "test", json!("test"), WebID::new())
            .with_visibility(Visibility::Public);
        let id = triple.id;
        let store = self.db.triple_store();
        store.insert(&triple).unwrap();
        id
    }

    pub fn create_test_embedding(&self) -> String {
        use hkask_storage::Embedding;

        let embedding = Embedding::new(vec![0.1, 0.2, 0.3], "test");
        let id = embedding.id.clone();
        let store = self.db.embedding_store();
        store.insert(&embedding).unwrap();
        id
    }

    pub fn create_test_blob(&self) -> String {
        use hkask_storage::Blob;
        use hkask_types::{Visibility, WebID};

        let blob = Blob::new(b"test".to_vec(), "text/plain", WebID::new());
        let id = blob.id.clone();
        let _store = self.db.blob_store();
        id
    }

    pub fn clear(&self) {
        // In-memory DB is cleared on drop
    }
}

impl Default for StorageTestFixture {
    fn default() -> Self {
        Self::new().expect("Failed to create storage test fixture")
    }
}