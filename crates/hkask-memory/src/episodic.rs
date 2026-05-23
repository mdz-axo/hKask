//! Episodic memory pipeline — personal experience

use crate::recall_dedup::{self, DedupResult};
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

    /// Query by entity for specific perspective with deduplication.
    ///
    /// Filters duplicate triples at recall time using the EAV hash strategy.
    pub fn query_for_deduped(
        &self,
        entity: &str,
        perspective: WebID,
    ) -> Result<Vec<Triple>, EpisodicMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        let filtered: Vec<Triple> = triples
            .into_iter()
            .filter(|t| t.perspective == Some(perspective))
            .collect();
        Ok(recall_dedup::dedup_triples(filtered))
    }

    /// Query all episodic memories by entity
    pub fn query(&self, entity: &str) -> Result<Vec<Triple>, EpisodicMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        Ok(triples.into_iter().filter(|t| t.is_episodic()).collect())
    }

    /// Query all episodic memories by entity with deduplication.
    pub fn query_deduped(&self, entity: &str) -> Result<Vec<Triple>, EpisodicMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        let episodic: Vec<Triple> = triples.into_iter().filter(|t| t.is_episodic()).collect();
        Ok(recall_dedup::dedup_triples(episodic))
    }

    /// Query all episodic memories by entity with deduplication and statistics.
    pub fn query_deduped_with_stats(
        &self,
        entity: &str,
    ) -> Result<DedupResult, EpisodicMemoryError> {
        let triples = self.triple_store.query_by_entity(entity)?;
        let episodic: Vec<Triple> = triples.into_iter().filter(|t| t.is_episodic()).collect();
        Ok(recall_dedup::dedup_triples_with_stats(episodic))
    }
}
