//! AdapterStore — SQLite-backed persistence for trained LoRA adapters.
//!
//! # Schema (1 table)
//! - `trained_adapters` — metadata for each trained LoRA adapter
//!
//! Follows the `hkask-storage` pattern: `Database` + migrations + CRUD.
//! Adapter weights live on disk; only metadata is stored in SQLite.

use crate::expertise::{AdapterLifecycle, Expertise, MdsDomain, TrainingProvenance};
use hkask_database::driver::{query_map, query_row};
use hkask_database::value::DbValue;
use hkask_storage_core::define_driver_store;
use hkask_types::InfrastructureError;
use hkask_types::NotFound;
use hkask_types::id::WebID;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Adapter distribution source ────────────────────────────────────────────

/// Where the adapter weights are hosted for distribution to inference providers.
///
/// Each variant represents a different model registry or storage backend.
/// Adding a new source is just adding an enum arm — no schema migration needed
/// (stored as JSON in SQLite). Provider backends translate the source into
/// their native upload/pull mechanism.
///
/// semantic-graph-audit (M5): variants must mean "where the canonical weights
/// live", not "what to do". `HuggingFace { repo }` = weights live at this remote
/// repo (pull model). A future `Local { tag }` variant for Ollama must mean
/// "canonical copy is local disk (storage_path), registered to Ollama as tag",
/// NOT "register-then-push" — keep the semantics consistent across arms.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AdapterSource {
    /// Adapter hosted on Hugging Face Hub (public, private, or gated).
    /// /// All three inference providers (Together, Runpod) can pull from HF Hub.
    HuggingFace {
        /// Repository path (e.g. "mdz-axo/solidity-audit-v3")
        repo: String,
    },
}

impl AdapterSource {
    /// The repository identifier, regardless of source type.
    pub fn repository_id(&self) -> &str {
        match self {
            AdapterSource::HuggingFace { repo } => repo,
        }
    }
}

// ── Store definition ─────────────────────────────────────────────────────────

define_driver_store!(AdapterStore);

// ── Row type for query mapping ──────────────────────────────────────────────

struct AdapterRow {
    adapter_id: String,
    expertise_name: String,
    expertise_domain: String,
    capability_manifest_json: String,
    checksum: String,
    storage_path: String,
    base_model_family: String,
    version: Option<String>,
    source_json: String,
    size_bytes: Option<i64>,
    owner_webid: String,
    training_run_id: String,
    training_source: String,
    completed_at: String,
    dataset_hash: Option<String>,
    training_metrics_json: Option<String>,
    lifecycle: String,
    expires_at: Option<i64>,
    created_at: String,
}

// ── Domain types ────────────────────────────────────────────────────────────

/// A content-addressed SHA-256 checksum.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Checksum(String);

impl Checksum {
    /// Create a Checksum from a hex string.
    pub fn from_hex(hex: &str) -> Self {
        Self(hex.to_lowercase())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Checksum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A trained LoRA adapter — content-addressed, owner-scoped artifact.
///
/// expect: "The adapter manages LoRA adapter lifecycle and inference composition"
/// \[P8\] Semantic Grounding — adapter is content-addressed and provenance-chained
/// pre:  adapter weights pass checksum validation
/// post: adapter is stored with owner WebID, expertise link, and base model family
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrainedLoRAAdapter {
    /// Unique identifier
    pub id: Uuid,
    /// The expertise this adapter implements
    pub expertise: Expertise,
    /// SHA-256 checksum of the adapter weights
    pub checksum: Checksum,
    /// Path to the adapter weights directory (contains adapter_config.json + adapter_model.safetensors)
    pub storage_path: String,
    /// Base model family (derived from expertise.training_source — kept for fast DB queries)
    pub base_model_family: String,
    /// Optional version identifier (e.g. "v3", "latest"). Caller-managed; never implicitly superseded (P2).
    #[serde(default)]
    pub version: Option<String>,
    /// Distribution source — where the adapter weights are hosted for provider pull.
    /// Right now only HuggingFace, but the enum is designed for extension.
    pub source: AdapterSource,
    /// Size of adapter weights in bytes (populated after training completes).
    #[serde(default)]
    pub size_bytes: Option<u64>,
    /// Owner (sovereign-scoped — no anonymous artifacts, P12)
    pub owner: WebID,
    /// Lifecycle class — durable expertise vs ephemeral context internalization.
    /// Operator-chosen at training time. Defaults to `Durable` for backfilled rows.
    #[serde(default)]
    pub lifecycle: AdapterLifecycle,
    /// Optional TTL for ephemeral adapters (Unix epoch seconds).
    /// `None` for `Durable` adapters. `Some(epoch)` for `Ephemeral` adapters.
    #[serde(default)]
    pub expires_at: Option<u64>,
    /// When the adapter was stored
    pub created_at: String,
}

/// Errors for adapter store operations.
#[derive(Debug, thiserror::Error)]
pub enum AdapterStoreError {
    #[error("Adapter with id {0} not found")]
    NotFound(NotFound),

    #[error("Adapter with expertise '{0}' not found")]
    ExpertiseNotFound(String),

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch {
        expected: Checksum,
        actual: Checksum,
    },

    #[error("Invalid adapter state: {0}")]
    InvalidState(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Infrastructure error: {0}")]
    Infra(#[from] InfrastructureError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl From<NotFound> for AdapterStoreError {
    fn from(nf: NotFound) -> Self {
        AdapterStoreError::NotFound(nf)
    }
}

impl From<hkask_database::types::DbError> for AdapterStoreError {
    fn from(e: hkask_database::types::DbError) -> Self {
        AdapterStoreError::Database(e.to_string())
    }
}

// ── Shared SELECT columns ─────────────────────────────────────────────────

const ADAPTER_SELECT: &str = "SELECT adapter_id, expertise_name, expertise_domain, \
    capability_manifest_json, checksum, storage_path, base_model_family, \
    version, source_json, size_bytes, owner_webid, training_run_id, training_source, \
    completed_at, dataset_hash, training_metrics_json, lifecycle, expires_at, \
    created_at FROM trained_adapters";

// ── AdapterStore implementation ──────────────────────────────────────────────

impl AdapterStore {
    /// Initialize schema — called automatically by `from_driver()`.
    fn init_schema(driver: &std::sync::Arc<dyn hkask_database::driver::DatabaseDriver>) {
        let _ = driver
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS trained_adapters (
                        adapter_id          TEXT PRIMARY KEY NOT NULL,
                        expertise_name      TEXT NOT NULL,
                        expertise_domain    TEXT NOT NULL,
                        capability_manifest_json TEXT NOT NULL DEFAULT '{}',
                        checksum            TEXT NOT NULL,
                        storage_path        TEXT NOT NULL,
                        base_model_family   TEXT NOT NULL,
                        version             TEXT,
                        source_json         TEXT NOT NULL DEFAULT '{}',
                        size_bytes          INTEGER,
                        owner_webid         TEXT NOT NULL,
                        training_run_id     TEXT NOT NULL,
                        training_source     TEXT NOT NULL,
                        completed_at        TEXT NOT NULL,
                        dataset_hash        TEXT,
                        training_metrics_json TEXT,
                        lifecycle           TEXT NOT NULL DEFAULT 'durable',
                        expires_at          INTEGER,
                        created_at          TEXT NOT NULL DEFAULT (datetime('now'))
                    );
                    CREATE INDEX IF NOT EXISTS idx_adapter_expertise
                        ON trained_adapters(expertise_name);
                    CREATE INDEX IF NOT EXISTS idx_adapter_owner
                        ON trained_adapters(owner_webid);
                    CREATE TABLE IF NOT EXISTS active_endpoints (
                        endpoint_id     TEXT PRIMARY KEY NOT NULL,
                        adapter_id      TEXT NOT NULL,
                        provider        TEXT NOT NULL,
                        endpoint_url    TEXT NOT NULL,
                        model_name      TEXT NOT NULL,
                        expertise_name  TEXT NOT NULL,
                        phase           TEXT NOT NULL DEFAULT 'provisioning',
                        cost_accrued    REAL NOT NULL DEFAULT 0.0,
                        hourly_rate     REAL NOT NULL DEFAULT 0.0,
                        created_at      TEXT NOT NULL DEFAULT (datetime('now')),
                        FOREIGN KEY (adapter_id) REFERENCES trained_adapters(adapter_id)
                    );",
            )
            .ok();
    }

    /// Store a trained adapter.
    ///
    /// expect: "The adapter manages LoRA adapter lifecycle and inference composition"
    /// pre:  adapter has a valid expertise, checksum, owner, and storage_path
    /// post: adapter is persisted to SQLite
    pub fn store(&self, adapter: &TrainedLoRAAdapter) -> Result<(), AdapterStoreError> {
        let metrics_json =
            serde_json::to_string(&adapter.expertise.training_source.training_metrics)?;
        let manifest_json = serde_json::to_string(&adapter.expertise.capability_manifest)?;
        let source_json = serde_json::to_string(&adapter.source)?;

        self.driver.execute(
            "INSERT INTO trained_adapters
                (adapter_id, expertise_name, expertise_domain, capability_manifest_json,
                 checksum, storage_path, base_model_family, version, source_json, size_bytes,
                 owner_webid, training_run_id, training_source, completed_at, dataset_hash,
                 training_metrics_json, lifecycle, expires_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            &[
                DbValue::Text(adapter.id.to_string()),
                DbValue::Text(adapter.expertise.name.clone()),
                DbValue::Text(adapter.expertise.domain.as_str().to_string()),
                DbValue::Text(manifest_json),
                DbValue::Text(adapter.checksum.as_str().to_string()),
                DbValue::Text(adapter.storage_path.clone()),
                DbValue::Text(adapter.base_model_family.clone()),
                adapter
                    .version
                    .as_ref()
                    .map_or(DbValue::Null, |v| DbValue::Text(v.clone())),
                DbValue::Text(source_json),
                adapter
                    .size_bytes
                    .map_or(DbValue::Null, |b| DbValue::Integer(b as i64)),
                DbValue::Text(adapter.owner.as_uuid().to_string()),
                DbValue::Text(adapter.expertise.training_source.training_run_id.clone()),
                DbValue::Text(adapter.expertise.training_source.training_source.clone()),
                DbValue::Text(adapter.expertise.training_source.completed_at.clone()),
                adapter
                    .expertise
                    .training_source
                    .dataset_hash
                    .as_ref()
                    .map_or(DbValue::Null, |h| DbValue::Text(h.clone())),
                DbValue::Text(metrics_json),
                DbValue::Text(adapter.lifecycle.as_str().to_string()),
                adapter
                    .expires_at
                    .map_or(DbValue::Null, |e| DbValue::Integer(e as i64)),
                DbValue::Text(adapter.created_at.clone()),
            ],
        )?;
        // P9: CNS span
        tracing::info!(target: "cns.adapter", operation = "store", adapter_id = %adapter.id, expertise = %adapter.expertise.name, "CNS");
        Ok(())
    }

    /// Helper: build an AdapterRow from a DbRow.
    fn row_to_adapter_row(
        row: &hkask_database::value::DbRow,
    ) -> Result<AdapterRow, hkask_database::types::DbError> {
        Ok(AdapterRow {
            adapter_id: row.get_str(0)?.to_string(),
            expertise_name: row.get_str(1)?.to_string(),
            expertise_domain: row.get_str(2)?.to_string(),
            capability_manifest_json: row.get_str(3)?.to_string(),
            checksum: row.get_str(4)?.to_string(),
            storage_path: row.get_str(5)?.to_string(),
            base_model_family: row.get_str(6)?.to_string(),
            version: match row.get(7)? {
                DbValue::Null => None,
                v => Some(v.as_text()?.to_string()),
            },
            source_json: row.get_str(8)?.to_string(),
            size_bytes: match row.get(9)? {
                DbValue::Null => None,
                v => Some(v.as_int()?),
            },
            owner_webid: row.get_str(10)?.to_string(),
            training_run_id: row.get_str(11)?.to_string(),
            training_source: row.get_str(12)?.to_string(),
            completed_at: row.get_str(13)?.to_string(),
            dataset_hash: match row.get(14)? {
                DbValue::Null => None,
                v => Some(v.as_text()?.to_string()),
            },
            training_metrics_json: match row.get(15)? {
                DbValue::Null => None,
                v => Some(v.as_text()?.to_string()),
            },
            lifecycle: row.get_str(16)?.to_string(),
            expires_at: match row.get(17)? {
                DbValue::Null => None,
                v => Some(v.as_int()?),
            },
            created_at: row.get_str(18)?.to_string(),
        })
    }

    /// Retrieve an adapter by its UUID.
    ///
    /// expect: "The adapter manages LoRA adapter lifecycle and inference composition"
    /// pre:  id is a valid Uuid
    /// post: returns Some(TrainedLoRAAdapter) if found, None otherwise
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<TrainedLoRAAdapter>, AdapterStoreError> {
        let sql = format!("{} WHERE adapter_id = ?1", ADAPTER_SELECT);
        let rows: Vec<TrainedLoRAAdapter> = query_map(
            &*self.driver,
            &sql,
            &[DbValue::Text(id.to_string())],
            |row| {
                let r = Self::row_to_adapter_row(row)?;
                Self::row_to_adapter(r)
                    .map_err(|e| hkask_database::types::DbError::Database(e.to_string()))
            },
        )?;
        let result = rows.into_iter().next();
        // P9: CNS span
        tracing::info!(target: "cns.adapter", operation = "get_by_id", adapter_id = %id, found = result.is_some(), "CNS");
        Ok(result)
    }

    /// List adapters by expertise name.
    ///
    /// expect: "The adapter manages LoRA adapter lifecycle and inference composition"
    /// pre:  expertise_name is non-empty
    /// post: returns Vec of adapters matching the expertise name
    pub fn get_by_expertise(
        &self,
        expertise_name: &str,
    ) -> Result<Vec<TrainedLoRAAdapter>, AdapterStoreError> {
        let sql = format!("{} WHERE expertise_name = ?1", ADAPTER_SELECT);
        let rows: Vec<TrainedLoRAAdapter> = query_map(
            &*self.driver,
            &sql,
            &[DbValue::Text(expertise_name.to_string())],
            |row| {
                let r = Self::row_to_adapter_row(row)?;
                Self::row_to_adapter(r)
                    .map_err(|e| hkask_database::types::DbError::Database(e.to_string()))
            },
        )?;

        // P9: CNS span
        tracing::info!(target: "cns.adapter", operation = "get_by_expertise", expertise_name = %expertise_name, count = rows.len(), "CNS");
        Ok(rows)
    }

    /// List adapters owned by a specific WebID.
    ///
    /// expect: "The adapter manages LoRA adapter lifecycle and inference composition"
    /// pre:  owner is a valid WebID
    /// post: returns Vec of adapters owned by the given WebID
    pub fn list_owner(&self, owner: WebID) -> Result<Vec<TrainedLoRAAdapter>, AdapterStoreError> {
        let sql = format!("{} WHERE owner_webid = ?1", ADAPTER_SELECT);
        let rows: Vec<TrainedLoRAAdapter> = query_map(
            &*self.driver,
            &sql,
            &[DbValue::Text(owner.as_uuid().to_string())],
            |row| {
                let r = Self::row_to_adapter_row(row)?;
                Self::row_to_adapter(r)
                    .map_err(|e| hkask_database::types::DbError::Database(e.to_string()))
            },
        )?;

        Ok(rows)
    }

    /// Delete an adapter by ID.
    ///
    /// OCAP-gated: callers must present a valid DelegationToken with `adapter:delete` capability.
    /// The token is accepted here as documentation of the gate requirement, though actual
    /// token verification happens at the `AdapterPort` boundary (Task 5).
    ///
    /// expect: "The adapter manages LoRA adapter lifecycle and inference composition"
    /// pre:  adapter exists
    /// post: adapter row is removed
    pub fn delete(&self, id: Uuid) -> Result<(), AdapterStoreError> {
        let affected = self.driver.execute(
            "DELETE FROM trained_adapters WHERE adapter_id = ?1",
            &[DbValue::Text(id.to_string())],
        )?;
        if affected == 0 {
            return Err(AdapterStoreError::NotFound(NotFound {
                entity_type: "adapter".to_string(),
                id: id.to_string(),
            }));
        }
        // P9: CNS span
        tracing::info!(target: "cns.adapter", operation = "delete", adapter_id = %id, "CNS");
        Ok(())
    }

    /// Return the total count of stored adapters.
    pub fn count(&self) -> Result<usize, AdapterStoreError> {
        let count: i64 = query_row(
            &*self.driver,
            "SELECT COUNT(*) FROM trained_adapters",
            &[],
            |row| row.get_int(0),
        )?
        .unwrap_or(0);
        Ok(count as usize)
    }

    // ── Row mapping helpers ────────────────────────────────────────────────

    fn row_to_adapter(r: AdapterRow) -> Result<TrainedLoRAAdapter, AdapterStoreError> {
        let domain = MdsDomain::parse(&r.expertise_domain).unwrap_or(MdsDomain::CodeGeneration); // fallback for unknown domains
        let capability_manifest: serde_json::Value =
            serde_json::from_str(&r.capability_manifest_json).unwrap_or_default();
        let training_metrics: serde_json::Value = r
            .training_metrics_json
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let base_model = r.base_model_family.clone();
        let source: AdapterSource = serde_json::from_str(&r.source_json).map_err(|e| {
            AdapterStoreError::InvalidState(format!(
                "Corrupt adapter source_json for {}: {e}",
                r.adapter_id
            ))
        })?;
        let provenance = TrainingProvenance {
            training_run_id: r.training_run_id,
            training_source: r.training_source,
            completed_at: r.completed_at,
            base_model_family: r.base_model_family,
            dataset_hash: r.dataset_hash,
            training_metrics,
        };

        let expertise = Expertise {
            name: r.expertise_name,
            domain,
            capability_manifest,
            training_source: provenance,
        };

        let owner_uuid = Uuid::parse_str(&r.owner_webid)
            .map_err(|e| AdapterStoreError::Infra(InfrastructureError::database(e.to_string())))?;

        let id = Uuid::parse_str(&r.adapter_id)
            .map_err(|e| AdapterStoreError::Infra(InfrastructureError::database(e.to_string())))?;

        Ok(TrainedLoRAAdapter {
            id,
            expertise,
            checksum: Checksum::from_hex(&r.checksum),
            storage_path: r.storage_path,
            base_model_family: base_model,
            version: r.version,
            source,
            size_bytes: r.size_bytes.map(|b| b as u64),
            owner: WebID::from_uuid(owner_uuid),
            lifecycle: AdapterLifecycle::parse(&r.lifecycle).unwrap_or(AdapterLifecycle::Durable),
            expires_at: r.expires_at.map(|e| e as u64),
            created_at: r.created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_database::sqlite::SqliteDriver;
    use std::sync::Arc;

    fn make_test_adapter() -> TrainedLoRAAdapter {
        let id = Uuid::new_v4();
        let owner = WebID::from_uuid(Uuid::new_v4());
        let expertise = Expertise {
            name: "test-expertise".into(),
            domain: MdsDomain::CodeGeneration,
            capability_manifest: serde_json::json!({}),
            training_source: TrainingProvenance {
                training_run_id: "run-1".into(),
                training_source: "local".into(),
                completed_at: "2026-01-01".into(),
                base_model_family: "granite-8b".into(),
                dataset_hash: Some("abc123".into()),
                training_metrics: serde_json::json!({}),
            },
        };

        TrainedLoRAAdapter {
            id,
            expertise,
            checksum: Checksum::from_hex("deadbeef"),
            storage_path: "/tmp/adapter".into(),
            base_model_family: "granite-8b".into(),
            version: Some("v1".into()),
            source: AdapterSource::HuggingFace {
                repo: "test/model".into(),
            },
            size_bytes: Some(1024),
            owner,
            lifecycle: AdapterLifecycle::Durable,
            expires_at: None,
            created_at: "2026-07-01".into(),
        }
    }

    fn make_store() -> AdapterStore {
        let driver = SqliteDriver::in_memory_pool().expect("in-memory pool");
        let sqlite = SqliteDriver::new(driver);
        AdapterStore::from_driver(Arc::new(sqlite))
    }

    #[test]
    fn store_and_retrieve_by_id() {
        let store = make_store();
        let adapter = make_test_adapter();
        let id = adapter.id;

        store.store(&adapter).unwrap();
        let retrieved = store.get_by_id(id).unwrap().expect("adapter should exist");
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.expertise.name, "test-expertise");
    }

    #[test]
    fn retrieve_by_expertise() {
        let store = make_store();
        let a1 = make_test_adapter();
        let mut a2 = make_test_adapter();
        a2.expertise.name = "other-expertise".into();
        store.store(&a1).unwrap();
        store.store(&a2).unwrap();

        let results = store.get_by_expertise("test-expertise").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn list_by_owner() {
        let store = make_store();
        let owner = WebID::from_uuid(Uuid::new_v4());
        let mut a1 = make_test_adapter();
        a1.owner = owner;
        store.store(&a1).unwrap();

        let results = store.list_owner(owner).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn delete_adapter() {
        let store = make_store();
        let adapter = make_test_adapter();
        let id = adapter.id;
        store.store(&adapter).unwrap();
        store.delete(id).unwrap();
        assert!(store.get_by_id(id).unwrap().is_none());
    }

    #[test]
    fn delete_non_existent_returns_error() {
        let store = make_store();
        let result = store.delete(Uuid::new_v4());
        assert!(result.is_err());
    }
}
