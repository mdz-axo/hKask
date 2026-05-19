//! Bitemporal triples storage

use chrono::{DateTime, Utc};
use hkask_types::{TripleID, Visibility, WebID};
use rusqlite::Connection;
use serde_json::Value;
use std::rc::Rc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TripleError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
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

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn with_perspective(mut self, perspective: WebID) -> Self {
        self.perspective = Some(perspective);
        self
    }

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

/// Triple store for bitemporal RDF-like triples
pub struct TripleStore {
    conn: Rc<Connection>,
}

impl TripleStore {
    pub fn new(conn: Rc<Connection>) -> Self {
        Self { conn }
    }

    /// Insert a triple
    pub fn insert(&self, triple: &Triple) -> Result<(), TripleError> {
        self.conn.execute(
            "INSERT INTO triples (id, entity, attribute, value, valid_from, valid_to, confidence, perspective, visibility, owner_webid)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                triple.id.0.to_string(),
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
    pub fn query_by_entity(&self, _entity: &str) -> Result<Vec<Triple>, TripleError> {
        // Stub - returns empty for now
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_triple_new() {
        let owner = WebID::new();
        let triple = Triple::new("entity1", "attribute1", json!("value1"), owner);
        assert_eq!(triple.entity, "entity1");
        assert_eq!(triple.confidence, 1.0);
        assert!(triple.is_semantic());
    }
}
