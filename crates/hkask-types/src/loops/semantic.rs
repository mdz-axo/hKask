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
//! Semantic memory is SHARED across agents.
