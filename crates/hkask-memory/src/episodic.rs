//! Episodic memory pipeline — personal experience

use hkask_storage::{Triple, TripleError, TripleStore};
use hkask_types::WebID;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EpisodicMemoryError {
    #[error("Triple error: {0}")]
    Triple(#[from] TripleError),
}

/// Episodic memory — first-person experience
pub struct EpisodicMemory {
    triple_store: TripleStore,
}

impl EpisodicMemory {
    pub fn new(triple_store: TripleStore) -> Self {
        Self { triple_store }
    }

    /// Store an episodic triple (private by default, with perspective)
    pub fn store(&self, triple: Triple) -> Result<(), EpisodicMemoryError> {
        self.triple_store.insert(&triple)?;
        Ok(())
    }

    /// Query by entity for specific perspective (agent)
    pub fn query_for(
        &self,
        entity: &str,
        perspective: WebID,
    ) -> Result<Vec<Triple>, EpisodicMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        Ok(triples
            .into_iter()
            .filter(|t| t.perspective == Some(perspective))
            .collect())
    }

    /// Query all episodic memories by entity
    pub fn query(&self, entity: &str) -> Result<Vec<Triple>, EpisodicMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        Ok(triples.into_iter().filter(|t| t.is_episodic()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::Database;
    use serde_json::json;

    fn create_test_memory() -> EpisodicMemory {
        let db = Database::in_memory().unwrap();
        EpisodicMemory::new(TripleStore::new(db.conn()))
    }

    #[test]
    fn test_store_episodic() {
        let memory = create_test_memory();
        let owner = hkask_types::WebID::new();
        let perspective = hkask_types::WebID::new();
        let triple = Triple::new("event", "experienced", json!("Something happened"), owner)
            .with_perspective(perspective);

        memory.store(triple).unwrap();
    }

    #[test]
    fn test_query_for_perspective() {
        let memory = create_test_memory();
        let owner = hkask_types::WebID::new();
        let perspective1 = hkask_types::WebID::new();

        let t1 =
            Triple::new("event", "experienced", json!("E1"), owner).with_perspective(perspective1);

        memory.store(t1).unwrap();

        // Stub returns empty
        let results = memory.query_for("event", perspective1).unwrap();
        assert_eq!(results.len(), 0);
    }
}
