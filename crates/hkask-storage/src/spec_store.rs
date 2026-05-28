//! SpecStore — SQLite-backed specification storage and curation

use hkask_types::spec::{
    Spec, SpecCategory, SpecCurationRecord, SpecError, SpecObserver, SpecStore,
};
use hkask_types::{
    CollectionCoherence, CompletenessCheck, CurationDecision, OCAPBoundary, SpecCurator, SpecId,
};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct SqliteSpecStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteSpecStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn init_schema(&self) -> Result<(), SpecError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS specs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                category TEXT NOT NULL,
                domain_anchor TEXT NOT NULL,
                signed_by TEXT,
                created_at TEXT NOT NULL,
                data TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| SpecError::Storage(e.to_string()))?;
        Ok(())
    }
}

impl SpecStore for SqliteSpecStore {
    fn load(&self, id: SpecId) -> Result<Spec, SpecError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT data FROM specs WHERE id = ?1")
            .map_err(|e| SpecError::Storage(e.to_string()))?;
        let data: String = stmt
            .query_row(rusqlite::params![id.to_string()], |row| row.get(0))
            .map_err(|_| SpecError::NotFound(id))?;
        serde_json::from_str(&data).map_err(|e| SpecError::Storage(e.to_string()))
    }

    fn save(&self, spec: &Spec) -> Result<(), SpecError> {
        let conn = self.conn.lock().unwrap();
        let data = serde_json::to_string(spec).map_err(|e| SpecError::Storage(e.to_string()))?;
        let signed_by = spec.signed_by.map(|w| w.to_string());
        conn.execute(
            "INSERT OR REPLACE INTO specs (id, name, category, domain_anchor, signed_by, created_at, data)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                spec.id.to_string(),
                spec.name,
                spec.category.as_str(),
                spec.domain_anchor.as_str(),
                signed_by,
                spec.created_at.to_rfc3339(),
                data,
            ],
        )
        .map_err(|e| SpecError::Storage(e.to_string()))?;
        Ok(())
    }

    fn delete(&self, id: SpecId) -> Result<(), SpecError> {
        let conn = self.conn.lock().unwrap();
        let changed = conn
            .execute(
                "DELETE FROM specs WHERE id = ?1",
                rusqlite::params![id.to_string()],
            )
            .map_err(|e| SpecError::Storage(e.to_string()))?;
        if changed == 0 {
            return Err(SpecError::NotFound(id));
        }
        Ok(())
    }

    fn list_all(&self) -> Result<Vec<Spec>, SpecError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT data FROM specs")
            .map_err(|e| SpecError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                let data: String = row.get(0)?;
                Ok(data)
            })
            .map_err(|e| SpecError::Storage(e.to_string()))?;
        let mut specs = Vec::new();
        for row in rows {
            let data = row.map_err(|e| SpecError::Storage(e.to_string()))?;
            let spec: Spec =
                serde_json::from_str(&data).map_err(|e| SpecError::Storage(e.to_string()))?;
            specs.push(spec);
        }
        Ok(specs)
    }

    fn list_by_category(&self, cat: SpecCategory) -> Result<Vec<Spec>, SpecError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT data FROM specs WHERE category = ?1")
            .map_err(|e| SpecError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![cat.as_str()], |row| {
                let data: String = row.get(0)?;
                Ok(data)
            })
            .map_err(|e| SpecError::Storage(e.to_string()))?;
        let mut specs = Vec::new();
        for row in rows {
            let data = row.map_err(|e| SpecError::Storage(e.to_string()))?;
            let spec: Spec =
                serde_json::from_str(&data).map_err(|e| SpecError::Storage(e.to_string()))?;
            specs.push(spec);
        }
        Ok(specs)
    }
}

pub struct DefaultSpecCurator {
    coherence_threshold: f64,
}

impl DefaultSpecCurator {
    pub fn new(coherence_threshold: f64) -> Self {
        Self {
            coherence_threshold: coherence_threshold.clamp(0.0, 1.0),
        }
    }
}

impl Default for DefaultSpecCurator {
    fn default() -> Self {
        Self::new(0.7)
    }
}

impl SpecCurator for DefaultSpecCurator {
    fn evaluate(&self, spec: &Spec) -> Result<SpecCurationRecord, SpecError> {
        let complete = spec.is_complete();
        let decision = if complete {
            CurationDecision::Merge
        } else if spec.goals.is_empty() {
            CurationDecision::Discard
        } else {
            CurationDecision::Revise
        };

        let rationale = if complete {
            "All criteria satisfied".to_string()
        } else if spec.goals.is_empty() {
            "No goals defined".to_string()
        } else {
            "Unsatisfied criteria remain".to_string()
        };

        let coherence = spec.coherence();
        let ocap_boundary = OCAPBoundary::explicit("spec:curate".to_string());

        Ok(SpecCurationRecord::new(
            spec.id,
            decision,
            &rationale,
            coherence,
            ocap_boundary,
        ))
    }

    fn reconcile(&self, specs: &[Spec]) -> Result<Vec<SpecCurationRecord>, SpecError> {
        specs.iter().map(|s| self.evaluate(s)).collect()
    }

    fn cultivate(&self, specs: &mut Vec<Spec>) -> Result<f64, SpecError> {
        let coherence = specs.as_slice().collection_coherence();
        if coherence < self.coherence_threshold {
            return Err(SpecError::CoherenceInsufficient(coherence));
        }
        Ok(coherence)
    }
}

pub struct CnsSpecObserver;

impl SpecObserver for CnsSpecObserver {
    fn emit_span(&self, spec_id: SpecId, operation: &str, outcome: &serde_json::Value) {
        tracing::info!(
            target: "cns.spec",
            spec_id = %spec_id,
            operation,
            outcome = %outcome,
            "spec operation"
        );
    }
}
