//! Embedding storage with vector similarity search

use hkask_types::TripleID;
use rusqlite::Connection;
use std::rc::Rc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EmbeddingError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
}

/// Embedding vector
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

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_entity_ref(mut self, entity_ref: TripleID) -> Self {
        self.entity_ref = Some(entity_ref);
        self
    }
}

/// Embedding store with vector similarity search
pub struct EmbeddingStore {
    conn: Rc<Connection>,
}

impl EmbeddingStore {
    pub fn new(conn: Rc<Connection>) -> Self {
        Self { conn }
    }

    /// Insert an embedding
    pub fn insert(&self, embedding: &Embedding) -> Result<(), EmbeddingError> {
        let vector_bytes: Vec<u8> = embedding
            .vector
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        let entity_ref = embedding.entity_ref.map(|e| e.0.to_string());

        self.conn.execute(
            "INSERT INTO embeddings (id, entity_ref, vector, dimensions, model)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                embedding.id,
                entity_ref,
                vector_bytes,
                embedding.dimensions as i64,
                embedding.model,
            ],
        )?;

        Ok(())
    }
}

