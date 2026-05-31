//! Loop 2b: Semantic Memory — Capability handles
//!
//! The Semantic Memory loop governs shared, public knowledge:
//! knowledge → store (public) → index → recall → dedup → combine → context
//!
//! Essential subloops:
//! - 2b.1 Semantic Deduplication (FILTER) — remove duplicate knowledge entries
//! - 2b.2 Confidence Combination (RECONCILE) — combine confidence from multiple sources
//! - 2b.3 Semantic Indexing (CACHE) — embed and index for similarity search
//!
//! Governance (via SemanticRegulation from Cybernetics):
//! - Storage budget adjustment — Cybernetics governs storage limits
//! - Indexing throttle — Cybernetics throttles embedding costs
//!
//! Composed methods (not separate subloops):
//! - recall_combined() — composes Dedup + Confidence Combination
//! - check_budget() — budget enforcement (governance via SemanticRegulation)
//!
//! # Capability Discipline
//!
//! Semantic memory is SHARED across agents. Any agent with a `SemanticReadHandle`
//! can query semantic triples. Only agents with `SemanticWriteHandle` can store
//! new semantic triples (including consolidation from episodic memory).
//!
//! - `SemanticReadHandle` can query semantic triples by entity, query by similarity,
//!   and assemble semantic context. It CANNOT store triples or delete triples.
//!
//! - `SemanticWriteHandle` can store semantic triples (with consolidation capability)
//!   and store embeddings. It CANNOT delete triples, access episodic memories,
//!   or write on behalf of other agents.

use crate::id::WebID;
use crate::sovereignty::DataCategory;

// =============================================================================
// SemanticReadHandle — Loop 2b read access
// =============================================================================

/// Semantic memory read handle.
///
/// Provides read access to the shared semantic memory. Any agent with
/// a semantic read handle can query triples by entity or by similarity.
///
/// # OCAP Boundaries
///
/// - **CAN** query semantic triples by entity
/// - **CAN** query semantic triples by similarity (embedding search)
/// - **CAN** assemble semantic context (deduplicated, confidence-combined)
/// - **CANNOT** store triples (use `SemanticWriteHandle`)
/// - **CANNOT** delete triples (use `CyberneticsHandle` with explicit revocation)
/// - **CANNOT** access episodic memories (use `EpisodicReadHandle`)
pub struct SemanticReadHandle {
    /// Agent this handle is scoped to
    reader: WebID,
    /// Maximum number of triples to return in a single query
    query_budget: u32,
}

impl SemanticReadHandle {
    /// Create a test handle with synthetic values.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            reader: WebID::new(),
            query_budget: 100,
        }
    }

    /// Create a semantic read handle for a specific agent.
    pub fn new(reader: WebID, query_budget: u32) -> Self {
        Self {
            reader,
            query_budget,
        }
    }

    /// The agent this handle is scoped to.
    pub fn reader(&self) -> &WebID {
        &self.reader
    }

    /// Maximum number of triples returnable in a single query.
    pub fn query_budget(&self) -> u32 {
        self.query_budget
    }

    /// Check if this handle can read data from the given category.
    ///
    /// Semantic read handles can access `SemanticMemory` and `Public` data.
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

/// Semantic memory write handle.
///
/// Provides write access to the shared semantic memory. Agents with
/// this handle can store new semantic triples, including consolidating
/// knowledge from episodic memory (perspective-stripped, deduplicated).
///
/// # OCAP Boundaries
///
/// - **CAN** store semantic triples (with consolidation capability)
/// - **CAN** store embeddings for semantic indexing
/// - **CAN** combine confidence from multiple sources (RECONCILE)
/// - **CANNOT** delete triples (confidence retraction only via governance)
/// - **CANNOT** access episodic memories (use `EpisodicReadHandle`)
/// - **CANNOT** write on behalf of other agents (WebID binding)
pub struct SemanticWriteHandle {
    /// Agent this handle is scoped to
    writer: WebID,
    /// Whether this handle has consolidation capability
    /// (can strip perspective from episodic triples and promote to semantic)
    can_consolidate: bool,
    /// Per-entity storage budget
    storage_budget: u32,
}

impl SemanticWriteHandle {
    /// Create a test handle with synthetic values.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            writer: WebID::new(),
            can_consolidate: true,
            storage_budget: 10000,
        }
    }

    /// Create a semantic write handle for a specific agent.
    pub fn new(writer: WebID, can_consolidate: bool, storage_budget: u32) -> Self {
        Self {
            writer,
            can_consolidate,
            storage_budget,
        }
    }

    /// The agent this handle is scoped to.
    pub fn writer(&self) -> &WebID {
        &self.writer
    }

    /// Whether this handle can perform consolidation (perspective stripping + dedup).
    pub fn can_consolidate(&self) -> bool {
        self.can_consolidate
    }

    /// Per-entity storage budget.
    pub fn storage_budget(&self) -> u32 {
        self.storage_budget
    }

    /// Check if this handle can write data in the given category.
    ///
    /// Semantic write handles can write to `SemanticMemory` data.
    pub fn can_write(&self, category: &DataCategory) -> bool {
        matches!(category, DataCategory::SemanticMemory)
    }
}

/// Regulation interface for the Semantic Memory Loop.
///
/// The Cybernetics Loop uses this to throttle semantic indexing
/// when embedding costs exceed energy budgets.
pub trait SemanticRegulation: Send + Sync {
    /// Throttle semantic indexing rate.
    fn throttle_indexing(&self, reason: &str);

    /// Adjust the per-entity storage budget.
    fn adjust_storage_budget(&self, new_budget: u32);
}
