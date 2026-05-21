//! Unit tests for hkask-storage crate
//! Migrated from inline tests in production code
//! Expanded to cover BlobStore, TripleStore, EmbeddingStore, and GitCas

use hkask_storage::{Blob, Embedding, Triple};
use hkask_testing::{StorageTestFixture, TempBlobStore, TempEmbeddingStore, TempTripleStore};
use hkask_types::{TripleID, Visibility, WebID};
use serde_json::json;

mod blob_tests {
    use super::*;

    #[test]
    fn test_blob_new() {
        let data = b"Hello!".to_vec();
        let owner = WebID::new();
        let blob = Blob::new(data.clone(), "text/plain", owner);
        assert!(blob.verify());
    }

    #[test]
    fn test_blob_verify() {
        let data = b"Test content".to_vec();
        let owner = WebID::new();
        let blob = Blob::new(data.clone(), "text/plain", owner);
        assert!(blob.verify());
    }

    #[test]
    fn test_blob_hash_consistency() {
        let data = b"Consistent hash test".to_vec();
        let owner = WebID::new();
        let blob1 = Blob::new(data.clone(), "text/plain", owner.clone());
        let blob2 = Blob::new(data.clone(), "text/plain", owner);
        assert_eq!(blob1.blake3_hash, blob2.blake3_hash);
    }

    #[test]
    fn test_blob_size() {
        let data = b"1234567890".to_vec();
        let owner = WebID::new();
        let blob = Blob::new(data.clone(), "text/plain", owner);
        assert_eq!(blob.size, 10);
    }

    #[test]
    fn test_blob_content_type() {
        let data = b"JSON data".to_vec();
        let owner = WebID::new();
        let blob = Blob::new(data, "application/json", owner);
        assert_eq!(blob.content_type, "application/json");
    }

    #[test]
    fn test_blob_visibility_default() {
        let data = b"Private by default".to_vec();
        let owner = WebID::new();
        let blob = Blob::new(data, "text/plain", owner);
        assert_eq!(blob.visibility, Visibility::Private);
    }

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
    fn test_temp_blob_store_multiple_blobs() {
        let store = TempBlobStore::new();
        let owner = WebID::new();

        for i in 0..5 {
            let data = format!("blob {}", i).into_bytes();
            let blob = Blob::new(data, "text/plain", owner.clone());
            store.store(blob);
        }

        assert_eq!(store.len(), 5);
    }

    #[test]
    fn test_temp_blob_store_all_blobs() {
        let store = TempBlobStore::new();
        let owner = WebID::new();

        store.store(Blob::new(b"blob1".to_vec(), "text/plain", owner.clone()));
        store.store(Blob::new(b"blob2".to_vec(), "text/plain", owner));

        let all = store.all_blobs();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_storage_fixture_create_blob() {
        let fixture = StorageTestFixture::new().unwrap();
        let blob = fixture.create_test_blob("test content", "text/plain");
        assert_eq!(blob.content_type, "text/plain");
        assert!(blob.verify());
    }
}

mod triple_tests {
    use super::*;

    #[test]
    fn test_triple_new() {
        let owner = WebID::new();
        let triple = Triple::new("entity1", "attribute1", json!("value1"), owner);
        assert_eq!(triple.entity, "entity1");
        assert_eq!(triple.attribute, "attribute1");
        assert_eq!(triple.confidence, 1.0);
        assert!(triple.is_semantic());
    }

    #[test]
    fn test_triple_with_confidence() {
        let owner = WebID::new();
        let triple = Triple::new("e", "a", json!("v"), owner).with_confidence(0.85);
        assert_eq!(triple.confidence, 0.85);
    }

    #[test]
    fn test_triple_with_perspective() {
        let owner = WebID::new();
        let perspective = WebID::new();
        let triple = Triple::new("e", "a", json!("v"), owner).with_perspective(perspective);
        assert!(triple.perspective.is_some());
        assert!(triple.is_episodic());
    }

    #[test]
    fn test_triple_with_visibility() {
        let owner = WebID::new();
        let triple = Triple::new("e", "a", json!("v"), owner).with_visibility(Visibility::Public);
        assert_eq!(triple.visibility, Visibility::Public);
    }

    #[test]
    fn test_triple_is_episodic() {
        let owner = WebID::new();
        let perspective = WebID::new();
        let triple = Triple::new("e", "a", json!("v"), owner).with_perspective(perspective);
        assert!(triple.is_episodic());
        assert!(!triple.is_semantic());
    }

    #[test]
    fn test_triple_is_semantic() {
        let owner = WebID::new();
        let triple = Triple::new("e", "a", json!("v"), owner);
        assert!(triple.is_semantic());
        assert!(!triple.is_episodic());
    }

    #[test]
    fn test_temp_triple_store_new() {
        let store = TempTripleStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
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
    fn test_temp_triple_store_query_by_entity_and_attribute() {
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
            "attr1",
            json!("value2"),
            owner.clone(),
        ));
        store.insert(Triple::new("entity2", "attr1", json!("value3"), owner));

        let results = store.get_by_entity_and_attribute("entity1", "attr1");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_temp_triple_store_clear() {
        let store = TempTripleStore::new();
        let owner = WebID::new();

        store.insert(Triple::new("e", "a", json!("v"), owner));
        assert_eq!(store.len(), 1);

        store.clear();
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn test_storage_fixture_create_triple() {
        let fixture = StorageTestFixture::new().unwrap();
        let triple = fixture.create_test_triple("entity", "attr", json!("value"));
        assert_eq!(triple.entity, "entity");
        assert_eq!(triple.attribute, "attr");
    }
}

mod embedding_tests {
    use super::*;

    #[test]
    fn test_embedding_new() {
        let vector = vec![0.1, 0.2, 0.3];
        let embedding = Embedding::new(vector.clone(), "test-model");
        assert_eq!(embedding.dimensions, 3);
        assert_eq!(embedding.model, "test-model");
    }

    #[test]
    fn test_embedding_with_entity_ref() {
        let vector = vec![0.1, 0.2];
        let entity_ref = TripleID::new();
        let embedding = Embedding::new(vector, "test-model").with_entity_ref(entity_ref);
        assert!(embedding.entity_ref.is_some());
    }

    #[test]
    fn test_embedding_id_generation() {
        let vector = vec![0.1, 0.2];
        let embedding1 = Embedding::new(vector.clone(), "test-model");
        let embedding2 = Embedding::new(vector, "test-model");
        assert_ne!(embedding1.id, embedding2.id);
    }

    #[test]
    fn test_temp_embedding_store_new() {
        let store = TempEmbeddingStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
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
    fn test_temp_embedding_store_multiple() {
        let store = TempEmbeddingStore::new();

        for i in 0..3 {
            let vector = vec![i as f32 * 0.1];
            let embedding = Embedding::new(vector, "test-model");
            store.insert(embedding);
        }

        assert_eq!(store.len(), 3);
    }

    #[test]
    fn test_temp_embedding_store_clear() {
        let store = TempEmbeddingStore::new();
        let embedding = Embedding::new(vec![0.1, 0.2], "test");
        let id = embedding.id.clone();

        store.insert(embedding);
        assert!(store.get(&id).is_some());

        store.clear();
        assert!(store.get(&id).is_none());
        assert!(store.is_empty());
    }

    #[test]
    fn test_temp_embedding_store_all_embeddings() {
        let store = TempEmbeddingStore::new();

        store.insert(Embedding::new(vec![0.1], "m"));
        store.insert(Embedding::new(vec![0.2], "m"));

        let all = store.all_embeddings();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let similarity = TempEmbeddingStore::similarity_search_helper(&a, &b);
        assert!((similarity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let similarity = TempEmbeddingStore::similarity_search_helper(&a, &b);
        assert!(similarity.abs() < 0.001);
    }

    #[test]
    fn test_storage_fixture_create_embedding() {
        let fixture = StorageTestFixture::new().unwrap();
        let embedding = fixture.create_test_embedding(vec![0.1, 0.2], "test-model");
        assert_eq!(embedding.dimensions, 2);
        assert_eq!(embedding.model, "test-model");
    }
}

mod git_cas_tests {
    use super::*;

    #[test]
    fn test_triple_id_new() {
        let id1 = TripleID::new();
        let id2 = TripleID::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_webid_new() {
        let webid1 = WebID::new();
        let webid2 = WebID::new();
        assert_ne!(webid1, webid2);
    }

    #[test]
    fn test_visibility_variants() {
        assert_eq!(Visibility::Private.as_str(), "private");
        assert_eq!(Visibility::Public.as_str(), "public");
        assert_eq!(Visibility::Shared.as_str(), "shared");
    }

    #[test]
    fn test_visibility_parse() {
        assert_eq!(Visibility::parse_str("private"), Some(Visibility::Private));
        assert_eq!(Visibility::parse_str("public"), Some(Visibility::Public));
        assert_eq!(Visibility::parse_str("shared"), Some(Visibility::Shared));
        assert_eq!(Visibility::parse_str("invalid"), None);
    }
}
