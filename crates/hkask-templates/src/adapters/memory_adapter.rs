//! Memory port adapters for connecting hkask-memory to hkask-templates
//!
//! Provides:
//! - `StubMemoryPort`: Returns empty results (for testing)
//! - `MemoryAdapter`: Generic wrapper (for custom types)
//! - `AppMemoryAdapter`: Concrete adapter for SemanticMemory + EpisodicMemory

use crate::ports::{MemoryFragment, MemoryPort, Result};
use hkask_memory::{EpisodicMemory, SemanticMemory};
use hkask_types::WebID;

pub struct StubMemoryPort;

impl MemoryPort for StubMemoryPort {
    fn query_semantic(&self, _entity: &str) -> Result<Vec<MemoryFragment>> {
        Ok(Vec::new())
    }

    fn query_episodic(&self, _entity: &str, _perspective: &str) -> Result<Vec<MemoryFragment>> {
        Ok(Vec::new())
    }

    fn get_session_history(&self, _session_id: &str, _max_messages: usize) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
}

pub struct MemoryAdapter<S, E> {
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

pub struct AppMemoryAdapter {
    semantic: SemanticMemory,
    episodic: EpisodicMemory,
}

impl AppMemoryAdapter {
    pub fn new(semantic: SemanticMemory, episodic: EpisodicMemory) -> Self {
        Self { semantic, episodic }
    }
}

impl MemoryPort for AppMemoryAdapter {
    fn query_semantic(&self, entity: &str) -> Result<Vec<MemoryFragment>> {
        Ok(self
            .semantic
            .query_deduped(entity)
            .unwrap_or_default()
            .into_iter()
            .map(|triple| MemoryFragment {
                content: format!("{}: {} = {}", triple.entity, triple.attribute, triple.value),
                source: "semantic".to_string(),
                confidence: triple.confidence,
            })
            .collect())
    }

    fn query_episodic(&self, entity: &str, perspective: &str) -> Result<Vec<MemoryFragment>> {
        let webid = WebID::from_string(perspective);
        Ok(self
            .episodic
            .query_for_deduped(entity, webid)
            .unwrap_or_default()
            .into_iter()
            .map(|triple| MemoryFragment {
                content: format!("{}: {} = {}", triple.entity, triple.attribute, triple.value),
                source: "episodic".to_string(),
                confidence: triple.confidence,
            })
            .collect())
    }

    fn get_session_history(&self, _session_id: &str, _max_messages: usize) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_memory_port_returns_empty() {
        let stub = StubMemoryPort;

        assert!(stub.query_semantic("test").unwrap().is_empty());
        assert!(stub.query_episodic("test", "user1").unwrap().is_empty());
        assert!(stub.get_session_history("session1", 10).unwrap().is_empty());
    }

    #[test]
    fn test_stub_memory_port_can_be_boxed() {
        let stub: Box<dyn MemoryPort> = Box::new(StubMemoryPort);

        assert!(stub.query_semantic("test").unwrap().is_empty());
    }
}
