//! SpecStore — SQLite-backed specification storage
//!
//! Also provides `SqliteCurationRecordStore` for persisting spec curation
//! decisions (MDS §5 Curation).
use crate::Store;
use crate::spec_types::{Spec, SpecCategory, SpecCurationRecord, SpecError, SpecId};
use chrono::{DateTime, Utc};
use hkask_types::InfrastructureError;
use hkask_types::curation::{CurationDecision, OCAPBoundary};
define_store!(SqliteSpecStore);
define_store!(SqliteCurationRecordStore);
/// SpecStore trait — storage abstraction for MDS specifications.
///
/// Trait-object-safe (no `Self: Sized` constraints on methods).
/// Implemented by `SqliteSpecStore`.
pub trait SpecStore: Send + Sync {
    fn save(&self, spec: &Spec) -> Result<(), SpecError>;
    fn load(&self, id: SpecId) -> Result<Spec, SpecError>;
    fn delete(&self, id: SpecId) -> Result<(), SpecError>;
    fn list_all(&self) -> Result<Vec<Spec>, SpecError>;
    fn list_by_category(&self, cat: SpecCategory) -> Result<Vec<Spec>, SpecError>;
    fn list_valid_at(&self, at: DateTime<Utc>) -> Result<Vec<Spec>, SpecError>;
    fn list_valid_in_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Spec>, SpecError>;
    fn list_since(&self, since: DateTime<Utc>) -> Result<Vec<Spec>, SpecError>;
    fn expire(&self, id: SpecId, valid_to: DateTime<Utc>) -> Result<(), SpecError>;
}
// ═══════════════════════════════════════════════════════════════════════════
// SqliteSpecStore — trait implementations
impl SpecStore for SqliteSpecStore {
    fn save(&self, spec: &Spec) -> Result<(), SpecError> {
        self.save_inner(spec)
    }
    fn load(&self, id: SpecId) -> Result<Spec, SpecError> {
        self.load_inner(id)
    }
    fn delete(&self, id: SpecId) -> Result<(), SpecError> {
        self.delete_inner(id)
    }
    fn list_all(&self) -> Result<Vec<Spec>, SpecError> {
        self.list_all_inner()
    }
    fn list_by_category(&self, cat: SpecCategory) -> Result<Vec<Spec>, SpecError> {
        self.list_by_category_inner(cat)
    }
    fn list_valid_at(&self, at: DateTime<Utc>) -> Result<Vec<Spec>, SpecError> {
        self.list_valid_at_inner(at)
    }
    fn list_valid_in_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Spec>, SpecError> {
        self.list_valid_in_range_inner(from, to)
    }
    fn list_since(&self, since: DateTime<Utc>) -> Result<Vec<Spec>, SpecError> {
        self.list_since_inner(since)
    }
    fn expire(&self, id: SpecId, valid_to: DateTime<Utc>) -> Result<(), SpecError> {
        self.expire_inner(id, valid_to)
    }
}
// ── Shared row extraction helpers ────────────────────────────────────────
fn row_to_spec(row: &rusqlite::Row<'_>) -> rusqlite::Result<Spec> {
    let data: String = row.get(0)?;
    serde_json::from_str(&data).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}
fn row_to_curation_record(
    row: &rusqlite::Row<'_>,
    spec_id: SpecId,
    decision_idx: usize,
    ocap_idx: usize,
) -> rusqlite::Result<SpecCurationRecord> {
    let decision_str: String = row.get(decision_idx)?;
    let rationale: String = row.get(1 + decision_idx)?;
    let coherence_score: f64 = row.get(2 + decision_idx)?;
    let boundary_json: String = row.get(ocap_idx)?;
    let curated_at_str: String = row.get(ocap_idx + 1)?;
    let decision = CurationDecision::try_from(decision_str.as_str()).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            decision_idx,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid decision",
            )),
        )
    })?;
    let ocap_boundary: OCAPBoundary = serde_json::from_str(&boundary_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            ocap_idx,
            rusqlite::types::Type::Text,
            Box::new(e),
        )
    })?;
    let curated_at = chrono::DateTime::parse_from_rfc3339(&curated_at_str)
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                ocap_idx + 1,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })?
        .to_utc();
    Ok(SpecCurationRecord {
        spec_id,
        decision,
        rationale,
        coherence_score,
        ocap_boundary,
        curated_at,
    })
}
// ── SqliteSpecStore ──────────────────────────────────────────────────────
impl SqliteSpecStore {
    /// Initialize the spec store schema.
    ///
    /// expect: "The system provides durable storage for spec data"
    /// \[P3\] Motivating: Generative Space — schema for specification documents
    /// post: specs table created if not exists
    pub fn init_schema(&self) -> Result<(), SpecError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS specs (
                id TEXT PRIMARY KEY, name TEXT NOT NULL, category TEXT NOT NULL,
                domain_anchor TEXT NOT NULL, signed_by TEXT, signature TEXT,
                created_at TEXT NOT NULL, valid_from TEXT, valid_to TEXT, data TEXT NOT NULL
            )",
            [],
        )?;
        Ok(())
    }
}
// ── SqliteCurationRecordStore ────────────────────────────────────────────
impl SqliteCurationRecordStore {
    /// Initialize the curation record store schema.
    ///
    /// expect: "The system provides durable storage for spec data"
    /// \[P3\] Motivating: Generative Space — schema for curation records
    /// post: spec_curation_records table created if not exists
    pub fn init_schema(&self) -> Result<(), SpecError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS spec_curation_records (
                spec_id TEXT NOT NULL, decision TEXT NOT NULL, rationale TEXT NOT NULL,
                coherence_score REAL NOT NULL, ocap_boundary TEXT NOT NULL,
                curated_at TEXT NOT NULL, recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )?;
        Ok(())
    }
    /// Save a curation record.
    ///
    /// expect: "The system provides durable storage for spec data"
    /// \[P3\] Motivating: Generative Space — save a curation decision
    /// pre:  record.spec_id is non-empty
    /// post: record inserted into spec_curation_records
    pub fn save_curation_record(&self, record: &SpecCurationRecord) -> Result<(), SpecError> {
        let conn = self.lock_conn()?;
        let boundary_json = serde_json::to_string(&record.ocap_boundary)
            .map_err(|e| SpecError::Infra(InfrastructureError::Serialization(e.to_string())))?;
        conn.execute(
            "INSERT INTO spec_curation_records (spec_id, decision, rationale, coherence_score, ocap_boundary, curated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                record.spec_id.to_string(), record.decision.to_string(),
                record.rationale, record.coherence_score,
                boundary_json, record.curated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| SpecError::Infra(InfrastructureError::Database(e.to_string())))?;
        Ok(())
    }
    /// Load curation records for a spec.
    ///
    /// expect: "The system provides durable storage for spec data"
    /// \[P3\] Motivating: Generative Space — load curation records for a spec
    /// pre:  spec_id is non-empty
    /// post: returns Vec of curation records for this spec
    pub fn load_curation_records(
        &self,
        spec_id: SpecId,
    ) -> Result<Vec<SpecCurationRecord>, SpecError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT decision, rationale, coherence_score, ocap_boundary, curated_at
             FROM spec_curation_records WHERE spec_id = ?1 ORDER BY curated_at DESC",
        )?;
        Ok(collect_rows!(
            stmt,
            rusqlite::params![spec_id.to_string()],
            |row| { row_to_curation_record(row, spec_id, 0, 3) }
        ))
    }
    /// List curation records since a timestamp.
    ///
    /// expect: "The system provides durable storage for spec data"
    /// \[P3\] Motivating: Generative Space — list curation records since timestamp
    /// post: returns Vec of records created after since_ts
    pub fn list_curation_records_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<SpecCurationRecord>, SpecError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT spec_id, decision, rationale, coherence_score, ocap_boundary, curated_at
             FROM spec_curation_records WHERE recorded_at >= ?1 ORDER BY recorded_at DESC",
        )?;
        let records = collect_rows!(stmt, rusqlite::params![since.to_rfc3339()], |row| {
            let s: String = row.get(0)?;
            let spec_id = SpecId::from_string(&s).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
                )
            })?;
            row_to_curation_record(row, spec_id, 1, 4)
        });
        Ok(records)
    }
    /// Load all curation records.
    ///
    /// expect: "The system provides durable storage for spec data"
    /// \[P3\] Motivating: Generative Space — load all curation records
    /// post: returns Vec of all curation records
    pub fn load_all_curation_records(&self) -> Result<Vec<SpecCurationRecord>, SpecError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT spec_id, decision, rationale, coherence_score, ocap_boundary, curated_at
             FROM spec_curation_records ORDER BY recorded_at DESC",
        )?;
        let records = collect_rows!(stmt, [], |row| {
            let s: String = row.get(0)?;
            let spec_id = SpecId::from_string(&s).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
                )
            })?;
            row_to_curation_record(row, spec_id, 1, 4)
        });
        Ok(records)
    }
}
// ── SqliteSpecStore inherent methods ──────────────────────────────────────
impl SqliteSpecStore {
    fn load_inner(&self, id: SpecId) -> Result<Spec, SpecError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT data FROM specs WHERE id = ?1")?;
        let data: String = stmt
            .query_row(rusqlite::params![id.to_string()], |row| row.get(0))
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => SpecError::NotFound(id),
                other => SpecError::from(other),
            })?;
        serde_json::from_str(&data).map_err(Into::into)
    }
    fn save_inner(&self, spec: &Spec) -> Result<(), SpecError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO specs (id, name, category, domain_anchor, signed_by, signature, created_at, valid_from, valid_to, data)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                spec.id.to_string(), spec.name, spec.category.as_str(), spec.domain_anchor.as_str(),
                spec.signed_by.map(|w| w.to_string()), spec.signature.as_deref(),
                spec.created_at.to_rfc3339(),
                spec.valid_from.map(|dt| dt.to_rfc3339()),
                spec.valid_to.map(|dt| dt.to_rfc3339()),
                serde_json::to_string(spec)?,
            ],
        )?;
        Ok(())
    }
    fn delete_inner(&self, id: SpecId) -> Result<(), SpecError> {
        let conn = self.lock_conn()?;
        let changed = conn.execute(
            "DELETE FROM specs WHERE id = ?1",
            rusqlite::params![id.to_string()],
        )?;
        if changed == 0 {
            Err(SpecError::NotFound(id))
        } else {
            Ok(())
        }
    }
    fn list_all_inner(&self) -> Result<Vec<Spec>, SpecError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT data FROM specs")?;
        Ok(collect_rows!(stmt, [], row_to_spec))
    }
    fn list_by_category_inner(&self, cat: SpecCategory) -> Result<Vec<Spec>, SpecError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT data FROM specs WHERE category = ?1")?;
        Ok(collect_rows!(
            stmt,
            rusqlite::params![cat.as_str()],
            row_to_spec
        ))
    }
    fn list_valid_at_inner(&self, at: DateTime<Utc>) -> Result<Vec<Spec>, SpecError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT data FROM specs WHERE (valid_from IS NULL OR valid_from <= ?1)
               AND (valid_to IS NULL OR valid_to > ?1)",
        )?;
        Ok(collect_rows!(
            stmt,
            rusqlite::params![at.to_rfc3339()],
            row_to_spec
        ))
    }
    fn list_valid_in_range_inner(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Spec>, SpecError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT data FROM specs WHERE valid_from IS NOT NULL
               AND valid_from <= ?2 AND (valid_to IS NULL OR valid_to >= ?1)",
        )?;
        Ok(collect_rows!(
            stmt,
            rusqlite::params![from.to_rfc3339(), to.to_rfc3339()],
            row_to_spec
        ))
    }
    fn list_since_inner(&self, since: DateTime<Utc>) -> Result<Vec<Spec>, SpecError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT data FROM specs WHERE created_at >= ?1")?;
        Ok(collect_rows!(
            stmt,
            rusqlite::params![since.to_rfc3339()],
            row_to_spec
        ))
    }
    fn expire_inner(&self, id: SpecId, valid_to: DateTime<Utc>) -> Result<(), SpecError> {
        let conn = self.lock_conn()?;
        let changed = conn.execute(
            "UPDATE specs SET valid_to = ?1 WHERE id = ?2",
            rusqlite::params![valid_to.to_rfc3339(), id.to_string()],
        )?;
        if changed == 0 {
            Err(SpecError::NotFound(id))
        } else {
            Ok(())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_types::DomainAnchor;
    use chrono::Duration;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};
    fn make_store() -> SqliteSpecStore {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        let store = SqliteSpecStore::new(conn);
        store.init_schema().unwrap();
        store
    }
    fn make_spec(name: &str, category: SpecCategory) -> Spec {
        Spec::new(name, category, DomainAnchor::Hkask)
    }
    #[test]
    fn list_valid_at_includes_currently_valid_specs() {
        let store = make_store();
        let now = Utc::now();
        let spec = make_spec("test", SpecCategory::Domain)
            .with_valid_from(now - Duration::hours(1))
            .with_valid_to(now + Duration::hours(1));
        store.save(&spec).unwrap();
        let valid = store.list_valid_at(now).unwrap();
        assert_eq!(valid.len(), 1);
        assert_eq!(valid[0].name, "test");
    }
    #[test]
    fn list_valid_at_excludes_expired_specs() {
        let store = make_store();
        let now = Utc::now();
        let spec = make_spec("expired", SpecCategory::Domain)
            .with_valid_from(now - Duration::hours(2))
            .with_valid_to(now - Duration::hours(1));
        store.save(&spec).unwrap();
        assert!(store.list_valid_at(now).unwrap().is_empty());
    }
    #[test]
    fn list_valid_at_includes_no_expiry_specs() {
        let store = make_store();
        let now = Utc::now();
        let spec =
            make_spec("perpetual", SpecCategory::Domain).with_valid_from(now - Duration::hours(1));
        store.save(&spec).unwrap();
        assert_eq!(store.list_valid_at(now).unwrap().len(), 1);
    }
    #[test]
    fn list_valid_in_range_overlap_query() {
        let store = make_store();
        let now = Utc::now();
        let spec = make_spec("overlap", SpecCategory::Domain)
            .with_valid_from(now - Duration::hours(2))
            .with_valid_to(now + Duration::hours(2));
        store.save(&spec).unwrap();
        assert_eq!(
            store
                .list_valid_in_range(now - Duration::hours(5), now + Duration::hours(5))
                .unwrap()
                .len(),
            1
        );
    }
    #[test]
    fn list_since_transaction_time_query() {
        let store = make_store();
        let now = Utc::now();
        store
            .save(&make_spec("recent", SpecCategory::Domain))
            .unwrap();
        assert!(
            store
                .list_since(now + Duration::hours(1))
                .unwrap()
                .is_empty()
        );
        assert_eq!(store.list_since(now - Duration::hours(1)).unwrap().len(), 1);
    }
    #[test]
    fn expire_updates_valid_to() {
        let store = make_store();
        let now = Utc::now();
        let spec =
            make_spec("temp", SpecCategory::Domain).with_valid_from(now - Duration::hours(2));
        store.save(&spec).unwrap();
        assert_eq!(store.list_valid_at(now).unwrap().len(), 1);
        store.expire(spec.id, now - Duration::hours(1)).unwrap();
        assert!(store.list_valid_at(now).unwrap().is_empty());
    }
}
