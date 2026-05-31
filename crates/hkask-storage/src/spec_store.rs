//! SpecStore — SQLite-backed specification storage and curation

use hkask_types::spec::{
    Spec, SpecCategory, SpecCurationRecord, SpecError, SpecObserver, SpecStore,
};
use hkask_types::{CurationDecision, OCAPBoundary, SYSTEM_MAX_RECURSION, SpecCurator, SpecId};
use rusqlite::Connection;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

pub struct SqliteSpecStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteSpecStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn init_schema(&self) -> Result<(), SpecError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SpecError::Storage(format!("Lock poisoned: {}", e)))?;
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
        let conn = self
            .conn
            .lock()
            .map_err(|e| SpecError::Storage(format!("Lock poisoned: {}", e)))?;
        let mut stmt = conn
            .prepare("SELECT data FROM specs WHERE id = ?1")
            .map_err(|e| SpecError::Storage(e.to_string()))?;
        let data: String = stmt
            .query_row(rusqlite::params![id.to_string()], |row| row.get(0))
            .map_err(|_| SpecError::NotFound(id))?;
        serde_json::from_str(&data).map_err(|e| SpecError::Storage(e.to_string()))
    }

    fn save(&self, spec: &Spec) -> Result<(), SpecError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SpecError::Storage(format!("Lock poisoned: {}", e)))?;
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
        let conn = self
            .conn
            .lock()
            .map_err(|e| SpecError::Storage(format!("Lock poisoned: {}", e)))?;
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
        let conn = self
            .conn
            .lock()
            .map_err(|e| SpecError::Storage(format!("Lock poisoned: {}", e)))?;
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
        let conn = self
            .conn
            .lock()
            .map_err(|e| SpecError::Storage(format!("Lock poisoned: {}", e)))?;
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
    max_iterations: u8,
}

impl DefaultSpecCurator {
    pub fn new(coherence_threshold: f64) -> Self {
        Self {
            coherence_threshold: coherence_threshold.clamp(0.0, 1.0),
            max_iterations: SYSTEM_MAX_RECURSION,
        }
    }

    pub fn with_max_iterations(mut self, max: u8) -> Self {
        self.max_iterations = max;
        self
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

        tracing::debug!(
            target: "cns.spec.evaluate",
            spec_id = %spec.id,
            decision = %decision,
            coherence,
            "spec evaluation"
        );

        Ok(SpecCurationRecord::new(
            spec.id,
            decision,
            &rationale,
            coherence,
            ocap_boundary,
        ))
    }

    fn reconcile(&self, specs: &[Spec]) -> Result<Vec<SpecCurationRecord>, SpecError> {
        let records: Result<Vec<SpecCurationRecord>, SpecError> =
            specs.iter().map(|s| self.evaluate(s)).collect();

        if let Ok(ref recs) = records {
            for (spec, record) in specs.iter().zip(recs.iter()) {
                if record.coherence_score < self.coherence_threshold {
                    let drift_magnitude = 1.0 - record.coherence_score;
                    tracing::debug!(
                        target: "cns.spec.drift",
                        domain = spec.category.as_str(),
                        drift_magnitude,
                        coherence = record.coherence_score,
                        "Drift detected during reconciliation: coherence {:.3} below threshold {:.3}",
                        record.coherence_score, self.coherence_threshold
                    );
                }
            }
        }

        records
    }

    fn cultivate(&self, specs: &mut Vec<Spec>) -> Result<f64, SpecError> {
        let mut iterations_attempted: u8 = 0;
        for _ in 0..self.max_iterations {
            iterations_attempted += 1;
            let coherence = Spec::collection_coherence(specs);
            if coherence >= self.coherence_threshold {
                return Ok(coherence);
            }

            let records = self.reconcile(specs)?;

            // Remove specs marked for discard
            let discard_ids: HashSet<_> = records
                .iter()
                .filter(|r| r.decision == CurationDecision::Discard)
                .map(|r| r.spec_id)
                .collect();
            specs.retain(|s| !discard_ids.contains(&s.id));

            // If all remaining records are Merge, check coherence again
            let all_merge = records
                .iter()
                .filter(|r| r.decision != CurationDecision::Discard)
                .all(|r| r.decision == CurationDecision::Merge);
            if all_merge {
                let coherence = Spec::collection_coherence(specs);
                if coherence >= self.coherence_threshold {
                    return Ok(coherence);
                }
            }
        }

        // Coherence still below threshold after all iterations
        let final_coherence = Spec::collection_coherence(specs);
        tracing::debug!(
            target: "cns.spec.drift",
            final_coherence,
            iterations_attempted,
            "Cultivation failed: coherence {:.3} below threshold {:.3} after {} iterations",
            final_coherence, self.coherence_threshold, iterations_attempted
        );

        Err(SpecError::CurationDepthExceeded)
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
