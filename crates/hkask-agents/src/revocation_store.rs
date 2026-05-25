//! Revocation Store — Persistent token revocation tracking
//!
//! Tracks revoked capability tokens in SQLite for persistence across restarts.

use rusqlite::params;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RevocationError {
    #[error("Database error: {0}")]
    Database(String),
}

/// Persistent store for revoked capability tokens
pub struct RevocationStore {
    conn: Arc<tokio::sync::Mutex<rusqlite::Connection>>,
}

impl RevocationStore {
    /// Create a new revocation store from an existing connection
    pub fn new(conn: rusqlite::Connection) -> Result<Self, RevocationError> {
        // Create table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS revoked_tokens (
                token_id TEXT PRIMARY KEY,
                revoked_at INTEGER NOT NULL,
                reason TEXT
            )",
            [],
        )
        .map_err(|e| RevocationError::Database(e.to_string()))?;

        Ok(Self {
            conn: Arc::new(tokio::sync::Mutex::new(conn)),
        })
    }

    /// Create an in-memory revocation store for testing
    pub fn in_memory() -> Result<Self, RevocationError> {
        let conn = rusqlite::Connection::open_in_memory()
            .map_err(|e| RevocationError::Database(e.to_string()))?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS revoked_tokens (
                token_id TEXT PRIMARY KEY,
                revoked_at INTEGER NOT NULL,
                reason TEXT
            )",
            [],
        )
        .map_err(|e| RevocationError::Database(e.to_string()))?;

        Ok(Self {
            conn: Arc::new(tokio::sync::Mutex::new(conn)),
        })
    }

    /// Record a token revocation
    pub async fn revoke(&self, token_id: &str, reason: Option<&str>) -> Result<(), RevocationError> {
        let conn = self.conn.lock().await;
        let now = chrono::Utc::now().timestamp();
        
        conn.execute(
            "INSERT OR REPLACE INTO revoked_tokens (token_id, revoked_at, reason) VALUES (?1, ?2, ?3)",
            params![token_id, now, reason],
        )
        .map_err(|e| RevocationError::Database(e.to_string()))?;

        Ok(())
    }

    /// Check if a token has been revoked
    pub async fn is_revoked(&self, token_id: &str) -> Result<bool, RevocationError> {
        let conn = self.conn.lock().await;
        
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM revoked_tokens WHERE token_id = ?1",
                params![token_id],
                |row| row.get(0),
            )
            .map_err(|e| RevocationError::Database(e.to_string()))?;

        Ok(count > 0)
    }

    /// Get all revoked token IDs
    pub async fn list_revoked(&self) -> Result<Vec<String>, RevocationError> {
        let conn = self.conn.lock().await;
        
        let mut stmt = conn
            .prepare("SELECT token_id FROM revoked_tokens")
            .map_err(|e| RevocationError::Database(e.to_string()))?;

        let tokens = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| RevocationError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tokens)
    }
}
