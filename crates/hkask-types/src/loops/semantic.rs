//! Loop 2b: Semantic Memory — shared, public knowledge
//!
//! knowledge → store (public) → index → recall → dedup → combine → context
//!
//! Essential subloops:
//! - 2b.1 Semantic Deduplication (FILTER) — remove duplicate knowledge entries
//! - 2b.2 Confidence Combination (RECONCILE) — combine confidence from multiple sources
//! - 2b.3 Semantic Indexing (CACHE) — embed and index for similarity search
//!
//! Cybernetics regulation: storage budget adjustment, indexing throttle
//!
//! Semantic memory is SHARED across agents. Any agent with a
//! SemanticReadHandle can query. Only SemanticWriteHandle can store.

use crate::id::WebID;
use crate::sovereignty::DataCategory;

// =============================================================================
// SemanticReadHandle — Loop 2b read access
// =============================================================================

/// Semantic memory read handle. Can query shared semantic triples.
pub struct SemanticReadHandle {
    reader: WebID,
    query_budget: u32,
}

impl SemanticReadHandle {
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            reader: WebID::new(),
            query_budget: 100,
        }
    }

    pub fn new(reader: WebID, query_budget: u32) -> Self {
        Self {
            reader,
            query_budget,
        }
    }

    pub fn reader(&self) -> &WebID {
        &self.reader
    }

    pub fn query_budget(&self) -> u32 {
        self.query_budget
    }

    pub fn can_access(&self, category: &DataCategory) -> bool {
        matches!(
            category,
            DataCategory::SemanticMemory
                | DataCategory::HLexiconTerms
                | DataCategory::TemplateRegistry
        )
    }
}

// =============================================================================
// SemanticWriteHandle — Loop 2b write access
// =============================================================================

/// Semantic memory write handle. Can store and consolidate semantic triples.
pub struct SemanticWriteHandle {
    writer: WebID,
    can_consolidate: bool,
    storage_budget: u32,
}

impl SemanticWriteHandle {
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            writer: WebID::new(),
            can_consolidate: true,
            storage_budget: 10000,
        }
    }

    pub fn new(writer: WebID, can_consolidate: bool, storage_budget: u32) -> Self {
        Self {
            writer,
            can_consolidate,
            storage_budget,
        }
    }

    pub fn writer(&self) -> &WebID {
        &self.writer
    }

    pub fn can_consolidate(&self) -> bool {
        self.can_consolidate
    }

    pub fn storage_budget(&self) -> u32 {
        self.storage_budget
    }

    pub fn can_write(&self, category: &DataCategory) -> bool {
        matches!(category, DataCategory::SemanticMemory)
    }
}
