//! Embedding store placeholder (vector methods removed — zero consumers)

use rusqlite::Connection;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct EmbeddingStore {
    /// Reserved for future embedding vector operations
    _conn: Arc<Mutex<Connection>>,
}

impl EmbeddingStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { _conn: conn }
    }
}
