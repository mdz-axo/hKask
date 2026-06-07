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

use crate::Store;
use crate::lock_helpers::lock_mutex;
use hkask_types::InfrastructureError;

/// Stored embedding record with metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredEmbedding {
    pub id: String,
    pub entity_ref: String,
    pub vector: Vec<f32>,
    pub model: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimilarityResult {
    pub embedding: StoredEmbedding,
    pub distance: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("Embedding not found: {0}")]
    NotFound(String),
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
    #[error("Storage error: {0}")]
    Storage(#[source] rusqlite::Error),
    #[error(transparent)]
    Infrastructure(#[from] hkask_types::InfrastructureError),
    #[error("Corrupt vector data: {0}")]
    Decode(String),
}

impl From<rusqlite::Error> for EmbeddingError {
    fn from(e: rusqlite::Error) -> Self {
        EmbeddingError::Storage(e)
    }
}
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

impl Store for EmbeddingStore {
    fn conn_arc(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    fn lock_conn(
        &self,
    ) -> std::result::Result<std::sync::MutexGuard<'_, Connection>, InfrastructureError> {
        lock_mutex(&self.conn)
    }
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
                EmbeddingError::Decode("corrupt vector blob: chunk not 4 bytes".into())
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

impl EmbeddingStore {
    /// Store an embedding vector indexed by entity reference.
    ///
    /// Inserts into both the `embeddings` metadata table and the
    /// `vec_embeddings` virtual table in a single transaction.
    /// Returns the generated embedding ID.
    pub fn store(
        &self,
        entity_ref: &str,
        vector: &[f32],
        model: &str,
    ) -> Result<String, EmbeddingError> {
        self.validate_dim(vector)?;

        let id = hkask_types::EmbeddingID::new().to_string();
        let blob = Self::encode_vector(vector);
        let dim = vector.len() as i32;

        let conn = lock_mutex(&self.conn)?;

        conn.execute_batch("BEGIN TRANSACTION;")?;

        // Insert metadata into embeddings table
        let result = conn.execute(
            "INSERT INTO embeddings (id, entity_ref, vector, dimensions, model) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, entity_ref, blob, dim, model],
        );

        if let Err(e) = result {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(EmbeddingError::Storage(e));
        }

        // Insert into vec_embeddings virtual table for KNN search.
        // sqlite-vec expects the vector as a blob in the same float32 LE encoding.
        let vec_result = conn.execute(
            "INSERT INTO vec_embeddings (id, embedding) VALUES (?1, ?2)",
            rusqlite::params![id, &blob],
        );

        if let Err(e) = vec_result {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(EmbeddingError::Storage(e));
        }

        conn.execute_batch("COMMIT;")?;

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
    pub fn get(&self, entity_ref: &str) -> Result<StoredEmbedding, EmbeddingError> {
        let conn = lock_mutex(&self.conn)?;

        let mut stmt = conn
            .prepare("SELECT id, entity_ref, vector, dimensions, model FROM embeddings WHERE entity_ref = ?1")?;

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
            Err(e) => Err(EmbeddingError::Storage(e)),
        }
    }

    /// Search for the K nearest neighbors of a query vector.
    ///
    /// Uses sqlite-vec's KNN query: `WHERE embedding MATCH ? ORDER BY distance LIMIT ?`.
    /// Results are joined with the `embeddings` metadata table to produce
    /// `StoredEmbedding` records.
    pub fn search(
        &self,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<SimilarityResult>, EmbeddingError> {
        self.validate_dim(query_vector)?;

        let query_blob = Self::encode_vector(query_vector);
        let conn = lock_mutex(&self.conn)?;

        let mut stmt = conn.prepare(
            "SELECT v.id, v.distance, e.entity_ref, e.vector, e.model
                 FROM vec_embeddings v
                 JOIN embeddings e ON v.id = e.id
                 WHERE v.embedding MATCH ?1 AND v.k = ?2
                 ORDER BY v.distance",
        )?;

        let rows = stmt.query_map(rusqlite::params![&query_blob, limit as i64], |row| {
            let id: String = row.get(0)?;
            let distance: f64 = row.get(1)?;
            let entity_ref: String = row.get(2)?;
            let vector_blob: Vec<u8> = row.get(3)?;
            let model: String = row.get(4)?;
            Ok((id, distance, entity_ref, vector_blob, model))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let (id, distance, entity_ref, blob, model) = row.map_err(EmbeddingError::Storage)?;
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
    pub fn delete(&self, entity_ref: &str) -> Result<(), EmbeddingError> {
        let conn = lock_mutex(&self.conn)?;

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
            Err(e) => return Err(EmbeddingError::Storage(e)),
        };

        conn.execute_batch("BEGIN TRANSACTION;")?;

        if let Err(e) = conn.execute(
            "DELETE FROM vec_embeddings WHERE id = ?1",
            rusqlite::params![id],
        ) {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(EmbeddingError::Storage(e));
        }

        if let Err(e) = conn.execute(
            "DELETE FROM embeddings WHERE id = ?1",
            rusqlite::params![id],
        ) {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(EmbeddingError::Storage(e));
        }

        conn.execute_batch("COMMIT;")?;

        tracing::debug!(
            target: "storage.embedding",
            id = %id,
            entity_ref = %entity_ref,
            "Embedding deleted"
        );

        Ok(())
    }

    /// Count total embeddings stored.
    pub fn count(&self) -> Result<usize, EmbeddingError> {
        let conn = lock_mutex(&self.conn)?;

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM embeddings", [], |row| row.get(0))?;

        Ok(count as usize)
    }

    /// Query all entity_refs matching a prefix.
    ///
    /// Uses SQL LIKE with the prefix + '%' pattern.
    /// Efficient when the `idx_embeddings_entity_ref` index exists.
    pub fn query_by_prefix(&self, prefix: &str) -> Result<Vec<String>, EmbeddingError> {
        let conn = lock_mutex(&self.conn)?;

        let pattern = format!("{}%", prefix);
        let mut stmt =
            conn.prepare("SELECT entity_ref FROM embeddings WHERE entity_ref LIKE ?1")?;

        let rows = stmt.query_map(rusqlite::params![pattern], |row| row.get(0))?;

        let mut refs = Vec::new();
        for row in rows {
            let entity_ref: String = row.map_err(EmbeddingError::Storage)?;
            refs.push(entity_ref);
        }

        Ok(refs)
    }
}
