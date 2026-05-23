//! Model Registry stub

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModelCategory {
    Thinking,
    Instruct,
    Categorization,
    Embedding,
    Specialist,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    Active,
    Inactive,
    Degraded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub category: ModelCategory,
    pub provider: String,
    pub context_length: u32,
    pub tokens_per_second: Option<f64>,
    pub energy_per_token: Option<f64>,
    pub capabilities: Vec<String>,
    pub recommended_for: Vec<String>,
    pub status: ModelStatus,
}

pub struct ModelRegistryStore {
    _conn: Arc<Connection>,
}

#[derive(Error, Debug)]
pub enum ModelRegistryError {
    #[error("Database error: {0}")]
    Database(String),
}

impl From<rusqlite::Error> for ModelRegistryError {
    fn from(e: rusqlite::Error) -> Self {
        ModelRegistryError::Database(e.to_string())
    }
}

impl ModelRegistryStore {
    pub fn new(conn: Arc<Connection>) -> Result<Self, ModelRegistryError> {
        Ok(Self { _conn: conn })
    }

    pub fn register(&self, _entry: &ModelEntry) -> Result<(), ModelRegistryError> {
        Ok(())
    }

    pub fn get(&self, _id: &str) -> Result<Option<ModelEntry>, ModelRegistryError> {
        Ok(None)
    }

    pub fn list_by_category(
        &self,
        _category: &ModelCategory,
    ) -> Result<Vec<ModelEntry>, ModelRegistryError> {
        Ok(Vec::new())
    }
}
