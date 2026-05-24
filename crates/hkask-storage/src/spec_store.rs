//! SpecStore — SQLite-backed specification storage and curation

use hkask_types::spec::{Spec, SpecCategory, SpecCurationRecord, SpecError, SpecStore};
use hkask_types::{CompletenessCheck, CurationDecision, OCAPBoundary, SpecCurator, SpecId};
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

pub struct SqliteSpecCurator {
    coherence_threshold: f64,
}

impl SqliteSpecCurator {
    pub fn new(coherence_threshold: f64) -> Self {
        Self {
            coherence_threshold: coherence_threshold.clamp(0.0, 1.0),
        }
    }
}

impl Default for SqliteSpecCurator {
    fn default() -> Self {
        Self::new(0.7)
    }
}

impl SpecCurator for SqliteSpecCurator {
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
        let coherence = specs.as_slice().coherence();
        if coherence < self.coherence_threshold {
            return Err(SpecError::CoherenceInsufficient(coherence));
        }
        Ok(coherence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::{DomainAnchor, GoalSpec};

    fn test_store() -> SqliteSpecStore {
        let conn = Connection::open_in_memory().unwrap();
        let store = SqliteSpecStore::new(Arc::new(Mutex::new(conn)));
        store.init_schema().unwrap();
        store
    }

    #[test]
    fn spec_store_roundtrip() {
        let store = test_store();
        let mut goal = GoalSpec::new("test goal").with_criterion("c1");
        goal.criteria[0].mark_satisfied();
        let spec =
            Spec::new("test spec", SpecCategory::Domain, DomainAnchor::Hkask).with_goal(goal);
        let id = spec.id;

        store.save(&spec).unwrap();
        let loaded = store.load(id).unwrap();
        assert_eq!(loaded.name, "test spec");
        assert_eq!(loaded.category, SpecCategory::Domain);
    }

    #[test]
    fn spec_store_not_found() {
        let store = test_store();
        let result = store.load(SpecId::new());
        assert!(result.is_err());
    }

    #[test]
    fn spec_store_list_by_category() {
        let store = test_store();
        let spec1 = Spec::new("s1", SpecCategory::Domain, DomainAnchor::Hkask);
        let spec2 = Spec::new("s2", SpecCategory::Capability, DomainAnchor::Hkask);
        let spec3 = Spec::new("s3", SpecCategory::Domain, DomainAnchor::Okapi);

        store.save(&spec1).unwrap();
        store.save(&spec2).unwrap();
        store.save(&spec3).unwrap();

        let domain_specs = store.list_by_category(SpecCategory::Domain).unwrap();
        assert_eq!(domain_specs.len(), 2);

        let capability_specs = store.list_by_category(SpecCategory::Capability).unwrap();
        assert_eq!(capability_specs.len(), 1);
    }

    #[test]
    fn spec_curator_evaluate_complete() {
        let curator = SqliteSpecCurator::default();
        let mut goal = GoalSpec::new("g1").with_criterion("c1");
        goal.criteria[0].mark_satisfied();
        let spec = Spec::new("test", SpecCategory::Domain, DomainAnchor::Hkask).with_goal(goal);

        let record = curator.evaluate(&spec).unwrap();
        assert_eq!(record.decision, CurationDecision::Merge);
        assert_eq!(record.coherence_score, 1.0);
    }

    #[test]
    fn spec_curator_evaluate_incomplete() {
        let curator = SqliteSpecCurator::default();
        let goal = GoalSpec::new("g1").with_criterion("c1");
        let spec = Spec::new("test", SpecCategory::Domain, DomainAnchor::Hkask).with_goal(goal);

        let record = curator.evaluate(&spec).unwrap();
        assert_eq!(record.decision, CurationDecision::Revise);
    }

    #[test]
    fn spec_curator_evaluate_empty() {
        let curator = SqliteSpecCurator::default();
        let spec = Spec::new("test", SpecCategory::Domain, DomainAnchor::Hkask);

        let record = curator.evaluate(&spec).unwrap();
        assert_eq!(record.decision, CurationDecision::Discard);
    }

    #[test]
    fn spec_curator_cultivate_above_threshold() {
        let curator = SqliteSpecCurator::new(0.5);
        let mut goal = GoalSpec::new("g1").with_criterion("c1");
        goal.criteria[0].mark_satisfied();
        let spec = Spec::new("test", SpecCategory::Domain, DomainAnchor::Hkask).with_goal(goal);
        let mut specs = vec![spec];

        let result = curator.cultivate(&mut specs);
        assert!(result.is_ok());
    }

    #[test]
    fn spec_curator_cultivate_below_threshold() {
        let curator = SqliteSpecCurator::new(0.9);
        let spec = Spec::new("test", SpecCategory::Domain, DomainAnchor::Hkask);
        let mut specs = vec![spec];

        let result = curator.cultivate(&mut specs);
        assert!(result.is_err());
    }
}
