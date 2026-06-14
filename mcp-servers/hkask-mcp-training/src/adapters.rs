//! LoRA adapter lifecycle management.
//!
//! Manages LoRA adapter artifacts — store, retrieve, list, delete — with metadata
//! linking back to originating `TrainingJob`, base model, and dataset provenance.
//!
//! Adapters are stored as blobs in `hkask-storage` with metadata in a dedicated
//! `lora_adapters` table. Referenced from `hkask-templates` registry for model
//! composition (base + adapter = effective model).

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

// ── LoRA adapter metadata ──────────────────────────────────────────────────

/// Metadata for a stored LoRA adapter.
///
/// Stores provenance information linking the adapter back to its originating
/// training job, base model, and dataset. The actual adapter weights are stored
/// as a blob in `hkask-storage`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoRAAdapter {
    /// Unique adapter identifier (UUIDv4).
    pub id: String,
    /// Human-readable name for the adapter.
    pub name: String,
    /// Base model this adapter was trained on (provider-prefixed, e.g., "OM/qwen3:8b").
    pub base_model: String,
    /// Content hash of the dataset used for training.
    pub dataset_hash: String,
    /// ID of the originating training job.
    pub training_job_id: String,
    /// Creation timestamp.
    pub created_at: i64,
    /// Size of adapter weights in bytes.
    pub size_bytes: u64,
    /// Training metrics (loss, perplexity, etc.).
    pub metrics: Option<AdapterMetrics>,
}

/// Training metrics recorded at adapter creation time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterMetrics {
    /// Final training loss.
    pub loss: Option<f32>,
    /// Perplexity at end of training.
    pub perplexity: Option<f32>,
    /// Training duration in seconds.
    pub training_duration_secs: Option<u64>,
    /// Number of tokens processed.
    pub tokens_processed: Option<u64>,
}

impl LoRAAdapter {
    /// Create a new adapter metadata entry.
    pub fn new(
        name: String,
        base_model: String,
        dataset_hash: String,
        training_job_id: String,
        size_bytes: u64,
        metrics: Option<AdapterMetrics>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            base_model,
            dataset_hash,
            training_job_id,
            created_at: Utc::now().timestamp(),
            size_bytes,
            metrics,
        }
    }
}

// ── Adapter store ──────────────────────────────────────────────────────────

/// Persistence interface for LoRA adapters.
///
/// Abstracts over `hkask-storage` SQLite (production) and in-memory storage
/// (tests). Each adapter consists of metadata (stored in a `lora_adapters`
/// table) and a blob payload (the actual adapter weights).
#[async_trait::async_trait]
pub trait AdapterStore: Send + Sync {
    /// Store adapter metadata.
    async fn store_metadata(&self, adapter: &LoRAAdapter) -> Result<(), AdapterStoreError>;

    /// Store adapter weight blob.
    async fn store_blob(&self, adapter_id: &str, blob: Vec<u8>) -> Result<(), AdapterStoreError>;

    /// Retrieve adapter metadata by ID.
    async fn get_metadata(
        &self,
        adapter_id: &str,
    ) -> Result<Option<LoRAAdapter>, AdapterStoreError>;

    /// Retrieve adapter weight blob by ID.
    async fn get_blob(&self, adapter_id: &str) -> Result<Option<Vec<u8>>, AdapterStoreError>;

    /// List all stored adapters.
    async fn list_all(&self) -> Result<Vec<LoRAAdapter>, AdapterStoreError>;

    /// Delete an adapter (both metadata and blob).
    async fn delete(&self, adapter_id: &str) -> Result<(), AdapterStoreError>;
}

// ── Store errors ───────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum AdapterStoreError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Adapter not found: {0}")]
    NotFound(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

// ── In-memory adapter store (for tests) ────────────────────────────────────

/// In-memory adapter store for testing.
///
/// Uses `HashMap` for fast lookups. Blobs and metadata are stored separately.
/// Not suitable for production — use `SqliteAdapterStore` backed by `hkask-storage`.
pub struct InMemoryAdapterStore {
    metadata: std::sync::RwLock<std::collections::HashMap<String, LoRAAdapter>>,
    blobs: std::sync::RwLock<std::collections::HashMap<String, Vec<u8>>>,
}

impl InMemoryAdapterStore {
    /// Create a new empty in-memory store.
    pub fn new() -> Self {
        Self {
            metadata: std::sync::RwLock::new(std::collections::HashMap::new()),
            blobs: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryAdapterStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AdapterStore for InMemoryAdapterStore {
    async fn store_metadata(&self, adapter: &LoRAAdapter) -> Result<(), AdapterStoreError> {
        self.metadata
            .write()
            .map_err(|e| AdapterStoreError::Storage(format!("Lock error: {}", e)))?
            .insert(adapter.id.clone(), adapter.clone());
        Ok(())
    }

    async fn store_blob(&self, adapter_id: &str, blob: Vec<u8>) -> Result<(), AdapterStoreError> {
        self.blobs
            .write()
            .map_err(|e| AdapterStoreError::Storage(format!("Lock error: {}", e)))?
            .insert(adapter_id.to_string(), blob);
        Ok(())
    }

    async fn get_metadata(
        &self,
        adapter_id: &str,
    ) -> Result<Option<LoRAAdapter>, AdapterStoreError> {
        Ok(self
            .metadata
            .read()
            .map_err(|e| AdapterStoreError::Storage(format!("Lock error: {}", e)))?
            .get(adapter_id)
            .cloned())
    }

    async fn get_blob(&self, adapter_id: &str) -> Result<Option<Vec<u8>>, AdapterStoreError> {
        Ok(self
            .blobs
            .read()
            .map_err(|e| AdapterStoreError::Storage(format!("Lock error: {}", e)))?
            .get(adapter_id)
            .cloned())
    }

    async fn list_all(&self) -> Result<Vec<LoRAAdapter>, AdapterStoreError> {
        Ok(self
            .metadata
            .read()
            .map_err(|e| AdapterStoreError::Storage(format!("Lock error: {}", e)))?
            .values()
            .cloned()
            .collect())
    }

    async fn delete(&self, adapter_id: &str) -> Result<(), AdapterStoreError> {
        self.metadata
            .write()
            .map_err(|e| AdapterStoreError::Storage(format!("Lock error: {}", e)))?
            .remove(adapter_id);
        self.blobs
            .write()
            .map_err(|e| AdapterStoreError::Storage(format!("Lock error: {}", e)))?
            .remove(adapter_id);
        Ok(())
    }
}

// ── SQLite adapter store ───────────────────────────────────────────────────

/// SQLite-backed adapter store using `hkask-storage`.
///
/// Stores metadata in a `lora_adapters` table and blobs in `hkask-storage`'s
/// blob table. Used in production deployments.
pub struct SqliteAdapterStore {
    conn: hkask_storage::DatabaseConnection,
}

impl SqliteAdapterStore {
    /// Create a new SQLite-backed store.
    ///
    /// `conn` should be an `hkask_storage::Database` connection.
    /// The caller is responsible for running migrations to create the
    /// `lora_adapters` table.
    pub fn new(conn: hkask_storage::DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Initialize the `lora_adapters` table schema.
    ///
    /// Call this once during server startup. Idempotent — uses `IF NOT EXISTS`.
    pub fn migrate(&self) -> Result<(), AdapterStoreError> {
        hkask_storage::migrate::run_migration(
            &self.conn,
            "hkask-mcp-training",
            "create_lora_adapters",
            "CREATE TABLE IF NOT EXISTS lora_adapters (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                base_model TEXT NOT NULL,
                dataset_hash TEXT NOT NULL,
                training_job_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                size_bytes INTEGER NOT NULL,
                metrics_json TEXT
            )",
        )
        .map_err(|e| AdapterStoreError::Storage(format!("Migration failed: {}", e)))
    }
}

#[async_trait::async_trait]
impl AdapterStore for SqliteAdapterStore {
    async fn store_metadata(&self, adapter: &LoRAAdapter) -> Result<(), AdapterStoreError> {
        let metrics_json = adapter
            .metrics
            .as_ref()
            .map(|m| {
                serde_json::to_string(m)
                    .map_err(|e| AdapterStoreError::Serialization(format!("Metrics: {}", e)))
            })
            .transpose()?;

        hkask_storage::migrate::execute_no_params(
            &self.conn,
            "INSERT OR REPLACE INTO lora_adapters
             (id, name, base_model, dataset_hash, training_job_id, created_at, size_bytes, metrics_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            &[
                &adapter.id,
                &adapter.name,
                &adapter.base_model,
                &adapter.dataset_hash,
                &adapter.training_job_id,
                &adapter.created_at.to_string(),
                &adapter.size_bytes.to_string(),
                &metrics_json.unwrap_or_default(),
            ],
        )
        .map_err(|e| AdapterStoreError::Storage(format!("Insert failed: {}", e)))?;

        tracing::info!(
            target: "cns.training.adapter.created",
            adapter_id = %adapter.id,
            base_model = %adapter.base_model,
            size_bytes = %adapter.size_bytes,
            "LoRA adapter stored"
        );
        Ok(())
    }

    async fn store_blob(&self, adapter_id: &str, blob: Vec<u8>) -> Result<(), AdapterStoreError> {
        hkask_storage::blob::store_blob_sqlite(&self.conn, adapter_id, &blob)
            .map_err(|e| AdapterStoreError::Storage(format!("Blob store failed: {}", e)))
    }

    async fn get_metadata(
        &self,
        adapter_id: &str,
    ) -> Result<Option<LoRAAdapter>, AdapterStoreError> {
        let rows = hkask_storage::migrate::query(
            &self.conn,
            "SELECT id, name, base_model, dataset_hash, training_job_id, created_at, size_bytes, metrics_json
             FROM lora_adapters WHERE id = ?1",
            &[adapter_id],
        )
        .map_err(|e| AdapterStoreError::Storage(format!("Query failed: {}", e)))?;

        if rows.is_empty() {
            return Ok(None);
        }
        let row = &rows[0];
        let created_at: i64 = row[5]
            .parse()
            .map_err(|e| AdapterStoreError::Serialization(format!("Parse created_at: {}", e)))?;
        let size_bytes: u64 = row[6]
            .parse()
            .map_err(|e| AdapterStoreError::Serialization(format!("Parse size_bytes: {}", e)))?;
        let metrics = if row[7] != "null" && !row[7].is_empty() {
            Some(
                serde_json::from_str(&row[7])
                    .map_err(|e| AdapterStoreError::Serialization(format!("Metrics: {}", e)))?,
            )
        } else {
            None
        };

        Ok(Some(LoRAAdapter {
            id: row[0].clone(),
            name: row[1].clone(),
            base_model: row[2].clone(),
            dataset_hash: row[3].clone(),
            training_job_id: row[4].clone(),
            created_at,
            size_bytes,
            metrics,
        }))
    }

    async fn get_blob(&self, adapter_id: &str) -> Result<Option<Vec<u8>>, AdapterStoreError> {
        hkask_storage::blob::read_blob_sqlite(&self.conn, adapter_id)
            .map_err(|e| AdapterStoreError::Storage(format!("Blob read failed: {}", e)))
    }

    async fn list_all(&self) -> Result<Vec<LoRAAdapter>, AdapterStoreError> {
        let rows = hkask_storage::migrate::query(
            &self.conn,
            "SELECT id, name, base_model, dataset_hash, training_job_id, created_at, size_bytes, metrics_json
             FROM lora_adapters ORDER BY created_at DESC",
            &[],
        )
        .map_err(|e| AdapterStoreError::Storage(format!("Query failed: {}", e)))?;

        let mut adapters = Vec::with_capacity(rows.len());
        for row in &rows {
            let created_at: i64 = row[5].parse().map_err(|e| {
                AdapterStoreError::Serialization(format!("Parse created_at: {}", e))
            })?;
            let size_bytes: u64 = row[6].parse().map_err(|e| {
                AdapterStoreError::Serialization(format!("Parse size_bytes: {}", e))
            })?;
            let metrics = if row[7] != "null" && !row[7].is_empty() {
                Some(
                    serde_json::from_str(&row[7])
                        .map_err(|e| AdapterStoreError::Serialization(format!("Metrics: {}", e)))?,
                )
            } else {
                None
            };
            adapters.push(LoRAAdapter {
                id: row[0].clone(),
                name: row[1].clone(),
                base_model: row[2].clone(),
                dataset_hash: row[3].clone(),
                training_job_id: row[4].clone(),
                created_at,
                size_bytes,
                metrics,
            });
        }
        Ok(adapters)
    }

    async fn delete(&self, adapter_id: &str) -> Result<(), AdapterStoreError> {
        hkask_storage::migrate::execute_no_params(
            &self.conn,
            "DELETE FROM lora_adapters WHERE id = ?1",
            &[adapter_id],
        )
        .map_err(|e| AdapterStoreError::Storage(format!("Delete metadata failed: {}", e)))?;

        hkask_storage::blob::delete_blob_sqlite(&self.conn, adapter_id)
            .map_err(|e| AdapterStoreError::Storage(format!("Delete blob failed: {}", e)))?;

        tracing::info!(
            target: "cns.training.adapter.deleted",
            adapter_id = %adapter_id,
            "LoRA adapter deleted from storage"
        );
        Ok(())
    }
}
