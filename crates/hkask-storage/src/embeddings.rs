//! Embedding store — sqlite-vec backed KNN similarity search.
//!
//! Two tables: `embeddings` (metadata) + `vec_embeddings` (vec0 virtual table).
use hkask_rsolidity as rs;
use crate::Store;
use crate::lock_helpers::lock_mutex;
use hkask_types::InfrastructureError;
/// Stored embedding record.
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
/// Default embedding dimension (must match `vec_embeddings` schema).
const DEFAULT_DIM: usize = 1024;
/// EmbeddingStore — sqlite-vec backed embedding storage.
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
    /// Create a new embedding store.
    ///
    /// REQ: P3-sto-embedding-new
    /// expect: "The system provides durable storage for embedding data" [P3]
    /// \[P3\] Motivating: Generative Space — create embedding store
    /// pre:  conn is a valid SQLite connection
    /// post: returns EmbeddingStore with default dimension
    #[rs::contract(id = "P3-sto-embedding-new", principle = "P3")]
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            conn,
            dim: DEFAULT_DIM,
        }
    }
    /// Create with a custom embedding dimension.
    /// Create an embedding store with a specific vector dimension.
    ///
    /// REQ: P3-sto-embedding-new-with-dim
    /// expect: "The system provides durable storage for embedding data" [P3]
    /// \[P3\] Motivating: Generative Space — create embedding store with dimension
    /// pre:  conn is valid, dim > 0
    /// post: returns EmbeddingStore with specified dimension
    #[rs::contract(id = "P3-sto-embedding-new-with-dim", principle = "P3")]
    pub fn with_dim(conn: Arc<Mutex<Connection>>, dim: usize) -> Self {
        Self { conn, dim }
    }
    /// Encode f32 vector as binary blob for sqlite-vec.
    fn encode_vector(vector: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(vector.len() * 4);
        for &f in vector {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        bytes
    }
    /// Decode binary blob back into f32 vector.
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
    /// Validate vector dimension.
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
    /// Store embedding in both tables (single transaction). Returns the embedding ID.
    /// Store an embedding vector.
    ///
    /// REQ: P3-sto-embedding-store
    /// expect: "The system provides durable storage for embedding data" [P3]
    /// \[P3\] Motivating: Generative Space — store an embedding vector
    /// pre:  entity_ref is non-empty, vector matches store dimension, model is non-empty
    /// post: embedding stored and indexed by entity_ref
    /// post: returns embedding ID
    #[rs::contract(id = "P3-sto-embedding-store", principle = "P3")]
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
    /// Retrieve an embedding by entity_ref.
    ///
    /// REQ: P3-sto-embedding-get
    /// expect: "The system provides durable storage for embedding data" [P3]
    /// \[P3\] Motivating: Generative Space — retrieve embedding by entity
    /// pre:  entity_ref is non-empty
    /// post: returns StoredEmbedding if found
    /// post: returns Err(NotFound) if not found
    #[rs::contract(id = "P3-sto-embedding-get", principle = "P3")]
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
    /// KNN search using sqlite-vec MATCH operator.
    /// Search for similar embeddings by vector distance.
    ///
    /// REQ: P3-sto-embedding-search
    /// expect: "The system provides durable storage for embedding data" [P3]
    /// \[P3\] Motivating: Generative Space — vector similarity search
    /// pre:  query_vector matches store dimension, limit > 0
    /// post: returns Vec<SimilarityResult> ordered by ascending distance
    #[rs::contract(id = "P3-sto-embedding-search", principle = "P3")]
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
    /// Delete embedding from both tables (single transaction).
    /// Delete an embedding by entity_ref.
    ///
    /// REQ: P3-sto-embedding-delete
    /// expect: "The system provides durable storage for embedding data" [P3]
    /// \[P3\] Motivating: Generative Space — delete embedding
    /// pre:  entity_ref is non-empty
    /// post: embedding deleted if existed
    #[rs::contract(id = "P3-sto-embedding-delete", principle = "P3")]
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
    /// Count stored embeddings.
    ///
    /// REQ: P3-sto-embedding-count
    /// expect: "The system provides durable storage for embedding data" [P3]
    /// \[P8\] Motivating: Semantic Grounding — count embeddings
    /// post: returns total count of embeddings
    #[rs::contract(id = "P3-sto-embedding-count", principle = "P3")]
    pub fn count(&self) -> Result<usize, EmbeddingError> {
        let conn = lock_mutex(&self.conn)?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM embeddings", [], |row| row.get(0))?;
        Ok(count as usize)
    }
    /// Query entity_refs matching a prefix.
    /// Query entity_refs by prefix.
    ///
    /// REQ: P3-sto-embedding-prefix
    /// expect: "The system provides durable storage for embedding data" [P3]
    /// \[P3\] Motivating: Generative Space — query entity refs by prefix
    /// pre:  prefix is non-empty
    /// post: returns Vec of entity_refs matching prefix
    #[rs::contract(id = "P3-sto-embedding-prefix", principle = "P3")]
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
