//! Embedding storage with vector similarity search via sqlite-vec

use hkask_types::{InfrastructureError, TripleID};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EmbeddingError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),
    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },
}

impl From<rusqlite::Error> for EmbeddingError {
    fn from(e: rusqlite::Error) -> Self {
        EmbeddingError::Infra(InfrastructureError::Database(e.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct Embedding {
    pub id: String,
    pub entity_ref: Option<TripleID>,
    pub vector: Vec<f32>,
    pub dimensions: usize,
    pub model: String,
}

#[derive(Debug, Clone)]
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
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
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

    pub fn search(
        &self,
        query: &[f32],
        limit: usize,
    ) -> Result<Vec<(Embedding, f64)>, EmbeddingError> {
        let expected_dim = crate::database::embedding_dim();
        if query.len() != expected_dim {
            return Err(EmbeddingError::DimensionMismatch {
                expected: expected_dim,
                got: query.len(),
            });
        }

        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let query_bytes = Self::vector_to_bytes(query);

        let mut stmt = conn.prepare(
            "SELECT e.id, e.entity_ref, e.vector, e.dimensions, e.model, v.distance \
             FROM vec_embeddings v \
             JOIN embeddings e ON v.id = e.id \
             WHERE v.embedding MATCH ?1 AND k = ?2 \
             ORDER BY v.distance",
        )?;

        let results = stmt
            .query_map(rusqlite::params![query_bytes, limit as i64], |row| {
                let id: String = row.get(0)?;
                let entity_ref: Option<String> = row.get(1)?;
                let vector_bytes: Vec<u8> = row.get(2)?;
                let dimensions: i64 = row.get(3)?;
                let model: String = row.get(4)?;
                let distance: f64 = row.get(5)?;

                let vector: Vec<f32> = vector_bytes
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();

                Ok((
                    Embedding {
                        id,
                        entity_ref: entity_ref.and_then(|s| s.parse().ok()).map(TripleID),
                        vector,
                        dimensions: dimensions as usize,
                        model,
                    },
                    distance,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }
}
