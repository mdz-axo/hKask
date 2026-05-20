//! Okapi Capability Cache
//!
//! Caches Okapi capabilities in SQLite for fast access and capability validation.
//! Capabilities are fetched from Okapi and cached with TTL for performance.

use chrono::{DateTime, Utc};
use hkask_ensemble::ports::OkapiCapabilities;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

/// Capability cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityCacheEntry {
    pub id: Uuid,
    pub runner_type: String,
    pub lora_hot_swap: bool,
    pub token_probs: bool,
    pub grammar_native: bool,
    pub advanced_sampling: bool,
    pub fetched_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub okapi_url: String,
}

impl CapabilityCacheEntry {
    /// Create new cache entry from Okapi capabilities
    pub fn from_capabilities(
        capabilities: OkapiCapabilities,
        okapi_url: String,
        ttl: Duration,
    ) -> Self {
        let now = Utc::now();
        let expires_at =
            now + chrono::Duration::from_std(ttl).unwrap_or(chrono::Duration::hours(1));

        Self {
            id: Uuid::new_v4(),
            runner_type: capabilities.runner_type,
            lora_hot_swap: capabilities.lora_hot_swap,
            token_probs: capabilities.token_probs,
            grammar_native: capabilities.grammar_native,
            advanced_sampling: capabilities.advanced_sampling,
            fetched_at: now,
            expires_at,
            okapi_url,
        }
    }

    /// Convert to Okapi capabilities
    pub fn to_capabilities(&self) -> OkapiCapabilities {
        OkapiCapabilities {
            runner_type: self.runner_type.clone(),
            lora_hot_swap: self.lora_hot_swap,
            token_probs: self.token_probs,
            grammar_native: self.grammar_native,
            advanced_sampling: self.advanced_sampling,
        }
    }

    /// Check if entry is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if entry is valid (not expired)
    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }
}

/// Capability cache error
#[derive(Debug, Error)]
pub enum CapabilityCacheError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Capability not found")]
    NotFound,

    #[error("Capability expired")]
    Expired,

    #[error("Fetch error: {0}")]
    FetchError(String),
}

/// Okapi capability cache
pub struct CapabilityCache {
    conn: Connection,
    default_ttl: Duration,
}

impl CapabilityCache {
    /// Create new capability cache with database connection
    pub fn new(conn: Connection, default_ttl: Duration) -> Result<Self, CapabilityCacheError> {
        let cache = Self { conn, default_ttl };
        cache.initialize()?;
        Ok(cache)
    }

    /// Initialize database schema
    fn initialize(&self) -> Result<(), CapabilityCacheError> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS okapi_capabilities (
                id TEXT PRIMARY KEY,
                runner_type TEXT NOT NULL,
                lora_hot_swap BOOLEAN NOT NULL,
                token_probs BOOLEAN NOT NULL,
                grammar_native BOOLEAN NOT NULL,
                advanced_sampling BOOLEAN NOT NULL,
                fetched_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                okapi_url TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Create index for expiration checking
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_capabilities_expires ON okapi_capabilities(expires_at)",
            [],
        )?;

        Ok(())
    }

    /// Get cached capabilities for Okapi URL
    pub fn get(&self, okapi_url: &str) -> Result<OkapiCapabilities, CapabilityCacheError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, runner_type, lora_hot_swap, token_probs, grammar_native, 
                    advanced_sampling, fetched_at, expires_at, okapi_url
             FROM okapi_capabilities
             WHERE okapi_url = ?
             ORDER BY fetched_at DESC
             LIMIT 1",
        )?;

        let entry: CapabilityCacheEntry = stmt
            .query_row(params![okapi_url], |row| {
                Ok(CapabilityCacheEntry {
                    id: Uuid::parse_str(row.get::<_, String>(0)?.as_str())
                        .unwrap_or(Uuid::new_v4()),
                    runner_type: row.get(1)?,
                    lora_hot_swap: row.get(2)?,
                    token_probs: row.get(3)?,
                    grammar_native: row.get(4)?,
                    advanced_sampling: row.get(5)?,
                    fetched_at: row.get::<_, String>(6)?.parse().unwrap_or(Utc::now()),
                    expires_at: row.get::<_, String>(7)?.parse().unwrap_or(Utc::now()),
                    okapi_url: row.get(8)?,
                })
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => CapabilityCacheError::NotFound,
                _ => CapabilityCacheError::Database(e),
            })?;

        if entry.is_expired() {
            return Err(CapabilityCacheError::Expired);
        }

        Ok(entry.to_capabilities())
    }

    /// Cache capabilities for Okapi URL
    pub fn cache(
        &self,
        capabilities: OkapiCapabilities,
        okapi_url: &str,
    ) -> Result<CapabilityCacheEntry, CapabilityCacheError> {
        let entry = CapabilityCacheEntry::from_capabilities(
            capabilities,
            okapi_url.to_string(),
            self.default_ttl,
        );

        self.conn.execute(
            "INSERT OR REPLACE INTO okapi_capabilities 
             (id, runner_type, lora_hot_swap, token_probs, grammar_native, 
              advanced_sampling, fetched_at, expires_at, okapi_url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                entry.id.to_string(),
                entry.runner_type,
                entry.lora_hot_swap,
                entry.token_probs,
                entry.grammar_native,
                entry.advanced_sampling,
                entry.fetched_at.to_rfc3339(),
                entry.expires_at.to_rfc3339(),
                entry.okapi_url,
            ],
        )?;

        Ok(entry)
    }

    /// Get or fetch capabilities (cache-aside pattern)
    pub async fn get_or_fetch<F, Fut>(
        &self,
        okapi_url: &str,
        fetch_fn: F,
    ) -> Result<OkapiCapabilities, CapabilityCacheError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<OkapiCapabilities, String>>,
    {
        // Try cache first
        match self.get(okapi_url) {
            Ok(caps) => return Ok(caps),
            Err(CapabilityCacheError::NotFound) | Err(CapabilityCacheError::Expired) => {
                // Cache miss or expired, fetch from source
            }
            Err(e) => return Err(e),
        }

        // Fetch from source
        let capabilities = fetch_fn()
            .await
            .map_err(|e| CapabilityCacheError::FetchError(e))?;

        // Cache the result
        self.cache(capabilities.clone(), okapi_url)?;

        Ok(capabilities)
    }

    /// Clear expired entries
    pub fn clear_expired(&self) -> Result<usize, CapabilityCacheError> {
        let now = Utc::now().to_rfc3339();
        let affected = self.conn.execute(
            "DELETE FROM okapi_capabilities WHERE expires_at < ?",
            params![now],
        )?;

        Ok(affected)
    }

    /// Clear all cached capabilities
    pub fn clear_all(&self) -> Result<(), CapabilityCacheError> {
        self.conn.execute("DELETE FROM okapi_capabilities", [])?;
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<CacheStats, CapabilityCacheError> {
        let total: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM okapi_capabilities", [], |row| {
                    row.get(0)
                })?;

        let expired: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM okapi_capabilities WHERE expires_at < ?",
            params![Utc::now().to_rfc3339()],
            |row| row.get(0),
        )?;

        let valid = total - expired;

        Ok(CacheStats {
            total_entries: total as usize,
            valid_entries: valid as usize,
            expired_entries: expired as usize,
        })
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub expired_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_cache() -> (CapabilityCache, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_capabilities.db");
        let conn = Connection::open(db_path).unwrap();
        let cache = CapabilityCache::new(conn, Duration::from_secs(3600)).unwrap();
        (cache, temp_dir)
    }

    #[test]
    fn test_capability_cache_entry_creation() {
        let capabilities = OkapiCapabilities {
            runner_type: "ollamarunner".to_string(),
            lora_hot_swap: true,
            token_probs: true,
            grammar_native: true,
            advanced_sampling: true,
        };

        let entry = CapabilityCacheEntry::from_capabilities(
            capabilities.clone(),
            "http://localhost:11435".to_string(),
            Duration::from_secs(3600),
        );

        assert_eq!(entry.runner_type, "ollamarunner");
        assert!(entry.lora_hot_swap);
        assert!(entry.token_probs);
        assert!(!entry.is_expired());
    }

    #[test]
    fn test_capability_cache_roundtrip() {
        let (cache, _temp_dir) = create_test_cache();

        let capabilities = OkapiCapabilities {
            runner_type: "ollamarunner".to_string(),
            lora_hot_swap: true,
            token_probs: false,
            grammar_native: true,
            advanced_sampling: false,
        };

        // Cache capabilities
        let entry = cache
            .cache(capabilities.clone(), "http://localhost:11435")
            .unwrap();

        assert_eq!(entry.okapi_url, "http://localhost:11435");

        // Retrieve capabilities
        let retrieved = cache.get("http://localhost:11435").unwrap();

        assert_eq!(retrieved.runner_type, "ollamarunner");
        assert_eq!(retrieved.lora_hot_swap, true);
        assert_eq!(retrieved.token_probs, false);
    }

    #[test]
    fn test_capability_cache_not_found() {
        let (cache, _temp_dir) = create_test_cache();

        let result = cache.get("http://nonexistent:11435");
        assert!(matches!(result, Err(CapabilityCacheError::NotFound)));
    }

    #[test]
    fn test_capability_cache_clear() {
        let (cache, _temp_dir) = create_test_cache();

        // Add some entries
        let capabilities = OkapiCapabilities {
            runner_type: "ollamarunner".to_string(),
            lora_hot_swap: true,
            token_probs: true,
            grammar_native: true,
            advanced_sampling: true,
        };

        cache
            .cache(capabilities.clone(), "http://localhost:11435")
            .unwrap();
        cache
            .cache(capabilities.clone(), "http://localhost:11436")
            .unwrap();

        // Check stats
        let stats = cache.stats().unwrap();
        assert_eq!(stats.total_entries, 2);

        // Clear all
        cache.clear_all().unwrap();

        let stats = cache.stats().unwrap();
        assert_eq!(stats.total_entries, 0);
    }

    #[test]
    fn test_capability_cache_stats() {
        let (cache, _temp_dir) = create_test_cache();

        let capabilities = OkapiCapabilities {
            runner_type: "ollamarunner".to_string(),
            lora_hot_swap: true,
            token_probs: true,
            grammar_native: true,
            advanced_sampling: true,
        };

        cache
            .cache(capabilities.clone(), "http://localhost:11435")
            .unwrap();

        let stats = cache.stats().unwrap();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.valid_entries, 1);
        assert_eq!(stats.expired_entries, 0);
    }
}
