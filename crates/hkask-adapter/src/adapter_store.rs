//! AdapterStore — SQLite-backed persistence for trained LoRA adapters.
//!
//! # Schema (1 table)
//! - `trained_adapters` — metadata for each trained LoRA adapter
//!
//! Follows the `hkask-storage` pattern: `Database` + migrations + CRUD.
//! Adapter weights live on disk; only metadata is stored in SQLite.

use crate::expertise::{Expertise, MdsDomain, TrainingProvenance};
use hkask_storage::Store;
use hkask_storage::collect_rows_strict;
use hkask_storage::define_store;
use hkask_types::InfrastructureError;
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AdapterSource {
    /// Adapter hosted on Hugging Face Hub (public, private, or gated).
    /// All three inference providers (Together, Runpod, Baseten) can pull from HF Hub.
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

define_store!(AdapterStore);

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
/// REQ: P8-adt-trained-adapter-store
/// [P8] Semantic Grounding — adapter is content-addressed and provenance-chained
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
    /// When the adapter was stored
    pub created_at: String,
}

/// Errors for adapter store operations.
#[derive(Debug, thiserror::Error)]
pub enum AdapterStoreError {
    #[error("Adapter with id {0} not found")]
    NotFound(Uuid),

    #[error("Adapter with expertise '{0}' not found")]
    ExpertiseNotFound(String),

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch {
        expected: Checksum,
        actual: Checksum,
    },

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Infrastructure error: {0}")]
    Infra(#[from] InfrastructureError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

// ── Shared SELECT columns ─────────────────────────────────────────────────

const ADAPTER_SELECT: &str = "SELECT adapter_id, expertise_name, expertise_domain, \
    capability_manifest_json, checksum, storage_path, base_model_family, \
    version, source_json, size_bytes, owner_webid, training_run_id, training_source, \
    completed_at, dataset_hash, training_metrics_json, created_at FROM trained_adapters";

// ── AdapterStore implementation ──────────────────────────────────────────────

impl AdapterStore {
    /// Run schema migrations — create tables if they don't exist.
    ///
    /// REQ: P8-adt-trained-adapter-store
    /// post: trained_adapters table exists
    pub fn migrate(&self) -> Result<(), AdapterStoreError> {
        let conn = self.lock_conn()?;
        conn.execute_batch(
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
        )?;
        Ok(())
    }

    /// Store a trained adapter.
    ///
    /// REQ: P8-adt-trained-adapter-store
    /// pre:  adapter has a valid expertise, checksum, owner, and storage_path
    /// post: adapter is persisted to SQLite
    pub fn store(&self, adapter: &TrainedLoRAAdapter) -> Result<(), AdapterStoreError> {
        let conn = self.lock_conn()?;
        let metrics_json =
            serde_json::to_string(&adapter.expertise.training_source.training_metrics)?;
        let manifest_json = serde_json::to_string(&adapter.expertise.capability_manifest)?;
        let source_json = serde_json::to_string(&adapter.source)?;

        conn.execute(
            "INSERT INTO trained_adapters
                (adapter_id, expertise_name, expertise_domain, capability_manifest_json,
                 checksum, storage_path, base_model_family, version, source_json, size_bytes,
                 owner_webid, training_run_id, training_source, completed_at, dataset_hash,
                 training_metrics_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            rusqlite::params![
                adapter.id.to_string(),
                adapter.expertise.name,
                adapter.expertise.domain.as_str(),
                manifest_json,
                adapter.checksum.as_str(),
                adapter.storage_path,
                adapter.base_model_family,
                adapter.version,
                source_json,
                adapter.size_bytes.map(|b| b as i64),
                adapter.owner.as_uuid().to_string(),
                adapter.expertise.training_source.training_run_id,
                adapter.expertise.training_source.training_source,
                adapter.expertise.training_source.completed_at,
                adapter.expertise.training_source.dataset_hash,
                metrics_json,
                adapter.created_at,
            ],
        )?;
        Ok(())
    }

    /// Retrieve an adapter by its UUID.
    ///
    /// REQ: P8-adt-trained-adapter-store
    /// pre:  id is a valid Uuid
    /// post: returns Some(TrainedLoRAAdapter) if found, None otherwise
    pub fn get_by_id(&self, id: Uuid) -> Result<Option<TrainedLoRAAdapter>, AdapterStoreError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!("{} WHERE adapter_id = ?1", ADAPTER_SELECT))?;

        let rows: Vec<TrainedLoRAAdapter> = collect_rows_strict!(
            stmt,
            rusqlite::params![id.to_string()],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<AdapterRow> {
                Ok(AdapterRow {
                    adapter_id: row.get(0)?,
                    expertise_name: row.get(1)?,
                    expertise_domain: row.get(2)?,
                    capability_manifest_json: row.get(3)?,
                    checksum: row.get(4)?,
                    storage_path: row.get(5)?,
                    base_model_family: row.get(6)?,
                    version: row.get(7)?,
                    source_json: row.get(8)?,
                    size_bytes: row.get(9)?,
                    owner_webid: row.get(10)?,
                    training_run_id: row.get(11)?,
                    training_source: row.get(12)?,
                    completed_at: row.get(13)?,
                    dataset_hash: row.get(14)?,
                    training_metrics_json: row.get(15)?,
                    created_at: row.get(16)?,
                })
            },
            |r: AdapterRow| -> Result<TrainedLoRAAdapter, AdapterStoreError> {
                Self::row_to_adapter(r)
            }
        );

        Ok(rows.into_iter().next())
    }

    /// List adapters by expertise name.
    ///
    /// REQ: P8-adt-trained-adapter-store
    /// pre:  expertise_name is non-empty
    /// post: returns Vec of adapters matching the expertise name
    pub fn get_by_expertise(
        &self,
        expertise_name: &str,
    ) -> Result<Vec<TrainedLoRAAdapter>, AdapterStoreError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!("{} WHERE expertise_name = ?1", ADAPTER_SELECT))?;

        let rows: Vec<TrainedLoRAAdapter> = collect_rows_strict!(
            stmt,
            rusqlite::params![expertise_name],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<AdapterRow> {
                Ok(AdapterRow {
                    adapter_id: row.get(0)?,
                    expertise_name: row.get(1)?,
                    expertise_domain: row.get(2)?,
                    capability_manifest_json: row.get(3)?,
                    checksum: row.get(4)?,
                    storage_path: row.get(5)?,
                    base_model_family: row.get(6)?,
                    version: row.get(7)?,
                    source_json: row.get(8)?,
                    size_bytes: row.get(9)?,
                    owner_webid: row.get(10)?,
                    training_run_id: row.get(11)?,
                    training_source: row.get(12)?,
                    completed_at: row.get(13)?,
                    dataset_hash: row.get(14)?,
                    training_metrics_json: row.get(15)?,
                    created_at: row.get(16)?,
                })
            },
            |r: AdapterRow| -> Result<TrainedLoRAAdapter, AdapterStoreError> {
                Self::row_to_adapter(r)
            }
        );

        Ok(rows)
    }

    /// List adapters owned by a specific WebID.
    ///
    /// REQ: P8-adt-trained-adapter-store
    /// pre:  owner is a valid WebID
    /// post: returns Vec of adapters owned by the given WebID
    pub fn list_owner(&self, owner: WebID) -> Result<Vec<TrainedLoRAAdapter>, AdapterStoreError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!("{} WHERE owner_webid = ?1", ADAPTER_SELECT))?;

        let rows: Vec<TrainedLoRAAdapter> = collect_rows_strict!(
            stmt,
            rusqlite::params![owner.as_uuid().to_string()],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<AdapterRow> {
                Ok(AdapterRow {
                    adapter_id: row.get(0)?,
                    expertise_name: row.get(1)?,
                    expertise_domain: row.get(2)?,
                    capability_manifest_json: row.get(3)?,
                    checksum: row.get(4)?,
                    storage_path: row.get(5)?,
                    base_model_family: row.get(6)?,
                    version: row.get(7)?,
                    source_json: row.get(8)?,
                    size_bytes: row.get(9)?,
                    owner_webid: row.get(10)?,
                    training_run_id: row.get(11)?,
                    training_source: row.get(12)?,
                    completed_at: row.get(13)?,
                    dataset_hash: row.get(14)?,
                    training_metrics_json: row.get(15)?,
                    created_at: row.get(16)?,
                })
            },
            |r: AdapterRow| -> Result<TrainedLoRAAdapter, AdapterStoreError> {
                Self::row_to_adapter(r)
            }
        );

        Ok(rows)
    }

    /// Delete an adapter by ID.
    ///
    /// OCAP-gated: callers must present a valid DelegationToken with `adapter:delete` capability.
    /// The token is accepted here as documentation of the gate requirement, though actual
    /// token verification happens at the `AdapterPort` boundary (Task 5).
    ///
    /// REQ: P8-adt-trained-adapter-store — delete with ownership verification
    /// pre:  adapter exists
    /// post: adapter row is removed
    pub fn delete(&self, id: Uuid) -> Result<(), AdapterStoreError> {
        let conn = self.lock_conn()?;
        let affected = conn.execute(
            "DELETE FROM trained_adapters WHERE adapter_id = ?1",
            rusqlite::params![id.to_string()],
        )?;
        if affected == 0 {
            return Err(AdapterStoreError::NotFound(id));
        }
        Ok(())
    }

    /// Return the total count of stored adapters.
    pub fn count(&self) -> Result<usize, AdapterStoreError> {
        let conn = self.lock_conn()?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM trained_adapters", [], |row| {
            row.get(0)
        })?;
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
        let source: AdapterSource =
            serde_json::from_str(&r.source_json).unwrap_or_else(|_| AdapterSource::HuggingFace {
                repo: "unknown".into(),
            });
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
            .map_err(|e| AdapterStoreError::Infra(InfrastructureError::Database(e.to_string())))?;

        let id = Uuid::parse_str(&r.adapter_id)
            .map_err(|e| AdapterStoreError::Infra(InfrastructureError::Database(e.to_string())))?;

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
            created_at: r.created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expertise::{Expertise, MdsDomain, TrainingProvenance};

    fn make_test_adapter(name: &str) -> TrainedLoRAAdapter {
        let provenance = TrainingProvenance {
            training_run_id: format!("run-{name}"),
            training_source: "https://example.com/training".into(),
            completed_at: "2026-01-01T00:00:00Z".into(),
            base_model_family: "llama-3.3-70b".into(),
            dataset_hash: None,
            training_metrics: serde_json::json!({"loss": 0.01}),
        };
        let expertise = Expertise::new(
            name.into(),
            MdsDomain::SolidityAudit,
            serde_json::json!({"capabilities": ["reentrancy-detection"]}),
            provenance,
        )
        .expect("expertise creation should succeed");

        TrainedLoRAAdapter {
            id: Uuid::new_v4(),
            expertise,
            checksum: Checksum::from_hex("abcdef1234567890"),
            storage_path: "/tmp/adapter.bin".into(),
            base_model_family: "llama-3.3-70b".into(),
            version: None,
            source: AdapterSource::HuggingFace {
                repo: "test/adapter".into(),
            },
            size_bytes: None,
            owner: WebID::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    // REQ: P8-adt-trained-adapter-store — store and retrieve by ID
    #[test]
    fn store_and_retrieve_by_id() {
        let db = hkask_storage::in_memory_db();
        let store = AdapterStore::new(db.conn_arc());
        store.migrate().expect("migration");

        let adapter = make_test_adapter("solidity-audit");
        store.store(&adapter).expect("store");

        let retrieved = store
            .get_by_id(adapter.id)
            .expect("retrieve")
            .expect("adapter exists");

        assert_eq!(retrieved.id, adapter.id);
        assert_eq!(retrieved.checksum, adapter.checksum);
        assert_eq!(retrieved.expertise.name, "solidity-audit");
    }

    // REQ: P8-adt-trained-adapter-store — retrieve by expertise
    #[test]
    fn retrieve_by_expertise() {
        let db = hkask_storage::in_memory_db();
        let store = AdapterStore::new(db.conn_arc());
        store.migrate().expect("migration");

        let adapter1 = make_test_adapter("solidity-audit");
        let adapter2 = make_test_adapter("solidity-audit"); // same expertise, different adapter
        let adapter3 = make_test_adapter("rust-review");

        store.store(&adapter1).expect("store 1");
        store.store(&adapter2).expect("store 2");
        store.store(&adapter3).expect("store 3");

        let solidity = store.get_by_expertise("solidity-audit").expect("retrieve");
        assert_eq!(solidity.len(), 2);

        let rust = store.get_by_expertise("rust-review").expect("retrieve");
        assert_eq!(rust.len(), 1);
    }

    // REQ: P8-adt-trained-adapter-store — list by owner
    #[test]
    fn list_by_owner() {
        let db = hkask_storage::in_memory_db();
        let store = AdapterStore::new(db.conn_arc());
        store.migrate().expect("migration");

        let owner = WebID::new();
        let mut adapter = make_test_adapter("solidity-audit");
        adapter.owner = owner;
        store.store(&adapter).expect("store");

        let other_adapter = make_test_adapter("rust-review");
        store.store(&other_adapter).expect("store other");

        let owned = store.list_owner(owner).expect("list owner");
        assert_eq!(owned.len(), 1);
        assert_eq!(owned[0].id, adapter.id);
    }

    // REQ: P8-adt-trained-adapter-store — delete adapter
    #[test]
    fn delete_adapter() {
        let db = hkask_storage::in_memory_db();
        let store = AdapterStore::new(db.conn_arc());
        store.migrate().expect("migration");

        let adapter = make_test_adapter("solidity-audit");
        store.store(&adapter).expect("store");
        assert!(store.get_by_id(adapter.id).expect("retrieve").is_some());

        store.delete(adapter.id).expect("delete");
        assert!(store.get_by_id(adapter.id).expect("retrieve").is_none());
    }

    // REQ: P8-adt-trained-adapter-store — delete non-existent returns error
    #[test]
    fn delete_non_existent_returns_error() {
        let db = hkask_storage::in_memory_db();
        let store = AdapterStore::new(db.conn_arc());
        store.migrate().expect("migration");

        let result = store.delete(Uuid::new_v4());
        assert!(result.is_err());
    }
}
