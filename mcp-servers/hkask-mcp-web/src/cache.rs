//! hKask MCP Web — In-memory TTL cache with LRU eviction

use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Clone)]
struct CacheEntry {
    data: serde_json::Value,
    inserted_at: std::time::Instant,
    ttl: Duration,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() > self.ttl
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct CacheKey(pub String);

pub struct ResponseCache {
    entries: RwLock<HashMap<CacheKey, CacheEntry>>,
    max_entries: usize,
    default_ttl: Duration,
}

impl ResponseCache {
    pub fn new(max_entries: usize, default_ttl: Duration) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            max_entries,
            default_ttl,
        }
    }

    pub async fn get(&self, key: &CacheKey) -> Option<serde_json::Value> {
        let entries = self.entries.read().await;
        entries.get(key).and_then(|e| {
            if e.is_expired() {
                None
            } else {
                Some(e.data.clone())
            }
        })
    }

    pub async fn insert(&self, key: CacheKey, data: serde_json::Value) {
        let mut entries = self.entries.write().await;
        if entries.len() >= self.max_entries
            && let Some(oldest_key) = entries
                .iter()
                .min_by_key(|(_, v)| v.inserted_at)
                .map(|(k, _)| k.clone())
        {
            entries.remove(&oldest_key);
        }
        entries.insert(
            key,
            CacheEntry {
                data,
                inserted_at: std::time::Instant::now(),
                ttl: self.default_ttl,
            },
        );
    }
}

pub fn cache_key(strategy: &str, query: &str, params: &serde_json::Value) -> CacheKey {
    let hash = blake3::hash(
        format!(
            "{strategy}:{query}:{}",
            serde_json::to_string(params).unwrap_or_default()
        )
        .as_bytes(),
    );
    CacheKey(hash.to_hex().to_string())
}
