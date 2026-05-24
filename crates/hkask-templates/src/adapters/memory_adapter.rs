//! Memory port adapters for connecting hkask-memory to hkask-templates
//!
//! Provides:
//! - `StubMemoryPort`: Returns empty results (for testing)
//! - `MemoryAdapter`: Connects to real SemanticMemory + EpisodicMemory

use crate::ports::{MemoryFragment, MemoryPort};

/// Stub memory port that returns empty results.
///
/// Use this for testing or when no memory backend is available.
pub struct StubMemoryPort;

impl MemoryPort for StubMemoryPort {
    fn query_semantic(&self, _entity: &str) -> Vec<MemoryFragment> {
        Vec::new()
    }

    fn query_episodic(&self, _entity: &str, _perspective: &str) -> Vec<MemoryFragment> {
        Vec::new()
    }

    fn get_session_history(&self, _session_id: &str, _max_messages: usize) -> Vec<String> {
        Vec::new()
    }
}

/// Memory adapter that connects to real SemanticMemory + EpisodicMemory.
///
/// This adapter converts hkask-storage Triple types into MemoryFragment types
/// expected by the template system.
///
/// # Example
///
/// ```ignore
/// use hkask_templates::adapters::memory_adapter::MemoryAdapter;
/// use hkask_memory::{SemanticMemory, EpisodicMemory};
///
/// let semantic = SemanticMemory::new(triple_store.clone(), embedding_store);
/// let episodic = EpisodicMemory::new(triple_store);
/// let adapter = MemoryAdapter::new(semantic, episodic);
///
/// let executor = ManifestExecutorImpl::new(renderer, inference, mcp, cns)
///     .with_memory(Box::new(adapter));
/// ```
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

// Note: The actual MemoryPort implementation requires hkask-memory as a dependency,
// which would create a circular dependency. Instead, users should implement MemoryPort
// for their specific memory types in their application code, or use the StubMemoryPort
// for testing.
//
// Example implementation in application code:
//
// ```ignore
// use hkask_templates::ports::{MemoryFragment, MemoryPort};
// use hkask_memory::{SemanticMemory, EpisodicMemory};
//
// struct AppMemoryAdapter {
//     semantic: SemanticMemory,
//     episodic: EpisodicMemory,
// }
//
// impl MemoryPort for AppMemoryAdapter {
//     fn query_semantic(&self, entity: &str) -> Vec<MemoryFragment> {
//         self.semantic.query_deduped(entity)
//             .unwrap_or_default()
//             .into_iter()
//             .map(|triple| MemoryFragment {
//                 content: format!("{}: {} = {}", triple.entity, triple.attribute, triple.value),
//                 source: "semantic".to_string(),
//                 confidence: triple.confidence,
//             })
//             .collect()
//     }
//
//     fn query_episodic(&self, entity: &str, perspective: &str) -> Vec<MemoryFragment> {
//         // Similar implementation for episodic memory
//         Vec::new()
//     }
//
//     fn get_session_history(&self, session_id: &str, max_messages: usize) -> Vec<String> {
//         // Query session storage
//         Vec::new()
//     }
// }
// ```

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_memory_port_returns_empty() {
        let stub = StubMemoryPort;

        assert!(stub.query_semantic("test").is_empty());
        assert!(stub.query_episodic("test", "user1").is_empty());
        assert!(stub.get_session_history("session1", 10).is_empty());
    }

    #[test]
    fn test_stub_memory_port_can_be_boxed() {
        let stub: Box<dyn MemoryPort> = Box::new(StubMemoryPort);

        assert!(stub.query_semantic("test").is_empty());
    }
}
