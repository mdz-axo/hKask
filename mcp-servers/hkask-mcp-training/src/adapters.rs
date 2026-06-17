//! LoRA adapter lifecycle management.
//!
//! Manages LoRA adapter artifacts — store, retrieve, list, delete — with metadata
//! linking back to originating `TrainingJob`, base model, and dataset provenance.
//!
//! Adapters are stored as blobs in `hkask-storage` with metadata in a dedicated
//! `lora_adapters` table. Referenced from `hkask-templates` registry for model
//! composition (base + adapter = effective model).

use chrono::Utc;
use hkask_adapter::adapter_store::Checksum;
use hkask_adapter::expertise::{Expertise, MdsDomain, TrainingProvenance};
use hkask_adapter::{AdapterSource, TrainedLoRAAdapter};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
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
    /// Version number for this adapter (incremented on retraining).
    /// Defaults to 1 for initial training, incremented by training_retrain.
    pub version: u32,
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        base_model: String,
        dataset_hash: String,
        training_job_id: String,
        size_bytes: u64,
        skill_name: String,
        version: u32,
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
            version,
            metrics,
        }
    }

    /// Convert to the canonical `TrainedLoRAAdapter` for deployment via `hkask-adapter`.
    pub fn to_canonical(&self) -> TrainedLoRAAdapter {
        let metrics_json = self
            .metrics
            .as_ref()
            .and_then(|m| serde_json::to_value(m).ok())
            .unwrap_or_default();
        let provenance = TrainingProvenance {
            training_run_id: self.training_job_id.clone(),
            training_source: String::new(),
            completed_at: chrono::DateTime::from_timestamp(self.created_at, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
            base_model_family: self.base_model.clone(),
            dataset_hash: if self.dataset_hash.is_empty() {
                None
            } else {
                Some(self.dataset_hash.clone())
            },
            training_metrics: metrics_json,
        };
        let expertise = Expertise::new(
            self.skill_name.clone(),
            MdsDomain::CodeGeneration,
            serde_json::Value::Null,
            provenance,
        )
        .unwrap_or_else(|_| Expertise {
            name: self.name.clone(),
            domain: MdsDomain::CodeGeneration,
            capability_manifest: serde_json::Value::Null,
            training_source: TrainingProvenance {
                training_run_id: String::new(),
                training_source: String::new(),
                completed_at: String::new(),
                base_model_family: String::new(),
                dataset_hash: None,
                training_metrics: serde_json::Value::Null,
            },
        });
        TrainedLoRAAdapter {
            id: self.id.parse().unwrap_or_else(|_| Uuid::new_v4()),
            expertise,
            checksum: Checksum::from_hex("0000000000000000"),
            storage_path: String::new(),
            base_model_family: self.base_model.clone(),
            version: Some(self.version.to_string()),
            source: AdapterSource::HuggingFace {
                repo: format!("hkask-training/{}", self.id),
            },
            size_bytes: if self.size_bytes > 0 {
                Some(self.size_bytes)
            } else {
                None
            },
            owner: hkask_types::id::WebID::from_persona(b"training-pipeline"),
            created_at: chrono::Utc::now().to_rfc3339(),
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

    /// Retrieve the latest adapter for a given skill name (highest version).
    /// Returns `None` if no adapter exists for this skill.
    async fn get_by_skill_name(
        &self,
        skill_name: &str,
    ) -> Result<Option<LoRAAdapter>, AdapterStoreError> {
        // Default: scan all adapters. SQLite overrides with an indexed query.
        let all = self.list_all().await?;
        Ok(all
            .into_iter()
            .filter(|a| a.skill_name == skill_name)
            .max_by_key(|a| a.version))
    }
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
                version INTEGER NOT NULL DEFAULT 1,
                metrics_json TEXT
            );
            CREATE TABLE IF NOT EXISTS lora_blobs (
                adapter_id TEXT PRIMARY KEY,
                data BLOB NOT NULL,
                FOREIGN KEY (adapter_id) REFERENCES lora_adapters(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS training_jobs (
                id TEXT PRIMARY KEY,
                base_model TEXT NOT NULL,
                dataset_path TEXT NOT NULL,
                params_json TEXT NOT NULL DEFAULT '{}',
                status TEXT NOT NULL DEFAULT 'queued',
                created_at INTEGER NOT NULL,
                host TEXT NOT NULL
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
             (id, name, base_model, dataset_hash, training_job_id, created_at, size_bytes, skill_name, version, metrics_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            &[&adapter.id, &adapter.name, &adapter.base_model, &adapter.dataset_hash, &adapter.training_job_id, &adapter.created_at as &dyn rusqlite::types::ToSql, &(adapter.size_bytes as i64) as &dyn rusqlite::types::ToSql, &adapter.skill_name as &dyn rusqlite::types::ToSql, &(adapter.version as i64) as &dyn rusqlite::types::ToSql, &metrics_json as &dyn rusqlite::types::ToSql],
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
                "SELECT id, name, base_model, dataset_hash, training_job_id, created_at, size_bytes, skill_name, version, metrics_json
                 FROM lora_adapters WHERE id = ?1",
            )
            .map_err(|e| AdapterStoreError::Storage(format!("Query failed: {}", e)))?;

        let result = stmt.query_row(rusqlite::params![adapter_id], |row| {
            let created_at: i64 = row.get(5)?;
            let size_bytes_i64: i64 = row.get(6)?;
            let skill_name: String = row.get(7)?;
            let version_i64: i64 = row.get(8)?;
            let metrics_json: Option<String> = row.get(9)?;
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
                version: version_i64 as u32,
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
                "SELECT id, name, base_model, dataset_hash, training_job_id, created_at, size_bytes, skill_name, version, metrics_json
                 FROM lora_adapters ORDER BY created_at DESC",
            )
            .map_err(|e| AdapterStoreError::Storage(format!("Query failed: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                let created_at: i64 = row.get(5)?;
                let size_bytes_i64: i64 = row.get(6)?;
                let skill_name: String = row.get(7)?;
                let version_i64: i64 = row.get(8)?;
                let metrics_json: Option<String> = row.get(9)?;
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
                    version: version_i64 as u32,
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

    async fn get_by_skill_name(
        &self,
        skill_name: &str,
    ) -> Result<Option<LoRAAdapter>, AdapterStoreError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, base_model, dataset_hash, training_job_id, created_at, size_bytes, skill_name, version, metrics_json
                 FROM lora_adapters WHERE skill_name = ?1 ORDER BY version DESC LIMIT 1",
            )
            .map_err(|e| AdapterStoreError::Storage(format!("Query failed: {}", e)))?;

        let result = stmt.query_row(rusqlite::params![skill_name], |row| {
            let created_at: i64 = row.get(5)?;
            let size_bytes_i64: i64 = row.get(6)?;
            let skill_name: String = row.get(7)?;
            let version_i64: i64 = row.get(8)?;
            let metrics_json: Option<String> = row.get(9)?;
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
                version: version_i64 as u32,
                metrics,
            })
        });

        match result {
            Ok(adapter) => Ok(Some(adapter)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AdapterStoreError::Storage(format!("Query failed: {}", e))),
        }
    }
}

// ── Job store ───────────────────────────────────────────────────────────

/// Persisted training job record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredJob {
    pub id: String,
    pub base_model: String,
    pub dataset_path: String,
    pub params_json: String,
    pub status: String,
    pub created_at: i64,
    pub host: String,
}

/// Persistent job registry backed by the same SQLite database.
/// Survives server restarts — enables `training_status` and `training_retrain`
/// to look up original job parameters.
pub struct JobStore {
    conn: Arc<Mutex<Connection>>,
}

impl JobStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AdapterStoreError> {
        self.conn
            .lock()
            .map_err(|e| AdapterStoreError::Storage(format!("Lock error: {}", e)))
    }

    /// Store a new training job.
    #[allow(clippy::too_many_arguments)]
    pub fn store(
        &self,
        id: &str,
        base_model: &str,
        dataset_path: &str,
        params_json: &str,
        status: &str,
        created_at: i64,
        host: &str,
    ) -> Result<(), AdapterStoreError> {
        let conn = self.lock()?;
        exec_discard(
            &conn,
            "INSERT OR REPLACE INTO training_jobs
             (id, base_model, dataset_path, params_json, status, created_at, host)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            &[
                &id as &dyn rusqlite::types::ToSql,
                &base_model as &dyn rusqlite::types::ToSql,
                &dataset_path as &dyn rusqlite::types::ToSql,
                &params_json as &dyn rusqlite::types::ToSql,
                &status as &dyn rusqlite::types::ToSql,
                &created_at as &dyn rusqlite::types::ToSql,
                &host as &dyn rusqlite::types::ToSql,
            ],
        )?;
        Ok(())
    }

    /// Update job status.
    pub fn update_status(&self, job_id: &str, status: &str) -> Result<(), AdapterStoreError> {
        let conn = self.lock()?;
        exec_discard(
            &conn,
            "UPDATE training_jobs SET status = ?1 WHERE id = ?2",
            &[
                &status as &dyn rusqlite::types::ToSql,
                &job_id as &dyn rusqlite::types::ToSql,
            ],
        )
    }

    /// Get a job by ID.
    pub fn get(&self, job_id: &str) -> Result<Option<StoredJob>, AdapterStoreError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, base_model, dataset_path, params_json, status, created_at, host
                     FROM training_jobs WHERE id = ?1",
            )
            .map_err(|e| AdapterStoreError::Storage(format!("Query failed: {}", e)))?;

        let result = stmt.query_row(rusqlite::params![job_id], |row| {
            Ok(StoredJob {
                id: row.get(0)?,
                base_model: row.get(1)?,
                dataset_path: row.get(2)?,
                params_json: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
                host: row.get(6)?,
            })
        });

        match result {
            Ok(job) => Ok(Some(job)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AdapterStoreError::Storage(format!("Query failed: {}", e))),
        }
    }

    /// List all jobs, most recent first.
    pub fn list_all(&self) -> Result<Vec<StoredJob>, AdapterStoreError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, base_model, dataset_path, params_json, status, created_at, host
                     FROM training_jobs ORDER BY created_at DESC",
            )
            .map_err(|e| AdapterStoreError::Storage(format!("Query failed: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(StoredJob {
                    id: row.get(0)?,
                    base_model: row.get(1)?,
                    dataset_path: row.get(2)?,
                    params_json: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                    host: row.get(6)?,
                })
            })
            .map_err(|e| AdapterStoreError::Storage(format!("Query failed: {}", e)))?;

        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(row.map_err(|e| AdapterStoreError::Storage(format!("Row error: {}", e)))?);
        }
        Ok(jobs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::Database;

    fn setup_db() -> Database {
        let db = Database::in_memory().expect("in-memory db");
        let conn = db.conn_arc();
        let conn_guard = conn.lock().unwrap();
        conn_guard
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS lora_adapters (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                base_model TEXT NOT NULL,
                dataset_hash TEXT NOT NULL,
                training_job_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                size_bytes INTEGER NOT NULL,
                skill_name TEXT NOT NULL DEFAULT '',
                version INTEGER NOT NULL DEFAULT 1,
                metrics_json TEXT
            );
            CREATE TABLE IF NOT EXISTS lora_blobs (
                adapter_id TEXT PRIMARY KEY,
                data BLOB NOT NULL,
                FOREIGN KEY (adapter_id) REFERENCES lora_adapters(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS training_jobs (
                id TEXT PRIMARY KEY,
                base_model TEXT NOT NULL,
                dataset_path TEXT NOT NULL,
                params_json TEXT NOT NULL DEFAULT '{}',
                status TEXT NOT NULL DEFAULT 'queued',
                created_at INTEGER NOT NULL,
                host TEXT NOT NULL
            );",
            )
            .expect("migration");
        drop(conn_guard);
        db
    }

    /// REQ: training-store-01 — SqliteAdapterStore stores and retrieves with version + skill_name
    #[tokio::test]
    async fn sqlite_store_version_and_skill_name() {
        let db = setup_db();
        let store = SqliteAdapterStore::new(db);

        let adapter = LoRAAdapter {
            id: "test-adapter-1".to_string(),
            name: "constraint-forces-v3".to_string(),
            base_model: "Qwen3.5-9B".to_string(),
            dataset_hash: "abc123".to_string(),
            training_job_id: "job-1".to_string(),
            created_at: 1000,
            size_bytes: 200_000_000,
            skill_name: "constraint-forces".to_string(),
            version: 3,
            metrics: Some(AdapterMetrics {
                loss: Some(0.12),
                perplexity: Some(1.5),
                training_duration_secs: Some(420),
                tokens_processed: Some(50_000),
            }),
        };

        store.store_metadata(&adapter).await.expect("store");

        let retrieved = store
            .get_metadata("test-adapter-1")
            .await
            .expect("get")
            .expect("found");

        assert_eq!(retrieved.skill_name, "constraint-forces");
        assert_eq!(retrieved.version, 3);
        assert_eq!(retrieved.name, "constraint-forces-v3");
        assert!(retrieved.metrics.is_some());
        assert_eq!(retrieved.metrics.unwrap().loss, Some(0.12));
    }

    /// REQ: training-store-02 — SqliteAdapterStore list_all returns multiple adapters
    #[tokio::test]
    async fn sqlite_store_list_all() {
        let db = setup_db();
        let store = SqliteAdapterStore::new(db);

        for i in 1..=3 {
            let adapter = LoRAAdapter {
                id: format!("adapter-{}", i),
                name: format!("skill-v{}", i),
                base_model: "Qwen3.5-9B".to_string(),
                dataset_hash: "hash".to_string(),
                training_job_id: format!("job-{}", i),
                created_at: 1000 + i,
                size_bytes: 100_000_000,
                skill_name: "test-skill".to_string(),
                version: i as u32,
                metrics: None,
            };
            store.store_metadata(&adapter).await.expect("store");
        }

        let all = store.list_all().await.expect("list");
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].version, 3);
        assert_eq!(all[2].version, 1);
    }

    /// REQ: training-store-03 — SqliteAdapterStore delete removes metadata and blob
    #[tokio::test]
    async fn sqlite_store_delete() {
        let db = setup_db();
        let store = SqliteAdapterStore::new(db);

        let adapter = LoRAAdapter {
            id: "to-delete".to_string(),
            name: "temp".to_string(),
            base_model: "test".to_string(),
            dataset_hash: "hash".to_string(),
            training_job_id: "job".to_string(),
            created_at: 1,
            size_bytes: 0,
            skill_name: "test".to_string(),
            version: 1,
            metrics: None,
        };
        store.store_metadata(&adapter).await.expect("store");
        store
            .store_blob("to-delete", vec![1, 2, 3])
            .await
            .expect("store blob");

        store.delete("to-delete").await.expect("delete");

        let meta = store.get_metadata("to-delete").await.expect("get");
        assert!(meta.is_none(), "metadata should be deleted");

        let blob = store.get_blob("to-delete").await.expect("get blob");
        assert!(blob.is_none(), "blob should be deleted");
    }

    /// REQ: training-job-01 — JobStore stores and retrieves jobs
    #[test]
    fn job_store_store_and_get() {
        let db = setup_db();
        let conn = db.conn_arc();
        let store = JobStore::new(conn);

        store
            .store(
                "job-1",
                "Qwen3.5-9B",
                "/data/train.jsonl",
                "{}",
                "queued",
                1000,
                "together",
            )
            .expect("store");

        let job = store.get("job-1").expect("get").expect("found");
        assert_eq!(job.base_model, "Qwen3.5-9B");
        assert_eq!(job.status, "queued");
        assert_eq!(job.host, "together");
    }

    /// REQ: training-job-02 — JobStore update_status changes job status
    #[test]
    fn job_store_update_status() {
        let db = setup_db();
        let conn = db.conn_arc();
        let store = JobStore::new(conn);

        store
            .store(
                "job-2",
                "model",
                "/data.jsonl",
                "{}",
                "queued",
                2000,
                "axolotl",
            )
            .expect("store");

        store.update_status("job-2", "running").expect("update");

        let job = store.get("job-2").expect("get").expect("found");
        assert_eq!(job.status, "running");
    }

    /// REQ: training-job-03 — JobStore list_all returns jobs in order
    #[test]
    fn job_store_list_all() {
        let db = setup_db();
        let conn = db.conn_arc();
        let store = JobStore::new(conn);

        store
            .store("j1", "m1", "/d1", "{}", "queued", 100, "t")
            .expect("store");
        store
            .store("j2", "m2", "/d2", "{}", "completed", 200, "t")
            .expect("store");

        let all = store.list_all().expect("list");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, "j2");
        assert_eq!(all[1].id, "j1");
    }

    /// REQ: training-job-04 — JobStore returns None for missing job
    #[test]
    fn job_store_missing_returns_none() {
        let db = setup_db();
        let conn = db.conn_arc();
        let store = JobStore::new(conn);

        let result = store.get("nonexistent").expect("get");
        assert!(result.is_none());
    }
}
