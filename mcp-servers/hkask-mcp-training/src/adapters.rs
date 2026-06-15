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
    /// Skill name this adapter was trained for (e.g., "constraint-forces").
    /// Enables adapter-to-skill mapping for the registry and auto-selection router.
    pub skill_name: String,
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
        skill_name: String,
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
            skill_name,
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

/// SQLite-backed adapter store using `hkask-storage::Database`.
///
/// Stores metadata in a `lora_adapters` table and blobs in a `lora_blobs` table.
/// Used in production deployments. The caller must run `migrate()` once during
/// server startup to create the tables (idempotent via `IF NOT EXISTS`).
pub struct SqliteAdapterStore {
    conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
}

impl SqliteAdapterStore {
    /// Create a new SQLite-backed store using an existing `hkask_storage::Database`.
    pub fn new(db: hkask_storage::Database) -> Self {
        Self {
            conn: db.conn_arc(),
        }
    }

    /// Initialize the `lora_adapters` and `lora_blobs` tables.
    ///
    /// Call once during server startup. Idempotent — uses `IF NOT EXISTS`.
    pub fn migrate(&self) -> Result<(), AdapterStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AdapterStoreError::Storage(format!("Lock error: {}", e)))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS lora_adapters (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                base_model TEXT NOT NULL,
                dataset_hash TEXT NOT NULL,
                training_job_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                size_bytes INTEGER NOT NULL,
                skill_name TEXT NOT NULL DEFAULT '',
                metrics_json TEXT
            );
            CREATE TABLE IF NOT EXISTS lora_blobs (
                adapter_id TEXT PRIMARY KEY,
                data BLOB NOT NULL,
                FOREIGN KEY (adapter_id) REFERENCES lora_adapters(id) ON DELETE CASCADE
            );",
        )
        .map_err(|e| AdapterStoreError::Storage(format!("Migration failed: {}", e)))
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, AdapterStoreError> {
        self.conn
            .lock()
            .map_err(|e| AdapterStoreError::Storage(format!("Lock error: {}", e)))
    }
}

// Helper to execute and discard row count
fn exec_discard(
    conn: &rusqlite::Connection,
    sql: &str,
    params: &[&dyn rusqlite::types::ToSql],
) -> Result<(), AdapterStoreError> {
    conn.execute(sql, params)
        .map(|_| ())
        .map_err(|e| AdapterStoreError::Storage(format!("Execute failed: {}", e)))
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

        let conn = self.lock()?;
        exec_discard(
            &conn,
            "INSERT OR REPLACE INTO lora_adapters
             (id, name, base_model, dataset_hash, training_job_id, created_at, size_bytes, skill_name, metrics_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            &[&adapter.id, &adapter.name, &adapter.base_model, &adapter.dataset_hash, &adapter.training_job_id, &adapter.created_at as &dyn rusqlite::types::ToSql, &(adapter.size_bytes as i64) as &dyn rusqlite::types::ToSql, &adapter.skill_name as &dyn rusqlite::types::ToSql, &metrics_json as &dyn rusqlite::types::ToSql],
        )?;

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
        let conn = self.lock()?;
        exec_discard(
            &conn,
            "INSERT OR REPLACE INTO lora_blobs (adapter_id, data) VALUES (?1, ?2)",
            &[
                &adapter_id as &dyn rusqlite::types::ToSql,
                &blob as &dyn rusqlite::types::ToSql,
            ],
        )
    }

    async fn get_metadata(
        &self,
        adapter_id: &str,
    ) -> Result<Option<LoRAAdapter>, AdapterStoreError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, base_model, dataset_hash, training_job_id, created_at, size_bytes, skill_name, metrics_json
                 FROM lora_adapters WHERE id = ?1",
            )
            .map_err(|e| AdapterStoreError::Storage(format!("Query failed: {}", e)))?;

        let result = stmt.query_row(rusqlite::params![adapter_id], |row| {
            let created_at: i64 = row.get(5)?;
            let size_bytes_i64: i64 = row.get(6)?;
            let skill_name: String = row.get(7)?;
            let metrics_json: Option<String> = row.get(8)?;
            let metrics = match metrics_json {
                Some(ref json) if !json.is_empty() && json != "null" => {
                    Some(serde_json::from_str(json).map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            7,
                            "Invalid metrics JSON".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?)
                }
                _ => None,
            };
            Ok(LoRAAdapter {
                id: row.get(0)?,
                name: row.get(1)?,
                base_model: row.get(2)?,
                dataset_hash: row.get(3)?,
                training_job_id: row.get(4)?,
                created_at,
                size_bytes: size_bytes_i64 as u64,
                skill_name,
                metrics,
            })
        });

        match result {
            Ok(adapter) => Ok(Some(adapter)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AdapterStoreError::Storage(format!("Query failed: {}", e))),
        }
    }

    async fn get_blob(&self, adapter_id: &str) -> Result<Option<Vec<u8>>, AdapterStoreError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare("SELECT data FROM lora_blobs WHERE adapter_id = ?1")
            .map_err(|e| AdapterStoreError::Storage(format!("Query failed: {}", e)))?;

        match stmt.query_row(rusqlite::params![adapter_id], |row| row.get(0)) {
            Ok(blob) => Ok(Some(blob)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AdapterStoreError::Storage(format!("Query failed: {}", e))),
        }
    }

    async fn list_all(&self) -> Result<Vec<LoRAAdapter>, AdapterStoreError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, base_model, dataset_hash, training_job_id, created_at, size_bytes, skill_name, metrics_json
                 FROM lora_adapters ORDER BY created_at DESC",
            )
            .map_err(|e| AdapterStoreError::Storage(format!("Query failed: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                let created_at: i64 = row.get(5)?;
                let size_bytes_i64: i64 = row.get(6)?;
                let skill_name: String = row.get(7)?;
                let metrics_json: Option<String> = row.get(8)?;
                let metrics = match metrics_json {
                    Some(ref json) if !json.is_empty() && json != "null" => {
                        Some(serde_json::from_str(json).map_err(|_| {
                            rusqlite::Error::InvalidColumnType(
                                7,
                                "Invalid metrics JSON".to_string(),
                                rusqlite::types::Type::Text,
                            )
                        })?)
                    }
                    _ => None,
                };
                Ok(LoRAAdapter {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    base_model: row.get(2)?,
                    dataset_hash: row.get(3)?,
                    training_job_id: row.get(4)?,
                    created_at,
                    size_bytes: size_bytes_i64 as u64,
                    skill_name,
                    metrics,
                })
            })
            .map_err(|e| AdapterStoreError::Storage(format!("Query failed: {}", e)))?;

        let mut adapters = Vec::new();
        for row in rows {
            adapters
                .push(row.map_err(|e| AdapterStoreError::Storage(format!("Row error: {}", e)))?);
        }
        Ok(adapters)
    }

    async fn delete(&self, adapter_id: &str) -> Result<(), AdapterStoreError> {
        let conn = self.lock()?;
        exec_discard(
            &conn,
            "DELETE FROM lora_blobs WHERE adapter_id = ?1",
            &[&adapter_id as &dyn rusqlite::types::ToSql],
        )?;
        exec_discard(
            &conn,
            "DELETE FROM lora_adapters WHERE id = ?1",
            &[&adapter_id as &dyn rusqlite::types::ToSql],
        )?;

        tracing::info!(
            target: "cns.training.adapter.deleted",
            adapter_id = %adapter_id,
            "LoRA adapter deleted from storage"
        );
        Ok(())
    }
}
