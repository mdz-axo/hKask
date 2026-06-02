//! Memory port adapters for template context
//!
//! Provides:
//! - `StubMemoryPort`: Returns empty results (for testing)
//! - `MemoryAdapter`: Generic wrapper (for custom types)
//! - `AppMemoryAdapter`: Concrete adapter backed by TripleStore

use crate::ports::{MemoryFragment, Result};
use hkask_storage::TripleStore;
use hkask_types::WebID;

pub(crate) struct StubMemoryPort;

impl StubMemoryPort {
    pub fn query_semantic(&self, _entity: &str) -> Result<Vec<MemoryFragment>> {
        Ok(Vec::new())
    }

    pub fn query_episodic(&self, _entity: &str, _perspective: &str) -> Result<Vec<MemoryFragment>> {
        Ok(Vec::new())
    }

    pub fn get_session_history(
        &self,
        _session_id: &str,
        _max_messages: usize,
    ) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
}

pub(crate) struct MemoryAdapter<S, E> {
    _semantic: S,
    _episodic: E,
}

impl<S, E> MemoryAdapter<S, E> {
    pub fn new(semantic: S, episodic: E) -> Self {
        Self {
            _semantic: semantic,
            _episodic: episodic,
        }
    }
}

/// Concrete memory adapter backed by TripleStore.
///
/// Queries semantic and episodic triples directly from storage.
/// Note: This bypasses hkask-memory domain logic (dedup, Bayesian
/// confidence decay, consolidation bridge). For domain-correct
/// behavior, wire through EpisodicLoop/SemanticLoop when created.
pub(crate) struct AppMemoryAdapter {
    store: TripleStore,
}

impl AppMemoryAdapter {
    pub fn new(store: TripleStore) -> Self {
        Self { store }
    }

    pub fn query_semantic(&self, entity: &str) -> Result<Vec<MemoryFragment>> {
        let triples = self
            .store
            .query_by_entity(entity)
            .map_err(|e| crate::ports::TemplateError::Database(e.to_string()))?;

        Ok(triples
            .into_iter()
            .filter(|t| t.is_semantic())
            .map(|triple| MemoryFragment {
                content: format!("{}: {} = {}", triple.entity, triple.attribute, triple.value),
                source: "semantic".to_string(),
                confidence: triple.confidence,
            })
            .collect())
    }

    pub fn query_episodic(&self, entity: &str, perspective: &str) -> Result<Vec<MemoryFragment>> {
        let webid = WebID::from_string(perspective);
        let triples = self
            .store
            .query_by_entity(entity)
            .map_err(|e| crate::ports::TemplateError::Database(e.to_string()))?;

        Ok(triples
            .into_iter()
            .filter(|t| t.perspective == Some(webid) && t.is_episodic())
            .map(|triple| MemoryFragment {
                content: format!("{}: {} = {}", triple.entity, triple.attribute, triple.value),
                source: "episodic".to_string(),
                confidence: triple.confidence,
            })
            .collect())
    }

    pub fn get_session_history(
        &self,
        _session_id: &str,
        _max_messages: usize,
    ) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
}
