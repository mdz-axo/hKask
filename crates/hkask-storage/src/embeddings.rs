//! Embedding storage with vector similarity search via sqlite-vec

use hkask_types::TripleID;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EmbeddingError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },
}

#[derive(Debug, Clone)]
pub struct Embedding {
    pub id: String,
    pub entity_ref: Option<TripleID>,
    pub vector: Vec<f32>,
    pub dimensions: usize,
    pub model: String,
}

impl Embedding {
    pub fn new(vector: Vec<f32>, model: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            entity_ref: None,
            dimensions: vector.len(),
            vector,
            model: model.to_string(),
        }
    }

    #[must_use]
    pub fn with_entity_ref(mut self, entity_ref: TripleID) -> Self {
        self.entity_ref = Some(entity_ref);
        self
    }
}

#[derive(Debug, Clone)]
pub struct KnnResult {
    pub id: String,
    pub distance: f64,
}

pub struct EmbeddingStore {
    conn: Arc<Mutex<Connection>>,
}

impl EmbeddingStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    fn vector_to_bytes(vector: &[f32]) -> Vec<u8> {
        vector.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    pub fn insert(&self, embedding: &Embedding) -> Result<(), EmbeddingError> {
        let conn = self.conn.lock().unwrap();
        let vector_bytes = Self::vector_to_bytes(&embedding.vector);
        let entity_ref = embedding.entity_ref.map(|e| e.0.to_string());

        conn.execute(
            "INSERT INTO embeddings (id, entity_ref, vector, dimensions, model) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![embedding.id, entity_ref, vector_bytes, embedding.dimensions as i64, embedding.model],
        )?;

        let expected_dim = crate::database::embedding_dim();
        if embedding.dimensions == expected_dim {
            conn.execute(
                "INSERT INTO vec_embeddings (id, embedding) VALUES (?1, ?2)",
                rusqlite::params![embedding.id, vector_bytes],
            )?;
        }

        Ok(())
    }

    pub fn knn_search(&self, query: &[f32], k: usize) -> Result<Vec<KnnResult>, EmbeddingError> {
        let expected_dim = crate::database::embedding_dim();
        if query.len() != expected_dim {
            return Err(EmbeddingError::DimensionMismatch {
                expected: expected_dim,
                got: query.len(),
            });
        }

        let conn = self.conn.lock().unwrap();
        let query_bytes = Self::vector_to_bytes(query);

        let mut stmt = conn.prepare(
            "SELECT id, distance FROM vec_embeddings WHERE embedding MATCH ?1 AND k = ?2 ORDER BY distance",
        )?;

        let results = stmt
            .query_map(rusqlite::params![query_bytes, k as i64], |row| {
                Ok(KnnResult {
                    id: row.get(0)?,
                    distance: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    pub fn delete(&self, id: &str) -> Result<(), EmbeddingError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM embeddings WHERE id = ?1",
            rusqlite::params![id],
        )?;
        let _ = conn.execute(
            "DELETE FROM vec_embeddings WHERE id = ?1",
            rusqlite::params![id],
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    #[test]
    fn test_insert_and_knn() {
        let db = Database::in_memory().unwrap();
        let store = EmbeddingStore::new(db.conn_arc());

        let dim = crate::database::embedding_dim();
        let v1: Vec<f32> = (0..dim).map(|i| i as f32 * 0.01).collect();
        let v2: Vec<f32> = (0..dim).map(|i| (i as f32 + 100.0) * 0.01).collect();
        let v3: Vec<f32> = v1.iter().map(|x| x + 0.001).collect();

        let e1 = Embedding::new(v1.clone(), "test-model");
        let e2 = Embedding::new(v2, "test-model");
        let e3 = Embedding::new(v3, "test-model");

        store.insert(&e1).unwrap();
        store.insert(&e2).unwrap();
        store.insert(&e3).unwrap();

        let results = store.knn_search(&v1, 2).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, e1.id);
    }

    #[test]
    fn test_dimension_mismatch() {
        let db = Database::in_memory().unwrap();
        let store = EmbeddingStore::new(db.conn_arc());
        let wrong_dim = vec![1.0f32; 3];
        let result = store.knn_search(&wrong_dim, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete() {
        let db = Database::in_memory().unwrap();
        let store = EmbeddingStore::new(db.conn_arc());

        let dim = crate::database::embedding_dim();
        let v: Vec<f32> = (0..dim).map(|i| i as f32 * 0.01).collect();
        let e = Embedding::new(v, "test-model");
        store.insert(&e).unwrap();

        store.delete(&e.id).unwrap();

        let results = store.knn_search(&e.vector, 5).unwrap();
        assert!(results.is_empty());
    }
}
