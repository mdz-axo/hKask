//! SpecStore — SQLite-backed specification storage
//!
//! Also provides `SqliteCurationRecordStore` for persisting spec curation
//! decisions (DDMVSS §5.9, audit remediation R17).

use crate::Store;
use crate::spec_types::{Spec, SpecCategory, SpecCurationRecord, SpecError, SpecId, SpecStore};
use chrono::{DateTime, Utc};
use hkask_types::InfrastructureError;

define_store!(SqliteSpecStore);

define_store!(SqliteCurationRecordStore);

impl SqliteSpecStore {
    pub fn init_schema(&self) -> Result<(), SpecError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS specs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                category TEXT NOT NULL,
                domain_anchor TEXT NOT NULL,
                signed_by TEXT,
                signature TEXT,
                created_at TEXT NOT NULL,
                valid_from TEXT,
                valid_to TEXT,
                data TEXT NOT NULL
            )",
            [],
        )?;
        Ok(())
    }
}

impl SqliteCurationRecordStore {
    /// Initialize the curation records schema.
    ///
    /// Creates the `spec_curation_records` table for persisting curation
    /// decisions (DDMVSS §5.9, audit remediation R17).
    ///
    /// `recorded_at` provides the transaction-time dimension for bitemporal
    /// tracking: when the curation decision was _recorded_ in the system,
    /// distinct from `curated_at` (when the Curator performed the evaluation).
    pub fn init_schema(&self) -> Result<(), SpecError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS spec_curation_records (
                spec_id TEXT NOT NULL,
                decision TEXT NOT NULL,
                rationale TEXT NOT NULL,
                coherence_score REAL NOT NULL,
                ocap_boundary TEXT NOT NULL,
                curated_at TEXT NOT NULL,
                recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )?;
        Ok(())
    }

    /// Persist a curation record.
    ///
    /// Each call to `DefaultSpecCurator::evaluate()` produces a record;
    /// this method stores it for audit and bitemporal tracking.
    pub fn save_curation_record(&self, record: &SpecCurationRecord) -> Result<(), SpecError> {
        let conn = self.lock_conn()?;
        let boundary_json = serde_json::to_string(&record.ocap_boundary)
            .map_err(|e| SpecError::Infra(InfrastructureError::Serialization(e.to_string())))?;
        conn.execute(
            "INSERT INTO spec_curation_records (spec_id, decision, rationale, coherence_score, ocap_boundary, curated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                record.spec_id.to_string(),
                record.decision.to_string(),
                record.rationale,
                record.coherence_score,
                boundary_json,
                record.curated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| SpecError::Infra(InfrastructureError::Database(e.to_string())))?;
        Ok(())
    }

    /// Load all curation records for a given spec.
    pub fn load_curation_records(
        &self,
        spec_id: SpecId,
    ) -> Result<Vec<SpecCurationRecord>, SpecError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT decision, rationale, coherence_score, ocap_boundary, curated_at
             FROM spec_curation_records WHERE spec_id = ?1
             ORDER BY curated_at DESC",
        )?;
        let records = collect_rows!(
            stmt,
            rusqlite::params![spec_id.to_string()],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<SpecCurationRecord> {
                let spec_id: SpecId = spec_id;
                let decision_str: String = row.get(0)?;
                let rationale: String = row.get(1)?;
                let coherence_score: f64 = row.get(2)?;
                let boundary_json: String = row.get(3)?;
                let curated_at_str: String = row.get(4)?;

                use hkask_types::curation::{CurationDecision, OCAPBoundary};
                let decision = CurationDecision::try_from(decision_str.as_str()).map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "invalid decision",
                        )),
                    )
                })?;
                let ocap_boundary: OCAPBoundary =
                    serde_json::from_str(&boundary_json).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
                let curated_at = chrono::DateTime::parse_from_rfc3339(&curated_at_str)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
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
        );
        Ok(records)
    }

    /// List all curation records recorded since the given timestamp.
    ///
    /// Uses `recorded_at` (transaction-time) — when the decision was persisted
    /// to the database, distinct from `curated_at` (when evaluation occurred).
    /// DDMVSS §11 #2: bitemporal semantics for curation audit trail.
    pub fn list_curation_records_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<SpecCurationRecord>, SpecError> {
        let conn = self.lock_conn()?;
        let since_str = since.to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT spec_id, decision, rationale, coherence_score, ocap_boundary, curated_at
             FROM spec_curation_records WHERE recorded_at >= ?1
             ORDER BY recorded_at DESC",
        )?;

        let records = collect_rows!(stmt, rusqlite::params![since_str], |row: &rusqlite::Row<
            '_,
        >|
         -> rusqlite::Result<
            SpecCurationRecord,
        > {
            let spec_id_str: String = row.get(0)?;
            let decision_str: String = row.get(1)?;
            let rationale: String = row.get(2)?;
            let coherence_score: f64 = row.get(3)?;
            let boundary_json: String = row.get(4)?;
            let curated_at_str: String = row.get(5)?;

            let spec_id = SpecId::from_string(&spec_id_str).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
                )
            })?;

            use hkask_types::curation::{CurationDecision, OCAPBoundary};
            let decision = CurationDecision::try_from(decision_str.as_str()).map_err(|_| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "invalid decision",
                    )),
                )
            })?;
            let ocap_boundary: OCAPBoundary =
                serde_json::from_str(&boundary_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
            let curated_at = chrono::DateTime::parse_from_rfc3339(&curated_at_str)
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
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
        });
        Ok(records)
    }
}

impl SpecStore for SqliteSpecStore {
    fn load(&self, id: SpecId) -> Result<Spec, SpecError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT data FROM specs WHERE id = ?1")?;
        let data: String = stmt
            .query_row(rusqlite::params![id.to_string()], |row| row.get(0))
            .map_err(|_| SpecError::NotFound(id))?;
        serde_json::from_str(&data).map_err(Into::into)
    }

    fn save(&self, spec: &Spec) -> Result<(), SpecError> {
        let conn = self.lock_conn()?;
        let data = serde_json::to_string(spec)?;
        let signed_by = spec.signed_by.map(|w| w.to_string());
        let signature = spec.signature.as_deref();
        let valid_from = spec.valid_from.map(|dt| dt.to_rfc3339());
        let valid_to = spec.valid_to.map(|dt| dt.to_rfc3339());
        conn.execute(
            "INSERT OR REPLACE INTO specs (id, name, category, domain_anchor, signed_by, signature, created_at, valid_from, valid_to, data)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                spec.id.to_string(),
                spec.name,
                spec.category.as_str(),
                spec.domain_anchor.as_str(),
                signed_by,
                signature,
                spec.created_at.to_rfc3339(),
                valid_from,
                valid_to,
                data,
            ],
        )?;
        Ok(())
    }

    fn delete(&self, id: SpecId) -> Result<(), SpecError> {
        let conn = self.lock_conn()?;
        let changed = conn.execute(
            "DELETE FROM specs WHERE id = ?1",
            rusqlite::params![id.to_string()],
        )?;
        if changed == 0 {
            return Err(SpecError::NotFound(id));
        }
        Ok(())
    }

    fn list_all(&self) -> Result<Vec<Spec>, SpecError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT data FROM specs")?;
        let specs = collect_rows!(
            stmt,
            [],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<Spec> {
                let data: String = row.get(0)?;
                serde_json::from_str(&data).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })
            }
        );
        Ok(specs)
    }

    fn list_by_category(&self, cat: SpecCategory) -> Result<Vec<Spec>, SpecError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT data FROM specs WHERE category = ?1")?;
        let specs = collect_rows!(
            stmt,
            rusqlite::params![cat.as_str()],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<Spec> {
                let data: String = row.get(0)?;
                serde_json::from_str(&data).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })
            }
        );
        Ok(specs)
    }

    fn list_valid_at(&self, at: DateTime<Utc>) -> Result<Vec<Spec>, SpecError> {
        let conn = self.lock_conn()?;
        let at_str = at.to_rfc3339();
        // Spec is valid if: (valid_from IS NULL OR valid_from <= at)
        //               AND (valid_to IS NULL OR valid_to > at)
        let mut stmt = conn.prepare(
            "SELECT data FROM specs \
             WHERE (valid_from IS NULL OR valid_from <= ?1) \
               AND (valid_to IS NULL OR valid_to > ?1)",
        )?;
        let specs = collect_rows!(stmt, rusqlite::params![at_str], |row: &rusqlite::Row<
            '_,
        >|
         -> rusqlite::Result<
            Spec,
        > {
            let data: String = row.get(0)?;
            serde_json::from_str(&data).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })
        });
        Ok(specs)
    }

    fn list_valid_in_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Spec>, SpecError> {
        let conn = self.lock_conn()?;
        let from_str = from.to_rfc3339();
        let to_str = to.to_rfc3339();
        // Spec has temporal window: valid_from IS NOT NULL
        // And overlaps: valid_from <= to AND (valid_to IS NULL OR valid_to >= from)
        let mut stmt = conn.prepare(
            "SELECT data FROM specs \
             WHERE valid_from IS NOT NULL \
               AND valid_from <= ?2 \
               AND (valid_to IS NULL OR valid_to >= ?1)",
        )?;
        let specs = collect_rows!(
            stmt,
            rusqlite::params![from_str, to_str],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<Spec> {
                let data: String = row.get(0)?;
                serde_json::from_str(&data).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })
            }
        );
        Ok(specs)
    }

    fn list_since(&self, since: DateTime<Utc>) -> Result<Vec<Spec>, SpecError> {
        let conn = self.lock_conn()?;
        let since_str = since.to_rfc3339();
        let mut stmt = conn.prepare("SELECT data FROM specs WHERE created_at >= ?1")?;
        let specs = collect_rows!(stmt, rusqlite::params![since_str], |row: &rusqlite::Row<
            '_,
        >|
         -> rusqlite::Result<
            Spec,
        > {
            let data: String = row.get(0)?;
            serde_json::from_str(&data).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })
        });
        Ok(specs)
    }

    fn expire(&self, id: SpecId, valid_to: DateTime<Utc>) -> Result<(), SpecError> {
        let conn = self.lock_conn()?;
        let valid_to_str = valid_to.to_rfc3339();
        let changed = conn.execute(
            "UPDATE specs SET valid_to = ?1 WHERE id = ?2",
            rusqlite::params![valid_to_str, id.to_string()],
        )?;
        if changed == 0 {
            return Err(SpecError::NotFound(id));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_types::{DomainAnchor, GoalSpec};
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

    // P8: list_valid_at returns specs with valid_from <= at < valid_to
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

    // P8: list_valid_at excludes specs whose valid_to has passed
    #[test]
    fn list_valid_at_excludes_expired_specs() {
        let store = make_store();
        let now = Utc::now();

        let spec = make_spec("expired", SpecCategory::Domain)
            .with_valid_from(now - Duration::hours(2))
            .with_valid_to(now - Duration::hours(1));
        store.save(&spec).unwrap();

        let valid = store.list_valid_at(now).unwrap();
        assert!(valid.is_empty());
    }

    // P8: list_valid_at includes specs with valid_to IS NULL (no expiry)
    #[test]
    fn list_valid_at_includes_no_expiry_specs() {
        let store = make_store();
        let now = Utc::now();

        let spec =
            make_spec("perpetual", SpecCategory::Domain).with_valid_from(now - Duration::hours(1));
        store.save(&spec).unwrap();

        let valid = store.list_valid_at(now).unwrap();
        assert_eq!(valid.len(), 1);
    }

    // P8: list_valid_in_range returns specs with overlapping temporal windows
    #[test]
    fn list_valid_in_range_overlap_query() {
        let store = make_store();
        let now = Utc::now();
        let t1 = now - Duration::hours(5);
        let t2 = now + Duration::hours(5);

        let spec = make_spec("overlap", SpecCategory::Domain)
            .with_valid_from(now - Duration::hours(2))
            .with_valid_to(now + Duration::hours(2));
        store.save(&spec).unwrap();

        let results = store.list_valid_in_range(t1, t2).unwrap();
        assert_eq!(results.len(), 1);
    }

    // P8: list_since returns specs created after a timestamp
    #[test]
    fn list_since_transaction_time_query() {
        let store = make_store();
        let now = Utc::now();

        let spec = make_spec("recent", SpecCategory::Domain);
        store.save(&spec).unwrap();

        let since_future = now + Duration::hours(1);
        let results = store.list_since(since_future).unwrap();
        assert!(results.is_empty());

        let since_past = now - Duration::hours(1);
        let results = store.list_since(since_past).unwrap();
        assert_eq!(results.len(), 1);
    }

    // P8: expire sets valid_to and the spec is excluded from list_valid_at
    #[test]
    fn expire_updates_valid_to() {
        let store = make_store();
        let now = Utc::now();

        let spec =
            make_spec("temp", SpecCategory::Domain).with_valid_from(now - Duration::hours(2));
        store.save(&spec).unwrap();

        assert!(store.list_valid_at(now).unwrap().len() == 1);

        store.expire(spec.id, now - Duration::hours(1)).unwrap();

        assert!(store.list_valid_at(now).unwrap().is_empty());
    }
}
