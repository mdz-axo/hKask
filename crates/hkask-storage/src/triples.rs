//! Uni-temporal triples — entity/attribute/value with valid_from/valid_to.
use crate::{Store, collect_rows, now_rfc3339};
use chrono::{DateTime, Utc};
use hkask_ports::git_cas::TripleEntry;
use hkask_types::id::{TripleID, WebID};
use hkask_types::visibility::{AccessControl, TemporalBounds};
use hkask_types::{Confidence, InfrastructureError, Visibility};
use serde_json::Value;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum TripleError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),
    #[error("Triple not found")]
    NotFound,
}
impl_from_rusqlite!(TripleError, Infra);
impl_from_serde_json!(TripleError, Infra);
/// Bitemporal triple
#[derive(Debug, Clone)]
pub struct Triple {
    pub id: TripleID,
    pub entity: String,
    pub attribute: String,
    pub value: Value,
    pub temporal: TemporalBounds,
    pub confidence: Confidence,
    pub access: AccessControl,
}
impl Triple {
    /// Create a new Triple with required fields.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — create a triple
    /// \[P1\] Constraining: User Sovereignty — owner_webid carries ownership
    /// pre:  entity and attribute are non-empty, owner_webid is valid
    /// post: returns Triple with defaults for temporal, confidence, access
    pub fn new(entity: &str, attribute: &str, value: Value, owner_webid: WebID) -> Self {
        Self {
            id: TripleID::new(),
            entity: entity.to_string(),
            attribute: attribute.to_string(),
            value,
            temporal: TemporalBounds::now(),
            confidence: Confidence::full(),
            access: AccessControl::new(owner_webid),
        }
    }
    /// Set confidence on a Triple.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — builder: set confidence
    /// post: returns Self with confidence set (builder pattern)
    pub fn with_confidence(mut self, c: impl Into<Confidence>) -> Self {
        self.confidence = c.into();
        self
    }
    /// Set perspective on a Triple.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — builder: set perspective
    /// post: returns Self with perspective set (builder pattern)
    pub fn with_perspective(mut self, p: WebID) -> Self {
        self.access = self.access.with_perspective(p);
        self
    }
    /// Set visibility on a Triple.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — builder: set visibility
    /// post: returns Self with visibility set (builder pattern)
    pub fn with_visibility(mut self, v: Visibility) -> Self {
        self.access = self.access.with_visibility(v);
        self
    }
    /// Check if this is an episodic triple (has perspective).
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P8\] Motivating: Semantic Grounding — predicate for episodic
    /// post: returns true iff perspective is Some
    pub fn is_episodic(&self) -> bool {
        self.access.is_episodic()
    }
    /// Check if this is a semantic triple (public, no perspective).
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P8\] Motivating: Semantic Grounding — predicate for semantic
    /// post: returns true iff visibility is Public and perspective is None
    pub fn is_semantic(&self) -> bool {
        self.access.is_semantic()
    }
}
define_store!(TripleStore);
const TRIPLE_COLUMNS: &str = "id, entity, attribute, value, valid_from, valid_to, confidence, perspective, visibility, owner_webid";
impl TripleStore {
    /// Insert a triple into the store.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — insert triple into store
    /// pre:  triple has valid entity, attribute, value
    /// post: triple inserted
    pub fn insert(&self, triple: &Triple) -> Result<(), TripleError> {
        let conn = self.lock_conn()?;
        conn.execute(
            &format!("INSERT INTO triples ({TRIPLE_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"),
            rusqlite::params![
                triple.id,
                triple.entity,
                triple.attribute,
                serde_json::to_string(&triple.value)?,
                triple.temporal.valid_from.to_rfc3339(),
                triple.temporal.valid_to.map(|t| t.to_rfc3339()),
                triple.confidence,
                triple.access.perspective,
                triple.access.visibility,
                triple.access.owner_webid,
            ],
        )?;
        Ok(())
    }
    /// Query triples by entity.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — query by entity
    /// pre:  entity is non-empty
    /// post: returns Vec of triples matching entity
    pub fn query_by_entity(&self, entity: &str) -> Result<Vec<Triple>, TripleError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {TRIPLE_COLUMNS} FROM triples WHERE entity = ?1 AND valid_to IS NULL ORDER BY valid_from DESC"
        ))?;
        Ok(collect_rows!(
            stmt,
            rusqlite::params![entity],
            Self::row_to_triple_row,
            Self::row_to_triple
        ))
    }
    /// Query triples by entity and attribute.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — query by entity + attribute
    /// pre:  entity and attribute are non-empty
    /// post: returns Vec of matching triples
    pub fn query_by_entity_attribute(
        &self,
        entity: &str,
        attribute: &str,
    ) -> Result<Vec<Triple>, TripleError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {TRIPLE_COLUMNS} FROM triples WHERE entity = ?1 AND attribute = ?2 AND valid_to IS NULL ORDER BY valid_from DESC"
        ))?;
        Ok(collect_rows!(
            stmt,
            rusqlite::params![entity, attribute],
            Self::row_to_triple_row,
            Self::row_to_triple
        ))
    }
    /// Query triples by perspective.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — query by perspective
    /// pre:  perspective is valid
    /// post: returns Vec of triples for this perspective
    pub fn query_by_perspective(&self, perspective: &WebID) -> Result<Vec<Triple>, TripleError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {TRIPLE_COLUMNS} FROM triples WHERE perspective = ?1 AND valid_to IS NULL ORDER BY valid_from DESC"
        ))?;
        Ok(collect_rows!(
            stmt,
            rusqlite::params![perspective],
            Self::row_to_triple_row,
            Self::row_to_triple
        ))
    }
    /// Query all triples with a given attribute, regardless of entity.
    /// Query triples by attribute.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — query by attribute
    /// pre:  attribute is non-empty
    /// post: returns Vec of triples matching attribute
    pub fn query_by_attribute(&self, attribute: &str) -> Result<Vec<Triple>, TripleError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {TRIPLE_COLUMNS} FROM triples WHERE attribute = ?1 AND valid_to IS NULL ORDER BY valid_from DESC"
        ))?;
        Ok(collect_rows!(
            stmt,
            rusqlite::params![attribute],
            Self::row_to_triple_row,
            Self::row_to_triple
        ))
    }
    /// Update a triple's value (close current version, insert new).
    /// Wrapped in a transaction for atomicity.
    /// Update a triple's value and confidence.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — update value and confidence
    /// pre:  id is valid
    /// post: triple value and confidence updated
    pub fn update(
        &self,
        id: &TripleID,
        new_value: Value,
        new_confidence: impl Into<Confidence>,
    ) -> Result<(), TripleError> {
        let new_confidence = new_confidence.into();
        let conn = self.lock_conn()?;
        let now = now_rfc3339();
        conn.execute("BEGIN IMMEDIATE", [])?;
        let result = (|| -> Result<(), TripleError> {
            conn.execute(
                "UPDATE triples SET valid_to = ?1 WHERE id = ?2 AND valid_to IS NULL",
                rusqlite::params![now, id],
            )?;
            let mut stmt = conn.prepare(
                "SELECT entity, attribute, perspective, visibility, owner_webid
                 FROM triples WHERE id = ?1",
            )?;
            let row = stmt.query_row(rusqlite::params![id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<WebID>>(2)?,
                    row.get::<_, Visibility>(3)?,
                    row.get::<_, WebID>(4)?,
                ))
            })?;
            let access = AccessControl {
                perspective: row.2,
                visibility: row.3,
                owner_webid: row.4,
            };
            let new_id = TripleID::new();
            conn.execute(
                &format!("INSERT INTO triples ({TRIPLE_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, ?9)"),
                rusqlite::params![
                    new_id,
                    row.0,
                    row.1,
                    serde_json::to_string(&new_value)?,
                    now,
                    new_confidence,
                    access.perspective,
                    access.visibility,
                    access.owner_webid,
                ],
            )?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                conn.execute("COMMIT", [])?;
                Ok(())
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                Err(e)
            }
        }
    }
    /// Get a triple by ID.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — get triple by ID
    /// pre:  id is valid
    /// post: returns Some(Triple) if found, None otherwise
    pub fn get_by_id(&self, id: &TripleID) -> Result<Option<Triple>, TripleError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {TRIPLE_COLUMNS} FROM triples WHERE id = ?1 AND valid_to IS NULL"
        ))?;
        // Use strict collection: a corrupt row on a primary-key lookup is an error,
        // not graceful degradation.
        let triples = collect_rows_strict!(
            stmt,
            rusqlite::params![id],
            Self::row_to_triple_row,
            Self::row_to_triple
        );
        Ok(triples.into_iter().next())
    }
    /// Semantic triples with lowest confidence, ordered ASC. Used by consolidation.
    /// Query lowest-confidence semantic triples.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — low-confidence semantic triples
    /// pre:  limit > 0
    /// post: returns up to limit triples ordered by confidence ascending
    pub fn query_semantic_lowest_confidence(
        &self,
        limit: usize,
    ) -> Result<Vec<Triple>, TripleError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {TRIPLE_COLUMNS} FROM triples \
             WHERE perspective IS NULL AND valid_to IS NULL \
             ORDER BY confidence ASC, valid_from ASC \
             LIMIT ?1"
        ))?;
        Ok(collect_rows!(
            stmt,
            rusqlite::params![limit as i64],
            Self::row_to_triple_row,
            Self::row_to_triple
        ))
    }
    /// Count semantic triples below confidence threshold. Used by consolidation.
    /// Count semantic triples below a confidence threshold.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P8\] Motivating: Semantic Grounding — count below threshold
    /// pre:  threshold in [0.0, 1.0]
    /// post: returns count of triples with confidence ≤ threshold
    pub fn count_semantic_below_confidence(&self, threshold: f64) -> Result<usize, TripleError> {
        let conn = self.lock_conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM triples WHERE perspective IS NULL AND valid_to IS NULL AND confidence <= ?1",
            rusqlite::params![threshold],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }
    /// Semantic triples below confidence threshold, ordered ASC. Used by consolidation.
    /// Query semantic triples below a confidence threshold.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — query below threshold
    /// pre:  threshold in [0.0, 1.0], limit > 0
    /// post: returns up to limit triples with confidence ≤ threshold
    pub fn query_semantic_below_confidence(
        &self,
        threshold: f64,
        limit: usize,
    ) -> Result<Vec<Triple>, TripleError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {TRIPLE_COLUMNS} FROM triples \
             WHERE perspective IS NULL AND valid_to IS NULL AND confidence <= ?1 \
             ORDER BY confidence ASC, valid_from ASC \
             LIMIT ?2"
        ))?;
        Ok(collect_rows!(
            stmt,
            rusqlite::params![threshold, limit as i64],
            Self::row_to_triple_row,
            Self::row_to_triple
        ))
    }
    /// Count semantic triples (perspective IS NULL, valid_to IS NULL).
    /// Count all semantic triples.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P8\] Motivating: Semantic Grounding — count semantic triples
    /// post: returns total count of semantic triples
    pub fn count_semantic(&self) -> Result<usize, TripleError> {
        let conn = self.lock_conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM triples WHERE perspective IS NULL AND valid_to IS NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }
    /// Count semantic triples for a given entity.
    /// Count semantic triples for an entity.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P8\] Motivating: Semantic Grounding — count per entity
    /// pre:  entity is non-empty
    /// post: returns count for entity
    pub fn count_semantic_by_entity(&self, entity: &str) -> Result<usize, TripleError> {
        let conn = self.lock_conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM triples WHERE entity = ?1 AND perspective IS NULL AND valid_to IS NULL",
            rusqlite::params![entity],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }
    /// Count triples for a given perspective (episodic).
    /// Count triples by perspective.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P8\] Motivating: Semantic Grounding — count per perspective
    /// pre:  perspective is valid
    /// post: returns count for perspective
    pub fn count_by_perspective(&self, perspective: &WebID) -> Result<usize, TripleError> {
        let conn = self.lock_conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM triples WHERE perspective = ?1 AND valid_to IS NULL",
            rusqlite::params![perspective],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }
    /// Query semantic triples older than N days, grouped by entity for condensation.
    ///
    /// Returns triples with `perspective IS NULL AND valid_to IS NULL` and
    /// `valid_from` earlier than the cutoff date, ordered by entity then
    /// confidence descending (best first), then valid_from descending (most recent first).
    /// This ordering enables the condensation loop to identify the best candidate
    /// to keep per entity group (first in each entity group).
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — query old triples for condensation
    /// \[P9\] Constraining: Homeostatic Self-Regulation — enables semantic condensation trigger
    /// pre:  days > 0, limit > 0
    /// post: returns up to limit triples older than cutoff, ordered by entity, confidence DESC, valid_from DESC
    pub fn query_semantic_older_than(
        &self,
        days: u32,
        limit: usize,
    ) -> Result<Vec<Triple>, TripleError> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(days as i64)).to_rfc3339();
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {TRIPLE_COLUMNS} FROM triples \
             WHERE perspective IS NULL AND valid_to IS NULL AND valid_from < ?1 \
             ORDER BY entity ASC, confidence DESC, valid_from DESC \
             LIMIT ?2"
        ))?;
        Ok(collect_rows!(
            stmt,
            rusqlite::params![cutoff, limit as i64],
            Self::row_to_triple_row,
            Self::row_to_triple
        ))
    }

    /// Soft-delete: set valid_to to close a triple.
    /// Soft-delete a triple by setting valid_to.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — soft-delete triple
    /// pre:  id is valid
    /// post: triple's valid_to set to now (soft-delete)
    pub fn close_by_id(&self, id: &TripleID) -> Result<(), TripleError> {
        let conn = self.lock_conn()?;
        let now = now_rfc3339();
        conn.execute(
            "UPDATE triples SET valid_to = ?1 WHERE id = ?2 AND valid_to IS NULL",
            rusqlite::params![now, id],
        )?;
        Ok(())
    }
    /// Hard-delete a triple row entirely.
    /// Hard-delete a triple by ID.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — hard-delete triple
    /// pre:  id is valid
    /// post: triple permanently deleted
    pub fn delete_by_id(&self, id: &TripleID) -> Result<(), TripleError> {
        let conn = self.lock_conn()?;
        conn.execute("DELETE FROM triples WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    }
    /// Hard-delete all triples whose entity starts with the given prefix.
    /// Returns the number of rows deleted.
    /// Delete triples by entity prefix.
    ///
    /// expect: "The system provides durable storage for triple data"
    /// \[P3\] Motivating: Generative Space — delete by entity prefix
    /// pre:  prefix is non-empty
    /// post: matching triples deleted
    /// post: returns count of deleted triples
    pub fn delete_by_entity_prefix(&self, prefix: &str) -> Result<usize, TripleError> {
        let conn = self.lock_conn()?;
        let pattern = format!("{}%", prefix);
        let count = conn.execute(
            "DELETE FROM triples WHERE entity LIKE ?1",
            rusqlite::params![pattern],
        )?;
        Ok(count)
    }
    /// Row → TripleRow: FromSql for IDs/WebID/Visibility. Timestamps stay String (orphan rule).
    fn row_to_triple_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TripleRow> {
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
    }
    /// TripleRow → Triple: parse timestamps + JSON value (orphan rule).
    fn row_to_triple(row: TripleRow) -> Result<Triple, TripleError> {
        let value: Value = serde_json::from_str(&row.value)?;
        let valid_from = DateTime::parse_from_rfc3339(&row.valid_from)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| {
                TripleError::Infra(InfrastructureError::Database(format!(
                    "corrupt valid_from timestamp '{}': {}",
                    row.valid_from, e
                )))
            })?;
        let valid_to = row
            .valid_to
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        Ok(Triple {
            id: row.id,
            entity: row.entity,
            attribute: row.attribute,
            value,
            temporal: TemporalBounds::new(valid_from, valid_to),
            confidence: row.confidence,
            access: AccessControl {
                perspective: row.perspective,
                visibility: row.visibility,
                owner_webid: row.owner_webid,
            },
        })
    }
}
/// Triple -> TripleEntry: lossy (flattens access control for CAS storage).
impl From<&Triple> for TripleEntry {
    fn from(t: &Triple) -> Self {
        Self {
            id: t.id.to_string(),
            entity: t.entity.clone(),
            attribute: t.attribute.clone(),
            value: t.value.clone(),
            valid_from: t.temporal.valid_from.to_rfc3339(),
            valid_to: t.temporal.valid_to.map(|dt| dt.to_rfc3339()),
            confidence: t.confidence.value(),
            perspective: t
                .access
                .perspective
                .map(|wid| wid.to_string())
                .unwrap_or_default(),
            visibility: t.access.visibility.as_str().to_string(),
        }
    }
}
struct TripleRow {
    id: TripleID,
    entity: String,
    attribute: String,
    value: String,
    valid_from: String,
    valid_to: Option<String>,
    confidence: Confidence,
    perspective: Option<WebID>,
    visibility: Visibility,
    owner_webid: WebID,
}
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};
    fn make_store() -> TripleStore {
        let conn = Arc::new(Mutex::new(
            Connection::open_in_memory().expect("in-memory DB"),
        ));
        let store = TripleStore::new(conn);
        store
            .lock_conn()
            .unwrap()
            .execute_batch(
                "CREATE TABLE triples (
                    id TEXT PRIMARY KEY, entity TEXT NOT NULL, attribute TEXT NOT NULL,
                    value TEXT NOT NULL, valid_from TEXT NOT NULL, valid_to TEXT,
                    confidence REAL NOT NULL, perspective TEXT, visibility TEXT NOT NULL,
                    owner_webid TEXT NOT NULL
                )",
            )
            .unwrap();
        store
    }
    //
    // Before fix, a corrupt valid_from was silently replaced with Utc::now(),
    // returning a triple with a fabricated temporal validity bound.
    // Now it propagates an Infra error, and collect_rows! logs and skips the row.
    #[test]
    fn corrupt_valid_from_propagates_infra_error() {
        let store = make_store();
        let webid = WebID::new();
        let id = TripleID::new();
        // Insert a triple with a garbage timestamp that cannot be parsed as RFC3339.
        store
            .lock_conn()
            .unwrap()
            .execute(
                "INSERT INTO triples (id, entity, attribute, value, valid_from, valid_to, confidence, perspective, visibility, owner_webid) \
                 VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, NULL, ?7, ?8)",
                rusqlite::params![
                    id,
                    "test-entity",
                    "attr",
                    serde_json::to_string(&serde_json::json!("val")).unwrap(),
                    "not-a-timestamp",
                    1.0f64,
                    "private",
                    webid,
                ],
            )
            .unwrap();
        // Query should return zero triples (row is logged and skipped by collect_rows!).
        let triples = store.query_by_entity("test-entity").unwrap();
        assert!(
            triples.is_empty(),
            "corrupt timestamp row should be skipped, not returned with Utc::now()"
        );
    }
    #[test]
    fn valid_from_round_trips_correctly() {
        let store = make_store();
        let webid = WebID::new();
        let triple = Triple::new("entity", "attr", serde_json::json!("val"), webid);
        store.insert(&triple).unwrap();
        let triples = store.query_by_entity("entity").unwrap();
        assert_eq!(triples.len(), 1);
        // valid_from should match the original to second precision.
        let delta = (triples[0].temporal.valid_from - triple.temporal.valid_from)
            .num_seconds()
            .abs();
        assert!(delta < 2, "valid_from should survive a round-trip");
    }
    #[test]
    fn get_by_id_missing_returns_none() {
        let store = make_store();
        let missing = TripleID::new();
        let result = store.get_by_id(&missing).unwrap();
        assert!(result.is_none());
    }
}
