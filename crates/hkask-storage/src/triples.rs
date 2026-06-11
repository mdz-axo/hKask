//! Uni-temporal triples — entity/attribute/value with valid_from/valid_to.

use crate::{Store, collect_rows, now_rfc3339};
use chrono::{DateTime, Utc};
use hkask_types::id::{TripleID, WebID};
use hkask_types::ports::git_cas::TripleEntry;
use hkask_types::{AccessControl, Confidence, InfrastructureError, TemporalBounds, Visibility};
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

    pub fn with_confidence(mut self, c: impl Into<Confidence>) -> Self {
        self.confidence = c.into();
        self
    }
    pub fn with_perspective(mut self, p: WebID) -> Self {
        self.access = self.access.with_perspective(p);
        self
    }
    pub fn with_visibility(mut self, v: Visibility) -> Self {
        self.access = self.access.with_visibility(v);
        self
    }

    pub fn is_episodic(&self) -> bool {
        self.access.is_episodic()
    }
    pub fn is_semantic(&self) -> bool {
        self.access.is_semantic()
    }
}

define_store!(TripleStore);

const TRIPLE_COLUMNS: &str = "id, entity, attribute, value, valid_from, valid_to, confidence, perspective, visibility, owner_webid";

impl TripleStore {
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

    /// Update a triple's value (close current version, insert new).
    /// Wrapped in a transaction for atomicity.
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

    pub fn get_by_id(&self, id: &TripleID) -> Result<Option<Triple>, TripleError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {TRIPLE_COLUMNS} FROM triples WHERE id = ?1 AND valid_to IS NULL"
        ))?;
        let triples = collect_rows!(
            stmt,
            rusqlite::params![id],
            Self::row_to_triple_row,
            Self::row_to_triple
        );
        Ok(triples.into_iter().next())
    }

    /// Semantic triples with lowest confidence, ordered ASC. Used by consolidation.
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
    pub fn count_by_perspective(&self, perspective: &WebID) -> Result<usize, TripleError> {
        let conn = self.lock_conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM triples WHERE perspective = ?1 AND valid_to IS NULL",
            rusqlite::params![perspective],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Soft-delete: set valid_to to close a triple.
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
    pub fn delete_by_id(&self, id: &TripleID) -> Result<(), TripleError> {
        let conn = self.lock_conn()?;
        conn.execute("DELETE FROM triples WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
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
            .unwrap_or_else(|_| Utc::now());
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

/// Triple → TripleEntry: lossy (flattens access control for CAS storage).
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
