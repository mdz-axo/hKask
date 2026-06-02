//! Prompt Caching with TTL and LRU Eviction
//!
//! Cache key: BLAKE3 hash of (prompt, model, params)
//! TTL by category: instruct=24h, thinking=1h, embedding=30d
//! LRU eviction when cache >100MB

use hkask_types::LLMParameters;
use hkask_types::ports::InferenceResult;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub(crate) struct CacheTtlConfig {
    pub instruct: Duration,
    pub thinking: Duration,
    pub categorization: Duration,
    pub embedding: Duration,
    pub specialist: Duration,
}

impl Default for CacheTtlConfig {
    fn default() -> Self {
        Self {
            instruct: Duration::from_secs(24 * 60 * 60),
            thinking: Duration::from_secs(60 * 60),
            categorization: Duration::from_secs(24 * 60 * 60),
            embedding: Duration::from_secs(30 * 24 * 60 * 60),
            specialist: Duration::from_secs(24 * 60 * 60),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PromptCacheConfig {
    pub max_size_mb: i64,
    pub ttl_config: CacheTtlConfig,
}

impl Default for PromptCacheConfig {
    fn default() -> Self {
        Self {
            max_size_mb: 100,
            ttl_config: CacheTtlConfig::default(),
        }
    }
}

pub(crate) struct PromptCache {
    conn: Arc<Mutex<Connection>>,
    config: PromptCacheConfig,
    current_size: Arc<std::sync::atomic::AtomicI64>,
}

#[derive(Error, Debug)]
pub(crate) enum CacheError {
    #[error(transparent)]
    Infra(#[from] hkask_types::InfrastructureError),

    #[error("Cache miss")]
    Miss,
}

impl From<rusqlite::Error> for CacheError {
    fn from(e: rusqlite::Error) -> Self {
        hkask_types::InfrastructureError::Database(e.to_string()).into()
    }
}

impl From<serde_json::Error> for CacheError {
    fn from(e: serde_json::Error) -> Self {
        hkask_types::InfrastructureError::from(e).into()
    }
}

impl PromptCache {
    pub fn generate_key(prompt: &str, model: &str, params: &LLMParameters) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        hasher.update(model.as_bytes());
        hasher.update(params.temperature.to_le_bytes());
        hasher.update(params.top_p.to_le_bytes());
        hasher.update(params.top_k.to_le_bytes());
        hasher.update(params.max_tokens.to_le_bytes());
        let hash = hasher.finalize();
        hex::encode(&hash[..16])
    }

    fn get_ttl(&self, model: &str) -> Duration {
        if model.contains("thinking") || model.contains("reason") {
            self.config.ttl_config.thinking
        } else if model.contains("embedding") || model.contains("embed") {
            self.config.ttl_config.embedding
        } else if model.contains("categorization") || model.contains("classify") {
            self.config.ttl_config.categorization
        } else if model.contains("code") || model.contains("specialist") {
            self.config.ttl_config.specialist
        } else {
            self.config.ttl_config.instruct
        }
    }

    pub fn get(&self, key: &str) -> Result<InferenceResult, CacheError> {
        let now = Instant::now().elapsed().as_secs() as i64;

        let conn = self
            .conn
            .lock()
            .map_err(|_| CacheError::Infra(hkask_types::InfrastructureError::LockPoisoned))?;
        let mut stmt = conn.prepare(
            "SELECT result, size_bytes FROM prompt_cache WHERE key = ?1 AND expires_at > ?2",
        )?;

        let result: Result<(String, i64), _> =
            stmt.query_row(params![key, now], |row| Ok((row.get(0)?, row.get(1)?)));

        match result {
            Ok((result_json, _size)) => {
                conn.execute(
                    "UPDATE prompt_cache SET access_count = access_count + 1, last_accessed = ?1 WHERE key = ?2",
                    params![now, key],
                )?;

                let inference_result: InferenceResult = serde_json::from_str(&result_json)?;
                debug!(target: "hkask.cache", key = %key, "Cache hit");
                Ok(inference_result)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                debug!(target: "hkask.cache", key = %key, "Cache miss");
                Err(CacheError::Miss)
            }
            Err(e) => Err(hkask_types::InfrastructureError::Database(e.to_string()).into()),
        }
    }

    pub fn put(
        &self,
        key: &str,
        prompt: &str,
        model: &str,
        result: &InferenceResult,
    ) -> Result<(), CacheError> {
        let now = Instant::now().elapsed().as_secs() as i64;
        let ttl = self.get_ttl(model);
        let expires_at = now + ttl.as_secs() as i64;

        let result_json = serde_json::to_string(result)?;
        let size_bytes = result_json.len() as i64 + prompt.len() as i64 + model.len() as i64;

        self.evict_if_needed(size_bytes)?;

        self.conn.lock().map_err(|_| CacheError::Infra(hkask_types::InfrastructureError::LockPoisoned))?.execute(
            "INSERT OR REPLACE INTO prompt_cache (key, prompt, model, result, created_at, expires_at, size_bytes, access_count, last_accessed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8)",
            params![key, prompt, model, result_json, now, expires_at, size_bytes, now],
        )?;

        self.current_size
            .fetch_add(size_bytes, std::sync::atomic::Ordering::Relaxed);
        info!(target: "hkask.cache", key = %key, size = %size_bytes, "Cache entry added");

        Ok(())
    }

    fn evict_if_needed(&self, new_size: i64) -> Result<(), CacheError> {
        let max_size = self.config.max_size_mb * 1024 * 1024;
        let current = self.current_size.load(std::sync::atomic::Ordering::Relaxed);

        if current + new_size <= max_size {
            return Ok(());
        }

        let conn = self
            .conn
            .lock()
            .map_err(|_| CacheError::Infra(hkask_types::InfrastructureError::LockPoisoned))?;
        let mut stmt = conn.prepare(
            "SELECT key, size_bytes FROM prompt_cache
             ORDER BY access_count ASC, last_accessed ASC
             LIMIT 10",
        )?;

        let mut to_delete = Vec::new();
        let mut freed = 0i64;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;

        for row in rows.flatten() {
            to_delete.push((row.0.clone(), row.1));
            freed += row.1;

            if freed >= new_size {
                break;
            }
        }

        for (key, size) in to_delete {
            conn.execute("DELETE FROM prompt_cache WHERE key = ?1", params![key])?;
            self.current_size
                .fetch_sub(size, std::sync::atomic::Ordering::Relaxed);
            info!(target: "hkask.cache", key = %key, "Cache entry evicted");
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub entry_count: i64,
    pub total_size_bytes: i64,
    pub total_accesses: i64,
}
