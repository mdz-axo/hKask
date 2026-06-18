//! BackupArchive — portable sovereignty archive for hKask cloud deployment.
//!
//! # REQ: DEP-100 — P1 User Sovereignty: downloadable, passphrase-encrypted triple export.
//!
//! Creates a single SQLCipher-encrypted SQLite file containing:
//! 1. A `backup_meta` table with export metadata
//! 2. The user's full live triple set from the source TripleStore

use crate::Store;
use crate::database::Database;
use crate::triples::TripleStore;
use chrono::Utc;
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("No triples found for user")]
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMeta {
    pub webid: String,
    pub source_server_url: String,
    pub exported_at: String,
    pub triple_count: i64,
    pub schema_version: u32,
}

/// Receipt returned after a successful migration import.
///
/// REQ: DEP-200 — P1 User Sovereignty: migration receipt for audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationReceipt {
    /// Number of triples imported (or already present).
    pub triple_count: i64,
    /// Replicant names that were auto-renamed to avoid collision.
    pub renamed_replicants: Vec<(String, String)>,
}

/// Receipt returned after replicant merge.
///
/// REQ: DEP-500 — P5 Migration: merge receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeReceipt {
    pub triple_count: u64,
    pub source: String,
    pub target: String,
}

pub struct BackupArchive {
    db: Database,
    path: PathBuf,
}

impl BackupArchive {
    pub fn create(
        output_path: PathBuf,
        passphrase: &str,
        source: &TripleStore,
        owner_webid: &WebID,
        source_server_url: &str,
    ) -> Result<Self, ArchiveError> {
        if passphrase.len() < 8 {
            return Err(ArchiveError::Database(
                "Passphrase must be at least 8 characters".to_string(),
            ));
        }

        let path_str = output_path.to_str().ok_or_else(|| {
            ArchiveError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid output path",
            ))
        })?;

        let db = Database::open(path_str, passphrase)
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // Create schema
        {
            let conn_arc = db.conn_arc();
            let conn = conn_arc
                .lock()
                .map_err(|e| ArchiveError::Database(e.to_string()))?;
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS backup_meta (
                    webid TEXT NOT NULL,
                    source_server_url TEXT NOT NULL,
                    exported_at TEXT NOT NULL,
                    triple_count INTEGER NOT NULL,
                    schema_version INTEGER NOT NULL DEFAULT 1
                );
                CREATE TABLE IF NOT EXISTS triples (
                    id TEXT PRIMARY KEY,
                    entity TEXT NOT NULL,
                    attribute TEXT NOT NULL,
                    value TEXT NOT NULL,
                    valid_from TEXT NOT NULL,
                    valid_to TEXT,
                    confidence REAL NOT NULL DEFAULT 1.0,
                    perspective TEXT,
                    visibility TEXT NOT NULL DEFAULT 'private',
                    owner_webid TEXT NOT NULL
                );",
            )
            .map_err(|e| ArchiveError::Database(e.to_string()))?;
        }

        let triple_count = Self::copy_triples(&db, source, owner_webid)?;

        let meta = BackupMeta {
            webid: owner_webid.to_string(),
            source_server_url: source_server_url.to_string(),
            exported_at: Utc::now().to_rfc3339(),
            triple_count,
            schema_version: 1,
        };

        {
            let conn_arc = db.conn_arc();
            let conn = conn_arc
                .lock()
                .map_err(|e| ArchiveError::Database(e.to_string()))?;
            conn.execute(
                "INSERT INTO backup_meta (webid, source_server_url, exported_at, triple_count, schema_version)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    meta.webid,
                    meta.source_server_url,
                    meta.exported_at,
                    meta.triple_count,
                    meta.schema_version,
                ],
            )
            .map_err(|e| ArchiveError::Database(e.to_string()))?;
        }

        Ok(Self {
            db,
            path: output_path,
        })
    }

    pub fn open(path: PathBuf, passphrase: &str) -> Result<Self, ArchiveError> {
        let path_str = path.to_str().ok_or_else(|| {
            ArchiveError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid path",
            ))
        })?;
        let db = Database::open(path_str, passphrase)
            .map_err(|e| ArchiveError::Database(e.to_string()))?;
        Ok(Self { db, path })
    }

    pub fn metadata(&self) -> Result<BackupMeta, ArchiveError> {
        let conn_arc = self.db.conn_arc();
        let conn = conn_arc
            .lock()
            .map_err(|e| ArchiveError::Database(e.to_string()))?;
        conn.query_row(
            "SELECT webid, source_server_url, exported_at, triple_count, schema_version FROM backup_meta LIMIT 1",
            [],
            |row| {
                Ok(BackupMeta {
                    webid: row.get(0)?,
                    source_server_url: row.get(1)?,
                    exported_at: row.get(2)?,
                    triple_count: row.get(3)?,
                    schema_version: row.get(4)?,
                })
            },
        )
        .map_err(|e| ArchiveError::Database(e.to_string()))
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn triple_count(&self) -> Result<i64, ArchiveError> {
        let conn_arc = self.db.conn_arc();
        let conn = conn_arc
            .lock()
            .map_err(|e| ArchiveError::Database(e.to_string()))?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM triples", [], |row| row.get(0))
            .map_err(|e| ArchiveError::Database(e.to_string()))?;
        Ok(count)
    }

    /// Import triples from this archive into a target TripleStore.
    ///
    /// REQ: DEP-201 — idempotent CRDT merge via INSERT OR REPLACE by TripleID.
    /// pre:  archive is open; target is a live TripleStore; existing_names are current replicant names
    /// post: all triples upserted into target
    /// post: auto-renamed entities where collision with existing replicant names
    /// post: returns MigrationReceipt with triple_count and renamed_replicants
    pub fn import_into(
        &self,
        target: &TripleStore,
        owner_webid: &WebID,
        existing_replicant_names: &HashSet<String>,
    ) -> Result<MigrationReceipt, ArchiveError> {
        let rows = self.read_triples()?;
        let total = rows.len() as i64;
        let mut renamed: Vec<(String, String)> = Vec::new();
        let date_suffix = Utc::now().format("%Y%m%d").to_string();

        let conn = target
            .lock_conn()
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        for mut row in rows {
            // Auto-rename entity if it collides with an existing replicant name
            if existing_replicant_names.contains(&row.entity) {
                let new_name = format!("{}-migrated-{}", row.entity, date_suffix);
                if !renamed.iter().any(|(old, _)| old == &row.entity) {
                    renamed.push((row.entity.clone(), new_name.clone()));
                }
                row.entity = new_name;
            }

            // Update owner_webid to target user
            row.owner_webid = owner_webid.to_string();

            // Idempotent upsert by TripleID
            conn.execute(
                "INSERT OR REPLACE INTO triples (id, entity, attribute, value, valid_from, valid_to, confidence, perspective, visibility, owner_webid)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    row.id,
                    row.entity,
                    row.attribute,
                    row.value,
                    row.valid_from,
                    row.valid_to,
                    row.confidence,
                    row.perspective,
                    row.visibility,
                    row.owner_webid,
                ],
            )
            .map_err(|e| ArchiveError::Database(e.to_string()))?;
        }

        Ok(MigrationReceipt {
            triple_count: total,
            renamed_replicants: renamed,
        })
    }

    /// Read all triples from this archive.
    fn read_triples(&self) -> Result<Vec<TripleRow>, ArchiveError> {
        let conn_arc = self.db.conn_arc();
        let conn = conn_arc
            .lock()
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, entity, attribute, value, valid_from, valid_to, confidence, perspective, visibility, owner_webid FROM triples",
        )
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        stmt.query_map([], |row| {
            Ok(TripleRow {
                id: row.get(0)?,
                entity: row.get(1)?,
                attribute: row.get(2)?,
                value: row.get(3)?,
                valid_from: row.get(4)?,
                valid_to: row.get(5)?,
                confidence: row.get(6)?,
                perspective: row.get(7)?,
                visibility: row.get(8)?,
                owner_webid: row.get(9)?,
            })
        })
        .map_err(|e| ArchiveError::Database(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ArchiveError::Database(e.to_string()))
    }

    fn copy_triples(
        archive_db: &Database,
        source: &TripleStore,
        owner_webid: &WebID,
    ) -> Result<i64, ArchiveError> {
        let webid_str = owner_webid.to_string();

        let rows: Vec<TripleRow> = {
            let source_conn = source
                .lock_conn()
                .map_err(|e| ArchiveError::Database(e.to_string()))?;

            let mut stmt = source_conn.prepare(
                "SELECT id, entity, attribute, value, valid_from, valid_to, confidence, perspective, visibility, owner_webid
                 FROM triples WHERE owner_webid = ?1",
            )
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

            stmt.query_map(rusqlite::params![webid_str], |row| {
                Ok(TripleRow {
                    id: row.get(0)?,
                    entity: row.get(1)?,
                    attribute: row.get(2)?,
                    value: row.get(3)?,
                    valid_from: row.get(4)?,
                    valid_to: row.get(5)?,
                    confidence: row.get(6)?,
                    perspective: row.get(7)?,
                    visibility: row.get(8)?,
                    owner_webid: row.get(9)?,
                })
            })
            .map_err(|e| ArchiveError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ArchiveError::Database(e.to_string()))?
        };

        let count = rows.len() as i64;
        if count == 0 {
            return Err(ArchiveError::Empty);
        }

        let conn_arc = archive_db.conn_arc();
        let archive_conn = conn_arc
            .lock()
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        for row in &rows {
            archive_conn.execute(
                "INSERT INTO triples (id, entity, attribute, value, valid_from, valid_to, confidence, perspective, visibility, owner_webid)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    row.id,
                    row.entity,
                    row.attribute,
                    row.value,
                    row.valid_from,
                    row.valid_to,
                    row.confidence,
                    row.perspective,
                    row.visibility,
                    row.owner_webid,
                ],
            )
            .map_err(|e| ArchiveError::Database(e.to_string()))?;
        }

        Ok(count)
    }
}

struct TripleRow {
    id: String,
    entity: String,
    attribute: String,
    value: String,
    valid_from: String,
    valid_to: Option<String>,
    confidence: f64,
    perspective: Option<String>,
    visibility: String,
    owner_webid: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::in_memory_db;
    use crate::triples::Triple;
    use hkask_types::WebID;
    use serde_json::json;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(name)
    }

    fn setup_triple_store() -> (TripleStore, WebID) {
        let db = in_memory_db();
        let store = TripleStore::new(db.conn_arc());
        let webid = WebID::new();

        // Create triples table
        store
            .lock_conn()
            .unwrap()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS triples (
                id TEXT PRIMARY KEY,
                entity TEXT NOT NULL,
                attribute TEXT NOT NULL,
                value TEXT NOT NULL,
                valid_from TEXT NOT NULL,
                valid_to TEXT,
                confidence REAL NOT NULL DEFAULT 1.0,
                perspective TEXT,
                visibility TEXT NOT NULL DEFAULT 'private',
                owner_webid TEXT NOT NULL
            );",
            )
            .unwrap();

        // Insert test triples
        let triples = vec![
            ("replicant:ada", "name", json!("Ada")),
            ("replicant:ada", "role", json!("developer")),
            ("goal:1", "status", json!("active")),
        ];
        for (entity, attr, val) in triples {
            let t = Triple::new(entity, attr, val, webid);
            store.insert(&t).unwrap();
        }

        (store, webid)
    }

    // REQ: DEP-TEST-001 — export creates encrypted archive with correct metadata
    #[test]
    fn export_creates_archive_with_metadata() {
        let (store, webid) = setup_triple_store();
        let path = temp_path(&format!("hkask-test-{}.db", uuid::Uuid::new_v4()));

        let archive = BackupArchive::create(
            path.clone(),
            "test-passphrase",
            &store,
            &webid,
            "test-server",
        )
        .unwrap();

        assert_eq!(archive.path(), &path);
        let count = archive.triple_count().unwrap();
        assert_eq!(count, 3);

        let meta = archive.metadata().unwrap();
        assert_eq!(meta.webid, webid.to_string());
        assert_eq!(meta.source_server_url, "test-server");
        assert_eq!(meta.triple_count, 3);
        assert_eq!(meta.schema_version, 1);
    }

    // REQ: DEP-TEST-002 — archive cannot be opened with wrong passphrase
    #[test]
    fn archive_rejects_wrong_passphrase() {
        let (store, webid) = setup_triple_store();
        let path = temp_path(&format!("hkask-test-{}.db", uuid::Uuid::new_v4()));

        BackupArchive::create(path.clone(), "correct-pass", &store, &webid, "srv").unwrap();
        let result = BackupArchive::open(path, "wrong-pass!!");
        assert!(result.is_err());
    }

    // REQ: DEP-TEST-003 — round-trip: export → open → verify counts match
    #[test]
    fn roundtrip_export_open_preserves_triples() {
        let (store, webid) = setup_triple_store();
        let path = temp_path(&format!("hkask-test-{}.db", uuid::Uuid::new_v4()));

        let archive =
            BackupArchive::create(path.clone(), "test-pass", &store, &webid, "srv").unwrap();
        drop(archive);

        let reopened = BackupArchive::open(path, "test-pass").unwrap();
        assert_eq!(reopened.triple_count().unwrap(), 3);

        let meta = reopened.metadata().unwrap();
        assert_eq!(meta.triple_count, 3);
    }

    // REQ: DEP-TEST-004 — import into empty target produces correct count
    #[test]
    fn import_into_empty_target() {
        let (source, src_webid) = setup_triple_store();
        let path = temp_path(&format!("hkask-test-{}.db", uuid::Uuid::new_v4()));

        let archive =
            BackupArchive::create(path.clone(), "test-pass", &source, &src_webid, "srv").unwrap();

        // Create empty target store
        let target_db = in_memory_db();
        let target = TripleStore::new(target_db.conn_arc());
        target
            .lock_conn()
            .unwrap()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS triples (
                id TEXT PRIMARY KEY, entity TEXT NOT NULL, attribute TEXT NOT NULL,
                value TEXT NOT NULL, valid_from TEXT NOT NULL, valid_to TEXT,
                confidence REAL NOT NULL DEFAULT 1.0, perspective TEXT,
                visibility TEXT NOT NULL DEFAULT 'private', owner_webid TEXT NOT NULL
            );",
            )
            .unwrap();

        let target_webid = WebID::new();
        let names = HashSet::new();
        let receipt = archive.import_into(&target, &target_webid, &names).unwrap();

        assert_eq!(receipt.triple_count, 3);
        assert!(receipt.renamed_replicants.is_empty());

        // Verify triples in target
        let target_count: i64 = target
            .lock_conn()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM triples", [], |r| r.get(0))
            .unwrap();
        assert_eq!(target_count, 3);

        // Verify owner_webid updated
        let owner: String = target
            .lock_conn()
            .unwrap()
            .query_row("SELECT owner_webid FROM triples LIMIT 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(owner, target_webid.to_string());
    }

    // REQ: DEP-TEST-005 — idempotent re-import produces same result
    #[test]
    fn import_is_idempotent() {
        let (source, src_webid) = setup_triple_store();
        let path = temp_path(&format!("hkask-test-{}.db", uuid::Uuid::new_v4()));

        let archive = BackupArchive::create(path, "test-pass", &source, &src_webid, "srv").unwrap();

        let target_db = in_memory_db();
        let target = TripleStore::new(target_db.conn_arc());
        target
            .lock_conn()
            .unwrap()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS triples (
                id TEXT PRIMARY KEY, entity TEXT NOT NULL, attribute TEXT NOT NULL,
                value TEXT NOT NULL, valid_from TEXT NOT NULL, valid_to TEXT,
                confidence REAL NOT NULL DEFAULT 1.0, perspective TEXT,
                visibility TEXT NOT NULL DEFAULT 'private', owner_webid TEXT NOT NULL
            );",
            )
            .unwrap();

        let tw = WebID::new();
        let names = HashSet::new();

        let r1 = archive.import_into(&target, &tw, &names).unwrap();
        assert_eq!(r1.triple_count, 3);

        let r2 = archive.import_into(&target, &tw, &names).unwrap();
        assert_eq!(r2.triple_count, 3);

        // Count should still be 3 (INSERT OR REPLACE)
        let count: i64 = target
            .lock_conn()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM triples", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 3);
    }

    // REQ: DEP-TEST-006 — auto-rename on replicant name collision
    #[test]
    fn import_auto_renames_on_collision() {
        let (source, src_webid) = setup_triple_store();
        let path = temp_path(&format!("hkask-test-{}.db", uuid::Uuid::new_v4()));

        let archive = BackupArchive::create(path, "test-pass", &source, &src_webid, "srv").unwrap();

        let target_db = in_memory_db();
        let target = TripleStore::new(target_db.conn_arc());
        target
            .lock_conn()
            .unwrap()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS triples (
                id TEXT PRIMARY KEY, entity TEXT NOT NULL, attribute TEXT NOT NULL,
                value TEXT NOT NULL, valid_from TEXT NOT NULL, valid_to TEXT,
                confidence REAL NOT NULL DEFAULT 1.0, perspective TEXT,
                visibility TEXT NOT NULL DEFAULT 'private', owner_webid TEXT NOT NULL
            );",
            )
            .unwrap();

        let tw = WebID::new();
        let mut names = HashSet::new();
        names.insert("replicant:ada".to_string());

        let receipt = archive.import_into(&target, &tw, &names).unwrap();
        assert_eq!(receipt.triple_count, 3);
        assert_eq!(receipt.renamed_replicants.len(), 1);
        assert_eq!(receipt.renamed_replicants[0].0, "replicant:ada");
        assert!(receipt.renamed_replicants[0].1.contains("migrated"));
    }
}
