//! Embedding store — vector storage and KNN similarity search via sqlite-vec
//!
//! Stores embedding vectors in two tables:
//! - `embeddings`: metadata (entity_ref, model, dimensions, created_at)
//! - `vec_embeddings`: sqlite-vec virtual table for KNN search
//!
//! The `vec_embeddings` table provides O(log n) approximate nearest neighbor
//! search using the sqlite-vec extension. The `embeddings` table provides
//! the join key and metadata.
//!
//! **Spec Reference:** Architecture v0.21.0 §2.3, sqlite-vec integration

use hkask_types::ports::{EmbeddingError, EmbeddingPort, SimilarityResult, StoredEmbedding};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Default embedding dimension (must match `vec_embeddings` schema)
const DEFAULT_DIM: usize = 384;

/// EmbeddingStore — sqlite-vec backed embedding storage
///
/// Stores and queries embedding vectors using sqlite-vec's `vec0` virtual
/// table for KNN similarity search. Each embedding is identified by its
/// `entity_ref` (typically a triple ID) and indexed for fast nearest-neighbor
/// retrieval.
pub struct EmbeddingStore {
    conn: Arc<Mutex<Connection>>,
    dim: usize,
}

impl EmbeddingStore {
    /// Create a new EmbeddingStore sharing an existing database connection.
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            conn,
            dim: DEFAULT_DIM,
        }
    }

    /// Create with a custom embedding dimension.
    ///
    /// Must match the dimension declared in the `vec_embeddings` schema.
    /// Use only if you've overridden `HKASK_EMBEDDING_DIM` at database creation.
    pub fn with_dim(conn: Arc<Mutex<Connection>>, dim: usize) -> Self {
        Self { conn, dim }
    }

    /// Encode a float vector as a compact binary blob for sqlite-vec.
    ///
    /// sqlite-vec expects `float[N]` vectors as a byte blob in native-endian
    /// f32 layout. This is the standard encoding for the `vec0` virtual table.
    fn encode_vector(vector: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(vector.len() * 4);
        for &f in vector {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        bytes
    }

    /// Decode a binary blob back into a float vector.
    fn decode_vector(blob: &[u8], expected_dim: usize) -> Result<Vec<f32>, EmbeddingError> {
        if blob.len() != expected_dim * 4 {
            return Err(EmbeddingError::DimensionMismatch {
                expected: expected_dim * 4,
                actual: blob.len(),
            });
        }
        let mut vector = Vec::with_capacity(expected_dim);
        for chunk in blob.chunks_exact(4) {
            let f = f32::from_le_bytes(chunk.try_into().map_err(|_| {
                EmbeddingError::Storage("corrupt vector blob: chunk not 4 bytes".into())
            })?);
            vector.push(f);
        }
        Ok(vector)
    }

    /// Validate vector dimension matches configured dimension.
    fn validate_dim(&self, vector: &[f32]) -> Result<(), EmbeddingError> {
        if vector.len() != self.dim {
            return Err(EmbeddingError::DimensionMismatch {
                expected: self.dim,
                actual: vector.len(),
            });
        }
        Ok(())
    }
}

impl EmbeddingPort for EmbeddingStore {
    /// Store an embedding vector indexed by entity reference.
    ///
    /// Inserts into both the `embeddings` metadata table and the
    /// `vec_embeddings` virtual table in a single transaction.
    /// Returns the generated embedding ID.
    fn store(
        &self,
        entity_ref: &str,
        vector: &[f32],
        model: &str,
    ) -> Result<String, EmbeddingError> {
        self.validate_dim(vector)?;

        let id = hkask_types::EmbeddingID::new().to_string();
        let blob = Self::encode_vector(vector);
        let dim = vector.len() as i32;

        let conn = self
            .conn
            .lock()
            .map_err(|e| EmbeddingError::Storage(format!("connection lock poisoned: {e}")))?;

        conn.execute_batch("BEGIN TRANSACTION;")
            .map_err(|e| EmbeddingError::Storage(e.to_string()))?;

        // Insert metadata into embeddings table
        let result = conn.execute(
            "INSERT INTO embeddings (id, entity_ref, vector, dimensions, model) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, entity_ref, blob, dim, model],
        );

        if let Err(e) = result {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(EmbeddingError::Storage(e.to_string()));
        }

        // Insert into vec_embeddings virtual table for KNN search.
        // sqlite-vec expects the vector as a blob in the same float32 LE encoding.
        let vec_result = conn.execute(
            "INSERT INTO vec_embeddings (id, embedding) VALUES (?1, ?2)",
            rusqlite::params![id, &blob],
        );

        if let Err(e) = vec_result {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(EmbeddingError::Storage(format!(
                "vec_embeddings insert failed: {e}"
            )));
        }

        conn.execute_batch("COMMIT;")
            .map_err(|e| EmbeddingError::Storage(e.to_string()))?;

        tracing::debug!(
            target: "storage.embedding",
            id = %id,
            entity_ref = %entity_ref,
            model = %model,
            dimensions = dim,
            "Embedding stored"
        );

        Ok(id)
    }

    /// Retrieve an embedding by entity reference.
    fn get(&self, entity_ref: &str) -> Result<StoredEmbedding, EmbeddingError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| EmbeddingError::Storage(format!("connection lock poisoned: {e}")))?;

        let mut stmt = conn
            .prepare("SELECT id, entity_ref, vector, dimensions, model FROM embeddings WHERE entity_ref = ?1")
            .map_err(|e| EmbeddingError::Storage(e.to_string()))?;

        let result = stmt.query_row(rusqlite::params![entity_ref], |row| {
            let id: String = row.get(0)?;
            let entity_ref: String = row.get(1)?;
            let vector_blob: Vec<u8> = row.get(2)?;
            let _dimensions: i32 = row.get(3)?;
            let model: String = row.get(4)?;
            Ok((id, entity_ref, vector_blob, model))
        });

        match result {
            Ok((id, er, blob, model)) => {
                let vector = Self::decode_vector(&blob, self.dim)?;
                Ok(StoredEmbedding {
                    id,
                    entity_ref: er,
                    vector,
                    model,
                })
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(EmbeddingError::NotFound(entity_ref.to_string()))
            }
            Err(e) => Err(EmbeddingError::Storage(e.to_string())),
        }
    }

    /// Search for the K nearest neighbors of a query vector.
    ///
    /// Uses sqlite-vec's KNN query: `WHERE embedding MATCH ? ORDER BY distance LIMIT ?`.
    /// Results are joined with the `embeddings` metadata table to produce
    /// `StoredEmbedding` records.
    fn search(
        &self,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<SimilarityResult>, EmbeddingError> {
        self.validate_dim(query_vector)?;

        let query_blob = Self::encode_vector(query_vector);
        let conn = self
            .conn
            .lock()
            .map_err(|e| EmbeddingError::Storage(format!("connection lock poisoned: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT v.id, v.distance, e.entity_ref, e.vector, e.model
                 FROM vec_embeddings v
                 JOIN embeddings e ON v.id = e.id
                 WHERE v.embedding MATCH ?1
                 ORDER BY v.distance
                 LIMIT ?2",
            )
            .map_err(|e| EmbeddingError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params![&query_blob, limit], |row| {
                let id: String = row.get(0)?;
                let distance: f64 = row.get(1)?;
                let entity_ref: String = row.get(2)?;
                let vector_blob: Vec<u8> = row.get(3)?;
                let model: String = row.get(4)?;
                Ok((id, distance, entity_ref, vector_blob, model))
            })
            .map_err(|e| EmbeddingError::Storage(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let (id, distance, entity_ref, blob, model) =
                row.map_err(|e| EmbeddingError::Storage(e.to_string()))?;
            let vector = Self::decode_vector(&blob, self.dim)?;
            results.push(SimilarityResult {
                embedding: StoredEmbedding {
                    id,
                    entity_ref,
                    vector,
                    model,
                },
                distance,
            });
        }

        Ok(results)
    }

    /// Delete an embedding by entity reference.
    ///
    /// Removes from both the `embeddings` metadata table and the
    /// `vec_embeddings` virtual table in a single transaction.
    fn delete(&self, entity_ref: &str) -> Result<(), EmbeddingError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| EmbeddingError::Storage(format!("connection lock poisoned: {e}")))?;

        // Look up the embedding ID first
        let id_result: Result<String, _> = conn.query_row(
            "SELECT id FROM embeddings WHERE entity_ref = ?1",
            rusqlite::params![entity_ref],
            |row| row.get(0),
        );

        let id = match id_result {
            Ok(id) => id,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err(EmbeddingError::NotFound(entity_ref.to_string()));
            }
            Err(e) => return Err(EmbeddingError::Storage(e.to_string())),
        };

        conn.execute_batch("BEGIN TRANSACTION;")
            .map_err(|e| EmbeddingError::Storage(e.to_string()))?;

        if let Err(e) = conn.execute(
            "DELETE FROM vec_embeddings WHERE id = ?1",
            rusqlite::params![id],
        ) {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(EmbeddingError::Storage(e.to_string()));
        }

        if let Err(e) = conn.execute(
            "DELETE FROM embeddings WHERE id = ?1",
            rusqlite::params![id],
        ) {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(EmbeddingError::Storage(e.to_string()));
        }

        conn.execute_batch("COMMIT;")
            .map_err(|e| EmbeddingError::Storage(e.to_string()))?;

        tracing::debug!(
            target: "storage.embedding",
            id = %id,
            entity_ref = %entity_ref,
            "Embedding deleted"
        );

        Ok(())
    }

    /// Count total embeddings stored.
    fn count(&self) -> Result<usize, EmbeddingError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| EmbeddingError::Storage(format!("connection lock poisoned: {e}")))?;

        let count: usize = conn
            .query_row("SELECT COUNT(*) FROM embeddings", [], |row| row.get(0))
            .map_err(|e| EmbeddingError::Storage(e.to_string()))?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::Database;

    fn test_store() -> EmbeddingStore {
        let db = Database::in_memory().expect("in-memory db");
        let conn = db.conn_arc();
        EmbeddingStore::new(conn)
    }

    fn test_vector(seed: f32) -> Vec<f32> {
        // Generate a 384-dim vector with a simple pattern for testing
        (0..384).map(|i| seed + i as f32 * 0.001).collect()
    }

    #[test]
    fn store_and_retrieve_embedding() {
        let store = test_store();
        let vector = test_vector(0.1);

        let id = store.store("triple_001", &vector, "test-model").unwrap();
        assert!(!id.is_empty());

        let retrieved = store.get("triple_001").unwrap();
        assert_eq!(retrieved.entity_ref, "triple_001");
        assert_eq!(retrieved.model, "test-model");
        assert_eq!(retrieved.vector.len(), 384);

        // Verify vector round-trip (f32 LE encoding should be lossless)
        for (i, (a, b)) in vector.iter().zip(retrieved.vector.iter()).enumerate() {
            assert!(
                (a - b).abs() < f32::EPSILON,
                "Mismatch at index {i}: expected {a}, got {b}"
            );
        }
    }

    #[test]
    fn get_nonexistent_returns_not_found() {
        let store = test_store();
        let result = store.get("does_not_exist");
        assert!(result.is_err());
        match result.unwrap_err() {
            EmbeddingError::NotFound(ref s) => assert_eq!(s, "does_not_exist"),
            other => panic!("Expected NotFound, got: {other:?}"),
        }
    }

    #[test]
    fn store_dimension_mismatch() {
        let store = test_store();
        let short_vector = vec![0.1f32; 128]; // Wrong dimension
        let result = store.store("triple_bad", &short_vector, "test-model");
        assert!(result.is_err());
        match result.unwrap_err() {
            EmbeddingError::DimensionMismatch { expected, actual } => {
                assert_eq!(expected, 384);
                assert_eq!(actual, 128);
            }
            other => panic!("Expected DimensionMismatch, got: {other:?}"),
        }
    }

    #[test]
    fn search_finds_nearest_neighbors() {
        let store = test_store();

        // Store several embeddings with different seeds
        let vec_a = test_vector(0.0);
        let vec_b = test_vector(0.5);
        let vec_c = test_vector(1.0);

        store.store("triple_a", &vec_a, "test-model").unwrap();
        store.store("triple_b", &vec_b, "test-model").unwrap();
        store.store("triple_c", &vec_c, "test-model").unwrap();

        // Query with a vector close to vec_b
        let query = test_vector(0.5);
        let results = store.search(&query, 2).unwrap();

        assert_eq!(results.len(), 2);
        // Closest should be triple_b (exact match, distance ≈ 0)
        assert_eq!(results[0].embedding.entity_ref, "triple_b");
        assert!(
            results[0].distance < 0.01,
            "Exact match should have near-zero distance: {}",
            results[0].distance
        );
    }

    #[test]
    fn search_dimension_mismatch() {
        let store = test_store();
        let bad_query = vec![0.0f32; 64];
        let result = store.search(&bad_query, 5);
        assert!(result.is_err());
        match result.unwrap_err() {
            EmbeddingError::DimensionMismatch { expected, actual } => {
                assert_eq!(expected, 384);
                assert_eq!(actual, 64);
            }
            other => panic!("Expected DimensionMismatch, got: {other:?}"),
        }
    }

    #[test]
    fn delete_embedding() {
        let store = test_store();
        let vector = test_vector(0.3);

        store.store("triple_del", &vector, "test-model").unwrap();
        assert_eq!(store.count().unwrap(), 1);

        store.delete("triple_del").unwrap();
        assert_eq!(store.count().unwrap(), 0);

        // Verify it's gone from both tables
        assert!(store.get("triple_del").is_err());
    }

    #[test]
    fn delete_nonexistent_returns_not_found() {
        let store = test_store();
        let result = store.delete("never_existed");
        assert!(result.is_err());
        match result.unwrap_err() {
            EmbeddingError::NotFound(ref s) => assert_eq!(s, "never_existed"),
            other => panic!("Expected NotFound, got: {other:?}"),
        }
    }

    #[test]
    fn count_returns_total() {
        let store = test_store();
        assert_eq!(store.count().unwrap(), 0);

        store.store("t1", &test_vector(0.0), "m1").unwrap();
        store.store("t2", &test_vector(0.1), "m2").unwrap();
        store.store("t3", &test_vector(0.2), "m2").unwrap();

        assert_eq!(store.count().unwrap(), 3);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let original: Vec<f32> = (0..384).map(|i| i as f32 * 0.01).collect();
        let encoded = EmbeddingStore::encode_vector(&original);
        let decoded = EmbeddingStore::decode_vector(&encoded, 384).unwrap();
        for (i, (a, b)) in original.iter().zip(decoded.iter()).enumerate() {
            assert!(
                (a - b).abs() < f32::EPSILON,
                "Roundtrip mismatch at index {i}: {a} vs {b}"
            );
        }
    }
}
