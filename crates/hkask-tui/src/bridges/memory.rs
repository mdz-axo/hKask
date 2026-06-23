//! MemoryDataBridge — trait for episodic/semantic memory data in the TUI.
//!
//! Provides the Memory window with live data from the episodic and
//! semantic memory stores. Implemented by the CLI via AgentService memory ports.

use std::sync::Arc;

/// A single memory triple for TUI display.
#[derive(Debug, Clone)]
pub struct MemoryTriple {
    pub entity: String,
    pub attribute: String,
    pub value: String,
}

/// Consolidation status snapshot.
#[derive(Debug, Clone)]
pub struct ConsolidationStatus {
    /// Number of episodic triples eligible for consolidation
    pub candidate_count: usize,
    /// Total semantic triple count
    pub semantic_count: usize,
    /// Semantic triples below confidence floor (default 0.33)
    pub low_confidence_count: usize,
    /// Budget: max allowed episodic triples (default 10,000)
    pub episodic_budget: usize,
}

/// Snapshot of all memory counts for the TUI display.
#[derive(Debug, Clone)]
pub struct MemorySummary {
    pub episodic_count: usize,
    pub episodic_budget: usize,
    pub semantic_count: usize,
    pub semantic_low_confidence: usize,
    pub consolidation_candidates: usize,
}

/// Trait for querying memory subsystem state.
pub trait MemoryDataBridge: Send + Sync {
    /// Overall memory counts snapshot.
    fn memory_summary(&self) -> MemorySummary;

    /// Recent episodic memory triples (newest first).
    fn recent_episodic(&self, limit: usize) -> Vec<MemoryTriple>;

    /// Recent semantic memory triples.
    fn recent_semantic(&self, limit: usize) -> Vec<MemoryTriple>;

    /// Consolidation subsystem status.
    fn consolidation_status(&self) -> ConsolidationStatus;
}

/// Mock implementation for TUI development and testing.
pub struct MockMemoryBridge {
    pub summary: MemorySummary,
    pub episodic_triples: Vec<MemoryTriple>,
    pub semantic_triples: Vec<MemoryTriple>,
    pub consolidation: ConsolidationStatus,
}

impl MockMemoryBridge {
    pub fn new() -> Self {
        Self {
            summary: MemorySummary {
                episodic_count: 0,
                episodic_budget: 10_000,
                semantic_count: 0,
                semantic_low_confidence: 0,
                consolidation_candidates: 0,
            },
            episodic_triples: Vec::new(),
            semantic_triples: Vec::new(),
            consolidation: ConsolidationStatus {
                candidate_count: 0,
                semantic_count: 0,
                low_confidence_count: 0,
                episodic_budget: 10_000,
            },
        }
    }

    pub fn with_data() -> Self {
        Self {
            summary: MemorySummary {
                episodic_count: 42,
                episodic_budget: 10_000,
                semantic_count: 156,
                semantic_low_confidence: 3,
                consolidation_candidates: 7,
            },
            episodic_triples: vec![
                MemoryTriple {
                    entity: "session_001".into(),
                    attribute: "tool:read_file".into(),
                    value: "src/main.rs".into(),
                },
                MemoryTriple {
                    entity: "session_001".into(),
                    attribute: "tool:bash".into(),
                    value: "cargo build".into(),
                },
                MemoryTriple {
                    entity: "session_001".into(),
                    attribute: "outcome".into(),
                    value: "success".into(),
                },
            ],
            semantic_triples: vec![
                MemoryTriple {
                    entity: "src/main.rs".into(),
                    attribute: "last_modified".into(),
                    value: "2026-06-20".into(),
                },
                MemoryTriple {
                    entity: "src/main.rs".into(),
                    attribute: "contains_module".into(),
                    value: "cli".into(),
                },
            ],
            consolidation: ConsolidationStatus {
                candidate_count: 7,
                semantic_count: 156,
                low_confidence_count: 3,
                episodic_budget: 10_000,
            },
        }
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl MemoryDataBridge for MockMemoryBridge {
    fn memory_summary(&self) -> MemorySummary {
        self.summary.clone()
    }
    fn recent_episodic(&self, limit: usize) -> Vec<MemoryTriple> {
        self.episodic_triples.iter().take(limit).cloned().collect()
    }
    fn recent_semantic(&self, limit: usize) -> Vec<MemoryTriple> {
        self.semantic_triples.iter().take(limit).cloned().collect()
    }
    fn consolidation_status(&self) -> ConsolidationStatus {
        self.consolidation.clone()
    }
}
