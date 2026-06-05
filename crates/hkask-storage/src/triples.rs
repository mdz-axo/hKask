//! Uni-temporal triples storage
//!
//! Uses `entity/attribute/value` naming (aligned with hKask schema conventions)
//! and `valid_from`/`valid_to` for temporal tracking.

use crate::Store;
use chrono::{DateTime, Utc};
use hkask_types::{InfrastructureError, TripleID, Visibility, WebID};
use serde_json::Value;
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TripleError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),
    #[error("Triple not found")]
    NotFound,
}

impl_from_rusqlite!(TripleError, Infra);

impl From<serde_json::Error> for TripleError {
    fn from(e: serde_json::Error) -> Self {
        InfrastructureError::from(e).into()
    }
}

/// Bitemporal triple
#[derive(Debug, Clone)]
pub struct Triple {
    pub id: TripleID,
    pub entity: String,
    pub attribute: String,
    pub value: Value,
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>,
    pub confidence: f64,
    pub perspective: Option<WebID>,
    pub visibility: Visibility,
    pub owner_webid: WebID,
}

impl Triple {
    pub fn new(entity: &str, attribute: &str, value: Value, owner_webid: WebID) -> Self {
        Self {
            id: TripleID::new(),
            entity: entity.to_string(),
            attribute: attribute.to_string(),
            value,
            valid_from: Utc::now(),
            valid_to: None,
            confidence: 1.0,
            perspective: None,
            visibility: Visibility::Private,
            owner_webid,
        }
    }

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_perspective(mut self, perspective: WebID) -> Self {
        self.perspective = Some(perspective);
        self
    }

    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn is_episodic(&self) -> bool {
        self.perspective.is_some()
    }

    pub fn is_semantic(&self) -> bool {
        self.perspective.is_none()
    }
}

define_store!(TripleStore);

impl TripleStore {
    /// Insert a triple
    pub fn insert(&self, triple: &Triple) -> Result<(), TripleError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO triples (id, entity, attribute, value, valid_from, valid_to, confidence, perspective, visibility, owner_webid)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                triple.id.as_uuid().to_string(),
                triple.entity,
                triple.attribute,
                serde_json::to_string(&triple.value)?,
                triple.valid_from.to_rfc3339(),
                triple.valid_to.map(|t| t.to_rfc3339()),
                triple.confidence,
                triple.perspective.map(|p| p.0.to_string()),
                triple.visibility.as_str(),
                triple.owner_webid.0.to_string(),
            ],
        )?;
        Ok(())
    }

    /// Query triples by entity
    pub fn query_by_entity(&self, entity: &str) -> Result<Vec<Triple>, TripleError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, entity, attribute, value, valid_from, valid_to, confidence, perspective, visibility, owner_webid
             FROM triples
             WHERE entity = ?1 AND valid_to IS NULL
             ORDER BY valid_from DESC",
        )?;

        let mapped: Vec<Result<TripleRow, rusqlite::Error>> = stmt
            .query_map(rusqlite::params![entity], |row| {
                let id_str: String = row.get(0)?;
                let entity: String = row.get(1)?;
                let attribute: String = row.get(2)?;
                let value_str: String = row.get(3)?;
                let valid_from_str: String = row.get(4)?;
                let valid_to_str: Option<String> = row.get(5)?;
                let confidence: f64 = row.get(6)?;
                let perspective_str: Option<String> = row.get(7)?;
                let visibility_str: String = row.get(8)?;
                let owner_webid_str: String = row.get(9)?;

                Ok(TripleRow {
                    id: id_str,
                    entity,
                    attribute,
                    value: value_str,
                    valid_from: valid_from_str,
                    valid_to: valid_to_str,
                    confidence,
                    perspective: perspective_str,
                    visibility: visibility_str,
                    owner_webid: owner_webid_str,
                })
            })?
            .collect();

        let mut triples = Vec::with_capacity(mapped.len());
        for row_result in mapped {
            match row_result {
                Ok(row) => match Self::row_to_triple(row) {
                    Ok(triple) => triples.push(triple),
                    Err(e) => {
                        tracing::warn!(target: "hkask.storage", error = %e, "Skipping malformed triple row")
                    }
                },
                Err(e) => {
                    tracing::warn!(target: "hkask.storage", error = %e, "Skipping unreadable database row")
                }
            }
        }

        Ok(triples)
    }

    /// Query triples by entity and attribute
    pub fn query_by_entity_attribute(
        &self,
        entity: &str,
        attribute: &str,
    ) -> Result<Vec<Triple>, TripleError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, entity, attribute, value, valid_from, valid_to, confidence, perspective, visibility, owner_webid
             FROM triples
             WHERE entity = ?1 AND attribute = ?2 AND valid_to IS NULL
             ORDER BY valid_from DESC",
        )?;

        let mapped: Vec<Result<TripleRow, rusqlite::Error>> = stmt
            .query_map(rusqlite::params![entity, attribute], |row| {
                let id_str: String = row.get(0)?;
                let entity: String = row.get(1)?;
                let attribute: String = row.get(2)?;
                let value_str: String = row.get(3)?;
                let valid_from_str: String = row.get(4)?;
                let valid_to_str: Option<String> = row.get(5)?;
                let confidence: f64 = row.get(6)?;
                let perspective_str: Option<String> = row.get(7)?;
                let visibility_str: String = row.get(8)?;
                let owner_webid_str: String = row.get(9)?;

                Ok(TripleRow {
                    id: id_str,
                    entity,
                    attribute,
                    value: value_str,
                    valid_from: valid_from_str,
                    valid_to: valid_to_str,
                    confidence,
                    perspective: perspective_str,
                    visibility: visibility_str,
                    owner_webid: owner_webid_str,
                })
            })?
            .collect();

        let mut triples = Vec::with_capacity(mapped.len());
        for row_result in mapped {
            match row_result {
                Ok(row) => match Self::row_to_triple(row) {
                    Ok(triple) => triples.push(triple),
                    Err(e) => {
                        tracing::warn!(target: "hkask.storage", error = %e, "Skipping malformed triple row")
                    }
                },
                Err(e) => {
                    tracing::warn!(target: "hkask.storage", error = %e, "Skipping unreadable database row")
                }
            }
        }

        Ok(triples)
    }

    /// Query all triples for a perspective (episodic memories)
    pub fn query_by_perspective(&self, perspective: &WebID) -> Result<Vec<Triple>, TripleError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, entity, attribute, value, valid_from, valid_to, confidence, perspective, visibility, owner_webid
             FROM triples
             WHERE perspective = ?1 AND valid_to IS NULL
             ORDER BY valid_from DESC",
        )?;

        let mapped: Vec<Result<TripleRow, rusqlite::Error>> = stmt
            .query_map(rusqlite::params![perspective.0.to_string()], |row| {
                let id_str: String = row.get(0)?;
                let entity: String = row.get(1)?;
                let attribute: String = row.get(2)?;
                let value_str: String = row.get(3)?;
                let valid_from_str: String = row.get(4)?;
                let valid_to_str: Option<String> = row.get(5)?;
                let confidence: f64 = row.get(6)?;
                let perspective_str: Option<String> = row.get(7)?;
                let visibility_str: String = row.get(8)?;
                let owner_webid_str: String = row.get(9)?;

                Ok(TripleRow {
                    id: id_str,
                    entity,
                    attribute,
                    value: value_str,
                    valid_from: valid_from_str,
                    valid_to: valid_to_str,
                    confidence,
                    perspective: perspective_str,
                    visibility: visibility_str,
                    owner_webid: owner_webid_str,
                })
            })?
            .collect();

        let mut triples = Vec::with_capacity(mapped.len());
        for row_result in mapped {
            match row_result {
                Ok(row) => match Self::row_to_triple(row) {
                    Ok(triple) => triples.push(triple),
                    Err(e) => {
                        tracing::warn!(target: "hkask.storage", error = %e, "Skipping malformed triple row")
                    }
                },
                Err(e) => {
                    tracing::warn!(target: "hkask.storage", error = %e, "Skipping unreadable database row")
                }
            }
        }

        Ok(triples)
    }

    /// Update a triple's value (closes current version, inserts new)
    pub fn update(
        &self,
        id: &TripleID,
        new_value: Value,
        new_confidence: f64,
    ) -> Result<(), TripleError> {
        let conn = self.lock_conn()?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE triples SET valid_to = ?1 WHERE id = ?2 AND valid_to IS NULL",
            rusqlite::params![now, id.as_uuid().to_string()],
        )?;

        let mut stmt = conn.prepare(
            "SELECT entity, attribute, perspective, visibility, owner_webid
             FROM triples WHERE id = ?1",
        )?;

        let row = stmt.query_row(rusqlite::params![id.as_uuid().to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;

        let new_id = TripleID::new();
        conn.execute(
            "INSERT INTO triples (id, entity, attribute, value, valid_from, valid_to, confidence, perspective, visibility, owner_webid)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                new_id.as_uuid().to_string(),
                row.0,
                row.1,
                serde_json::to_string(&new_value)?,
                now,
                new_confidence,
                row.2,
                row.3,
                row.4,
            ],
        )?;

        Ok(())
    }

    /// Get a single triple by ID (must be current: valid_to IS NULL)
    pub fn get_by_id(&self, id: &TripleID) -> Result<Option<Triple>, TripleError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, entity, attribute, value, valid_from, valid_to, confidence, perspective, visibility, owner_webid
             FROM triples
             WHERE id = ?1 AND valid_to IS NULL",
        )?;

        let mapped: Vec<Result<TripleRow, rusqlite::Error>> = stmt
            .query_map(rusqlite::params![id.as_uuid().to_string()], |row| {
                let id_str: String = row.get(0)?;
                let entity: String = row.get(1)?;
                let attribute: String = row.get(2)?;
                let value_str: String = row.get(3)?;
                let valid_from_str: String = row.get(4)?;
                let valid_to_str: Option<String> = row.get(5)?;
                let confidence: f64 = row.get(6)?;
                let perspective_str: Option<String> = row.get(7)?;
                let visibility_str: String = row.get(8)?;
                let owner_webid_str: String = row.get(9)?;

                Ok(TripleRow {
                    id: id_str,
                    entity,
                    attribute,
                    value: value_str,
                    valid_from: valid_from_str,
                    valid_to: valid_to_str,
                    confidence,
                    perspective: perspective_str,
                    visibility: visibility_str,
                    owner_webid: owner_webid_str,
                })
            })?
            .collect();

        let mut result = None;
        for row_result in mapped {
            match row_result {
                Ok(row) => match Self::row_to_triple(row) {
                    Ok(triple) => {
                        result = Some(triple);
                        break;
                    }
                    Err(e) => {
                        tracing::warn!(target: "hkask.storage", error = %e, "Skipping malformed triple row")
                    }
                },
                Err(e) => {
                    tracing::warn!(target: "hkask.storage", error = %e, "Skipping unreadable database row")
                }
            }
        }

        Ok(result)
    }

    /// Query semantic triples (perspective IS NULL) with lowest confidence,
    /// ordered by confidence ascending then valid_from ascending, limited to `limit`.
    ///
    /// Used by `SemanticMemory::lowest_confidence_triples()` for budget enforcement.
    pub fn query_semantic_lowest_confidence(
        &self,
        limit: usize,
    ) -> Result<Vec<Triple>, TripleError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, entity, attribute, value, valid_from, valid_to, confidence, perspective, visibility, owner_webid
             FROM triples
             WHERE perspective IS NULL AND valid_to IS NULL
             ORDER BY confidence ASC, valid_from ASC
             LIMIT ?1",
        )?;

        let mapped: Vec<Result<TripleRow, rusqlite::Error>> = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                let id_str: String = row.get(0)?;
                let entity: String = row.get(1)?;
                let attribute: String = row.get(2)?;
                let value_str: String = row.get(3)?;
                let valid_from_str: String = row.get(4)?;
                let valid_to_str: Option<String> = row.get(5)?;
                let confidence: f64 = row.get(6)?;
                let perspective_str: Option<String> = row.get(7)?;
                let visibility_str: String = row.get(8)?;
                let owner_webid_str: String = row.get(9)?;

                Ok(TripleRow {
                    id: id_str,
                    entity,
                    attribute,
                    value: value_str,
                    valid_from: valid_from_str,
                    valid_to: valid_to_str,
                    confidence,
                    perspective: perspective_str,
                    visibility: visibility_str,
                    owner_webid: owner_webid_str,
                })
            })?
            .collect();

        let mut triples = Vec::with_capacity(mapped.len());
        for row_result in mapped {
            match row_result {
                Ok(row) => match Self::row_to_triple(row) {
                    Ok(triple) => triples.push(triple),
                    Err(e) => {
                        tracing::warn!(target: "hkask.storage", error = %e, "Skipping malformed triple row")
                    }
                },
                Err(e) => {
                    tracing::warn!(target: "hkask.storage", error = %e, "Skipping unreadable database row")
                }
            }
        }

        Ok(triples)
    }

    /// Count semantic triples with confidence at or below a threshold.
    ///
    /// Used by `SemanticMemory::low_confidence_count()` for the consolidation
    /// trigger: triples at or below the threshold are candidates for review
    /// and deletion.
    pub fn count_semantic_below_confidence(&self, threshold: f64) -> Result<usize, TripleError> {
        let conn = self.lock_conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM triples WHERE perspective IS NULL AND valid_to IS NULL AND confidence <= ?1",
            rusqlite::params![threshold],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Query semantic triples with confidence at or below a threshold,
    /// ordered by confidence ascending then valid_from ascending, limited to `limit`.
    ///
    /// Used by `SemanticMemory::low_confidence_triples()` for the consolidation
    /// trigger.
    pub fn query_semantic_below_confidence(
        &self,
        threshold: f64,
        limit: usize,
    ) -> Result<Vec<Triple>, TripleError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, entity, attribute, value, valid_from, valid_to, confidence, perspective, visibility, owner_webid
             FROM triples
             WHERE perspective IS NULL AND valid_to IS NULL AND confidence <= ?1
             ORDER BY confidence ASC, valid_from ASC
             LIMIT ?2",
        )?;

        let mapped: Vec<Result<TripleRow, rusqlite::Error>> = stmt
            .query_map(rusqlite::params![threshold, limit as i64], |row| {
                let id_str: String = row.get(0)?;
                let entity: String = row.get(1)?;
                let attribute: String = row.get(2)?;
                let value_str: String = row.get(3)?;
                let valid_from_str: String = row.get(4)?;
                let valid_to_str: Option<String> = row.get(5)?;
                let confidence: f64 = row.get(6)?;
                let perspective_str: Option<String> = row.get(7)?;
                let visibility_str: String = row.get(8)?;
                let owner_webid_str: String = row.get(9)?;

                Ok(TripleRow {
                    id: id_str,
                    entity,
                    attribute,
                    value: value_str,
                    valid_from: valid_from_str,
                    valid_to: valid_to_str,
                    confidence,
                    perspective: perspective_str,
                    visibility: visibility_str,
                    owner_webid: owner_webid_str,
                })
            })?
            .collect();

        let mut triples = Vec::with_capacity(mapped.len());
        for row_result in mapped {
            match row_result {
                Ok(row) => match Self::row_to_triple(row) {
                    Ok(triple) => triples.push(triple),
                    Err(e) => {
                        tracing::warn!(target: "hkask.storage", error = %e, "Skipping malformed triple row")
                    }
                },
                Err(e) => {
                    tracing::warn!(target: "hkask.storage", error = %e, "Skipping unreadable database row")
                }
            }
        }

        Ok(triples)
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

    /// Count semantic triples for a given entity (perspective IS NULL, valid_to IS NULL).
    ///
    /// Used by `SemanticMemory::triple_count_for_entity()` to count only
    /// shared/semantic triples, excluding episodic (perspective IS NOT NULL).
    pub fn count_semantic_by_entity(&self, entity: &str) -> Result<usize, TripleError> {
        let conn = self.lock_conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM triples WHERE entity = ?1 AND perspective IS NULL AND valid_to IS NULL",
            rusqlite::params![entity],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Count triples for a given perspective (episodic, valid_to IS NULL).
    ///
    /// Used by `EpisodicMemory::storage_usage()` for budget enforcement
    /// without loading all triples into memory.
    pub fn count_by_perspective(&self, perspective: &WebID) -> Result<usize, TripleError> {
        let conn = self.lock_conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM triples WHERE perspective = ?1 AND valid_to IS NULL",
            rusqlite::params![perspective.0.to_string()],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Close a triple by setting its `valid_to` timestamp (soft-delete).
    ///
    /// Used by consolidation to mark episodic triples as expired after
    /// they have been promoted to semantic memory. The triple remains in
    /// the store for audit but is excluded from all current queries
    /// (which filter on `valid_to IS NULL`).
    pub fn close_by_id(&self, id: &TripleID) -> Result<(), TripleError> {
        let conn = self.lock_conn()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE triples SET valid_to = ?1 WHERE id = ?2 AND valid_to IS NULL",
            rusqlite::params![now, id.as_uuid().to_string()],
        )?;
        Ok(())
    }

    /// Delete a triple by ID.
    ///
    /// Used by `SemanticMemory::delete_triple()` for budget enforcement.
    /// Unlike update (which sets `valid_to`), this removes the row entirely.
    pub fn delete_by_id(&self, id: &TripleID) -> Result<(), TripleError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "DELETE FROM triples WHERE id = ?1",
            rusqlite::params![id.as_uuid().to_string()],
        )?;
        Ok(())
    }

    fn row_to_triple(row: TripleRow) -> Result<Triple, TripleError> {
        let id = TripleID::from_str(&row.id)
            .map_err(|e| InfrastructureError::Database(format!("unparseable triple ID: {e}")))?;
        let value: Value = serde_json::from_str(&row.value)?;
        let valid_from = DateTime::parse_from_rfc3339(&row.valid_from)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let valid_to = row
            .valid_to
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let perspective = row.perspective.and_then(|s| s.parse().ok());
        let visibility = Visibility::parse_str(&row.visibility).unwrap_or_default();
        let owner_webid = WebID::from_str(&row.owner_webid)
            .map_err(|e| InfrastructureError::Database(format!("unparseable owner WebID: {e}")))?;

        Ok(Triple {
            id,
            entity: row.entity,
            attribute: row.attribute,
            value,
            valid_from,
            valid_to,
            confidence: row.confidence,
            perspective,
            visibility,
            owner_webid,
        })
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
