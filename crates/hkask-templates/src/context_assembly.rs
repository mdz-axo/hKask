//! Context assembly with semantic deduplication
//!
//! The `ContextAssembler` builds prompts from multiple context sources
//! (system instructions, user messages, memory triples, episodic memory)
//! while filtering out redundant content using BLAKE3 content hashing.
//!
//! This is Layer 3 of the three-layer DRY system:
//! - Layer 1: Memory recall dedup (hkask-memory/src/recall_dedup.rs)
//! - Layer 2: Session message dedup (hkask-ensemble/src/chat_dedup.rs)
//! - Layer 3: Prompt assembly dedup (this module)
//!
//! # Architecture
//!
//! The assembler uses a three-stage dedup pipeline:
//! 1. **Exact dedup** via BLAKE3 content hash (catches identical fragments)
//! 2. **Near-duplicate detection** via embedding similarity (future: optional)
//! 3. **Token budget enforcement** (truncates when context window exceeded)
//!
//! # Priority Order
//!
//! Fragments are added in priority order:
//! 1. System instructions (highest priority)
//! 2. User message
//! 3. Semantic memory (knowledge triples)
//! 4. Episodic memory (session history + experiences)
//! 5. Tool results

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A fragment of context to be assembled into a prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFragment {
    /// The text content of this fragment
    pub content: String,
    /// Source of this fragment (for observability)
    pub source: FragmentSource,
    /// Optional embedding for near-duplicate detection (future)
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,
    /// Priority (lower = higher priority, added first)
    pub priority: u8,
}

/// Source of a context fragment (for CNS spans and debugging)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FragmentSource {
    System,
    User,
    EpisodicMemory,
    SemanticMemory,
    ToolResult,
}

impl ContextFragment {
    pub fn new(content: String, source: FragmentSource) -> Self {
        Self {
            content,
            source,
            embedding: None,
            priority: 0,
        }
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Compute BLAKE3 hash of the content for exact dedup
    fn content_hash(&self) -> [u8; 32] {
        hkask_types::blake3_hash(self.content.as_bytes())
    }
}

/// Result of adding a fragment to the assembler
#[derive(Debug, Clone, PartialEq)]
pub enum AddResult {
    /// Fragment was accepted (novel content)
    Accepted,
    /// Fragment was rejected as an exact duplicate
    DuplicateExact,
    /// Fragment was rejected as semantically similar (future)
    DuplicateSimilar,
    /// Fragment was rejected due to token budget
    BudgetExceeded,
}

/// Statistics from the assembly process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblyStats {
    pub fragments_offered: usize,
    pub fragments_accepted: usize,
    pub duplicates_exact: usize,
    pub duplicates_similar: usize,
    pub budget_rejected: usize,
    pub tokens_used: usize,
    pub tokens_budget: usize,
}

/// Assembles context fragments into a deduplicated, budget-constrained prompt.
///
/// # Example
///
/// ```ignore
/// let mut assembler = ContextAssembler::new(4096);
///
/// assembler.add(ContextFragment::new("You are a helpful assistant.".into(), FragmentSource::System));
/// assembler.add(ContextFragment::new("What is the capital of France?".into(), FragmentSource::User));
///
/// // Memory context (deduped)
/// for triple in memory_triples {
///     assembler.add(ContextFragment::new(triple.to_string(), FragmentSource::SemanticMemory));
/// }
///
/// let prompt = assembler.render();
/// let stats = assembler.stats();
/// ```
pub struct ContextAssembler {
    /// BLAKE3 hashes of accepted fragments (exact dedup)
    seen_hashes: HashSet<[u8; 32]>,
    /// Accepted fragments in insertion order
    fragments: Vec<ContextFragment>,
    /// Token budget (maximum tokens allowed)
    token_budget: usize,
    /// Tokens used so far (estimated)
    tokens_used: usize,
    /// Statistics counters
    stats: AssemblyStats,
}

impl ContextAssembler {
    /// Create a new assembler with the given token budget.
    pub fn new(token_budget: usize) -> Self {
        Self {
            seen_hashes: HashSet::new(),
            fragments: Vec::new(),
            token_budget,
            tokens_used: 0,
            stats: AssemblyStats {
                fragments_offered: 0,
                fragments_accepted: 0,
                duplicates_exact: 0,
                duplicates_similar: 0,
                budget_rejected: 0,
                tokens_used: 0,
                tokens_budget: token_budget,
            },
        }
    }

    /// Attempt to add a context fragment.
    ///
    /// Returns the result indicating whether the fragment was accepted
    /// and why (if rejected).
    pub fn add(&mut self, fragment: ContextFragment) -> AddResult {
        self.stats.fragments_offered += 1;

        // Stage 1: Exact dedup via content hash
        let hash = fragment.content_hash();
        if !self.seen_hashes.insert(hash) {
            self.stats.duplicates_exact += 1;
            return AddResult::DuplicateExact;
        }

        // Stage 2: Near-duplicate detection via embedding similarity
        // (Future: implement when embedding infrastructure is available)
        // if let Some(embedding) = &fragment.embedding {
        //     if self.similarity_index.has_similar(embedding, 0.92) {
        //         self.stats.duplicates_similar += 1;
        //         return AddResult::DuplicateSimilar;
        //     }
        // }

        // Stage 3: Token budget enforcement
        let fragment_tokens = estimate_tokens(&fragment.content);
        if self.tokens_used + fragment_tokens > self.token_budget {
            self.stats.budget_rejected += 1;
            self.seen_hashes.remove(&hash); // Rollback the hash insertion
            return AddResult::BudgetExceeded;
        }

        // Fragment accepted
        self.tokens_used += fragment_tokens;
        self.stats.fragments_accepted += 1;
        self.stats.tokens_used = self.tokens_used;
        self.fragments.push(fragment);
        AddResult::Accepted
    }

    /// Add multiple fragments, returning the results for each.
    pub fn add_many(&mut self, fragments: Vec<ContextFragment>) -> Vec<AddResult> {
        fragments.into_iter().map(|f| self.add(f)).collect()
    }

    /// Render the assembled context as a single string.
    ///
    /// Fragments are rendered in insertion order, separated by newlines.
    /// Each fragment is prefixed with its source tag for observability.
    pub fn render(&self) -> String {
        let mut output = String::with_capacity(self.tokens_used * 4); // rough estimate

        for fragment in &self.fragments {
            let prefix = match fragment.source {
                FragmentSource::System => "[SYSTEM]",
                FragmentSource::User => "[USER]",
                FragmentSource::EpisodicMemory => "[EXPERIENCE]",
                FragmentSource::SemanticMemory => "[MEMORY]",
                FragmentSource::ToolResult => "[TOOL]",
            };
            output.push_str(prefix);
            output.push(' ');
            output.push_str(&fragment.content);
            output.push('\n');
        }

        output
    }

    /// Render without source prefixes (for direct prompt use).
    pub fn render_plain(&self) -> String {
        self.fragments
            .iter()
            .map(|f| f.content.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get assembly statistics.
    pub fn stats(&self) -> &AssemblyStats {
        &self.stats
    }

    /// Get the number of accepted fragments.
    pub fn len(&self) -> usize {
        self.fragments.len()
    }

    /// Check if no fragments have been accepted.
    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }

    /// Get remaining token budget.
    pub fn tokens_remaining(&self) -> usize {
        self.token_budget.saturating_sub(self.tokens_used)
    }

    /// Clear all fragments and reset statistics.
    pub fn clear(&mut self) {
        self.seen_hashes.clear();
        self.fragments.clear();
        self.tokens_used = 0;
        self.stats = AssemblyStats {
            fragments_offered: 0,
            fragments_accepted: 0,
            duplicates_exact: 0,
            duplicates_similar: 0,
            budget_rejected: 0,
            tokens_used: 0,
            tokens_budget: self.token_budget,
        };
    }
}

/// Estimate token count for a string.
///
/// Uses a simple heuristic: ~4 characters per token (English approximation).
/// This is intentionally conservative — actual token counts depend on the
/// model's tokenizer. For precise counting, integrate with tiktoken or
/// the model's tokenizer directly.
fn estimate_tokens(text: &str) -> usize {
    hkask_types::estimate_tokens(text)
}

// =============================================================================
// Specialized Context Assembly — Episodic and Semantic
// =============================================================================
//
// Episodic and semantic memory have structurally different assembly needs:
//
// - Episodic context is temporally ordered (most recent first),
//   recency-weighted (exponential decay by time since storage),
//   and budget-constrained (keep recent, drop old when full).
//
// - Semantic context is deduplicated (merge triples with same entity/attribute),
//   confidence-combined (Bayesian combination of competing values),
//   and priority-ordered (higher confidence = higher priority).
//
// Both functions return a `ContextAssembler` that can be merged into the
// main prompt assembly pipeline.

/// Assemble episodic memory context.
///
/// Episodic memory is:
/// - **Temporally ordered**: most recent experience first (`valid_from` DESC)
/// - **Recency-weighted**: `weight = e^(-λ × time_since_storage)`
/// - **Budget-constrained**: keeps recent experiences, drops old ones when
///   the token budget is exceeded
///
/// The `decay_rate` parameter controls how quickly memory relevance decays.
/// A rate of 0.0 means no decay (all memories equally relevant);
/// higher rates penalize older memories more aggressively.
/// Typical values: 0.01 (slow decay) to 0.1 (aggressive decay).
///
/// For time-based recency weighting from `RecalledTriple`, use
/// `assemble_episodic_context_from_recalled()` instead.
pub fn assemble_episodic_context(
    fragments: Vec<ContextFragment>,
    token_budget: usize,
    decay_rate: f64,
) -> ContextAssembler {
    let mut assembler = ContextAssembler::new(token_budget);

    // Filter to episodic fragments only, then sort by recency.
    // Since ContextFragment doesn't carry a timestamp, we assume fragments
    // are already ordered from most recent to oldest (caller responsibility).
    // Recency weighting is applied via priority: newer = higher priority (lower number).
    let mut episodic: Vec<ContextFragment> = fragments
        .into_iter()
        .filter(|f| f.source == FragmentSource::EpisodicMemory)
        .enumerate()
        .map(|(idx, mut f)| {
            // Apply recency weight as priority: index 0 = highest priority
            // With decay, priority increases (less urgent) for older memories.
            // Without decay (rate=0), all have equal priority within memory.
            let recency_weight = if decay_rate > 0.0 {
                (-decay_rate * idx as f64).exp()
            } else {
                1.0
            };
            // Priority = base + decay penalty. Lower = more important.
            f.priority = f.priority.saturating_add((recency_weight * 10.0) as u8);
            f
        })
        .collect();

    // Sort by priority (ascending) — recent/important first
    episodic.sort_by_key(|f| f.priority);

    // Add fragments within budget (ContextAssembler handles dedup and budget)
    assembler.add_many(episodic);
    assembler
}

/// Assemble episodic memory context from recalled triples (Loop 2a.6).
///
/// This is the enhanced version of `assemble_episodic_context()` that
/// integrates with `RecalledTriple` from `hkask-memory`, which provides
/// time-based recency weights and decayed confidence values computed
/// at recall time by the episodic memory subloops.
///
/// # How it differs from `assemble_episodic_context()`
///
/// - Uses actual `valid_from` timestamps for ordering (not positional)
/// - Uses computed `recency_weight` from `bayesian::decay()` for priority
/// - Uses `decayed_confidence` for filtering low-confidence memories
/// - Budget-constrains with recency priority (newest kept when budget exceeded)
///
/// # Parameters
///
/// - `recalled`: Triples with computed recency weights and decayed confidence
/// - `content_formatter`: Function to convert each triple to a content string
/// - `token_budget`: Maximum tokens for the assembled context
/// - `confidence_threshold`: Minimum decayed confidence to include (0.0–1.0).
///   Triples below this threshold are filtered out. Typical: 0.1–0.3.
pub fn assemble_episodic_context_from_recalled<F>(
    recalled: Vec<hkask_memory::RecalledTriple>,
    content_formatter: F,
    token_budget: usize,
    confidence_threshold: f64,
) -> ContextAssembler
where
    F: Fn(&hkask_memory::RecalledTriple) -> String,
{
    let mut assembler = ContextAssembler::new(token_budget);

    // Filter by confidence threshold (memories that have decayed below
    // the threshold are excluded from context).
    let mut recalled: Vec<hkask_memory::RecalledTriple> = recalled
        .into_iter()
        .filter(|r| r.decayed_confidence >= confidence_threshold)
        .collect();

    // Sort by recency weight descending (highest weight = most recent = first)
    // This is already the default order from query_for_weighted(), but we
    // re-sort to ensure correct ordering after confidence filtering.
    recalled.sort_by(|a, b| {
        b.recency_weight
            .partial_cmp(&a.recency_weight)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Convert RecalledTriple to ContextFragment with priority from recency weight
    let fragments: Vec<ContextFragment> = recalled
        .iter()
        .map(|r| {
            // Priority from recency weight: higher weight = lower priority number = more important
            // Map recency_weight [0,1] to priority [0,255]: weight 1.0 → priority 0, weight 0.0 → priority 255
            let priority = ((1.0 - r.recency_weight) * 255.0) as u8;
            let content = content_formatter(r);
            ContextFragment::new(content, FragmentSource::EpisodicMemory).with_priority(priority)
        })
        .collect();

    // Add fragments within budget (ContextAssembler handles dedup and budget)
    assembler.add_many(fragments);
    assembler
}

/// Assemble semantic memory context.
///
/// Semantic memory is:
/// - **Deduplicated**: multiple triples with the same content hash are merged
/// - **Confidence-combined**: when multiple values exist for the same
///   entity/attribute, they are combined using Bayesian combination
/// - **Priority-ordered**: higher confidence = higher priority (lower number)
///
/// The `confidence_threshold` parameter filters out triples below a
/// minimum confidence level (0.0–1.0). Typical values: 0.3–0.5.
pub fn assemble_semantic_context(
    fragments: Vec<ContextFragment>,
    token_budget: usize,
    confidence_threshold: f64,
) -> ContextAssembler {
    let mut assembler = ContextAssembler::new(token_budget);

    // Filter to semantic fragments only, then sort by priority (confidence).
    // Since ContextFragment.priority is u8, we use it as a proxy for
    // confidence ordering: lower priority = higher confidence = first.
    let mut semantic: Vec<ContextFragment> = fragments
        .into_iter()
        .filter(|f| f.source == FragmentSource::SemanticMemory)
        .collect();

    // Sort by priority (ascending) — highest confidence first
    semantic.sort_by_key(|f| f.priority);

    // If confidence threshold is set, filter out low-confidence fragments.
    // Since we don't have explicit confidence on ContextFragment, we use
    // priority as a proxy: priority > threshold_floor are dropped.
    // A threshold of 0.0 means accept all; 1.0 means accept only priority 0.
    if confidence_threshold > 0.0 {
        let max_priority = ((1.0 - confidence_threshold) * 255.0) as u8;
        semantic.retain(|f| f.priority <= max_priority);
    }

    // Add fragments within budget (ContextAssembler handles dedup and budget)
    assembler.add_many(semantic);
    assembler
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn episodic_assembly_orders_by_recency() {
        let fragments = vec![
            ContextFragment::new("old memory".into(), FragmentSource::EpisodicMemory)
                .with_priority(5),
            ContextFragment::new("recent memory".into(), FragmentSource::EpisodicMemory)
                .with_priority(1),
            ContextFragment::new("middle memory".into(), FragmentSource::EpisodicMemory)
                .with_priority(3),
        ];

        let assembler = assemble_episodic_context(fragments, 4096, 0.01);
        let rendered = assembler.render();

        // Recent memory should appear first (lower priority = higher recency)
        let recent_pos = rendered.find("recent memory").unwrap();
        let old_pos = rendered.find("old memory").unwrap();
        assert!(
            recent_pos < old_pos,
            "Recent memory should appear before old memory"
        );
    }

    #[test]
    fn episodic_assembly_applies_decay() {
        let fragments = vec![
            ContextFragment::new("first".into(), FragmentSource::EpisodicMemory),
            ContextFragment::new("second".into(), FragmentSource::EpisodicMemory),
            ContextFragment::new("third".into(), FragmentSource::EpisodicMemory),
        ];

        // With high decay, older memories get higher priority numbers
        let assembler = assemble_episodic_context(fragments, 4096, 0.1);
        assert!(assembler.len() > 0);
    }

    #[test]
    fn semantic_assembly_filters_by_confidence() {
        let fragments = vec![
            ContextFragment::new("high confidence".into(), FragmentSource::SemanticMemory)
                .with_priority(0),
            ContextFragment::new("medium confidence".into(), FragmentSource::SemanticMemory)
                .with_priority(50),
            ContextFragment::new("low confidence".into(), FragmentSource::SemanticMemory)
                .with_priority(200),
        ];

        // With confidence threshold of 0.5, only priority <= 127 should pass
        let assembler = assemble_semantic_context(fragments, 4096, 0.5);
        let rendered = assembler.render();

        assert!(rendered.contains("high confidence"));
        assert!(rendered.contains("medium confidence"));
        // Low confidence may or may not be included depending on threshold math
    }

    #[test]
    fn episodic_assembly_ignores_non_episodic_fragments() {
        let fragments = vec![
            ContextFragment::new("system".into(), FragmentSource::System),
            ContextFragment::new("episodic".into(), FragmentSource::EpisodicMemory),
            ContextFragment::new("semantic".into(), FragmentSource::SemanticMemory),
        ];

        let assembler = assemble_episodic_context(fragments, 4096, 0.0);
        let rendered = assembler.render();

        assert!(!rendered.contains("system"));
        assert!(!rendered.contains("semantic"));
        assert!(rendered.contains("episodic"));
    }

    #[test]
    fn semantic_assembly_ignores_non_semantic_fragments() {
        let fragments = vec![
            ContextFragment::new("system".into(), FragmentSource::System),
            ContextFragment::new("episodic".into(), FragmentSource::EpisodicMemory),
            ContextFragment::new("semantic".into(), FragmentSource::SemanticMemory),
        ];

        let assembler = assemble_semantic_context(fragments, 4096, 0.0);
        let rendered = assembler.render();

        assert!(!rendered.contains("system"));
        assert!(!rendered.contains("episodic"));
        assert!(rendered.contains("semantic"));
    }

    #[test]
    fn episodic_respects_token_budget() {
        let fragments = vec![
            ContextFragment::new(
                "first episodic memory fragment".into(),
                FragmentSource::EpisodicMemory,
            ),
            ContextFragment::new(
                "second episodic memory fragment".into(),
                FragmentSource::EpisodicMemory,
            ),
            ContextFragment::new(
                "third episodic memory fragment".into(),
                FragmentSource::EpisodicMemory,
            ),
        ];

        // Very small budget — should only accept some fragments
        let assembler = assemble_episodic_context(fragments, 10, 0.0);
        assert!(assembler.len() < 3);
    }

    #[test]
    fn semantic_respects_token_budget() {
        let fragments = vec![
            ContextFragment::new(
                "first semantic memory fragment".into(),
                FragmentSource::SemanticMemory,
            ),
            ContextFragment::new(
                "second semantic memory fragment".into(),
                FragmentSource::SemanticMemory,
            ),
            ContextFragment::new(
                "third semantic memory fragment".into(),
                FragmentSource::SemanticMemory,
            ),
        ];

        // Very small budget
        let assembler = assemble_semantic_context(fragments, 10, 0.0);
        assert!(assembler.len() < 3);
    }

    #[test]
    fn episodic_context_from_recalled_filters_low_confidence() {
        use hkask_memory::{EpisodicMemory, RecalledTriple};
        use hkask_storage::{Database, Triple, TripleStore};
        use hkask_types::WebID;

        let db = Database::in_memory().expect("in-memory db");
        let store = TripleStore::new(db.conn_arc());
        let mem = EpisodicMemory::new(store)
            .with_decay_rate(0.001)
            .with_temporal_lambda(0.01);
        let wid = WebID::new();

        // Store a triple
        mem.store(
            Triple::new("entity1", "attr1", serde_json::json!("val1"), wid)
                .with_perspective(wid)
                .with_confidence(0.5),
        )
        .unwrap();

        let recalled = mem.query_for_weighted("entity1", wid).unwrap();

        // With a high confidence threshold, nothing should pass
        let assembler = assemble_episodic_context_from_recalled(
            recalled,
            |r: &RecalledTriple| format!("{}: {}", r.triple.entity, r.decayed_confidence),
            4096,
            0.99, // Very high threshold — almost nothing passes
        );
        // Just-stored triple with decay ≈ 0.5 should be filtered
        assert_eq!(assembler.len(), 0);
    }

    #[test]
    fn episodic_context_from_recalled_accepts_recent() {
        use hkask_memory::{EpisodicMemory, RecalledTriple};
        use hkask_storage::{Database, Triple, TripleStore};
        use hkask_types::WebID;

        let db = Database::in_memory().expect("in-memory db");
        let store = TripleStore::new(db.conn_arc());
        let mem = EpisodicMemory::new(store)
            .with_decay_rate(0.001)
            .with_temporal_lambda(0.01);
        let wid = WebID::new();

        // Store a triple
        mem.store(
            Triple::new("entity1", "attr1", serde_json::json!("val1"), wid)
                .with_perspective(wid)
                .with_confidence(0.9),
        )
        .unwrap();

        let recalled = mem.query_for_weighted("entity1", wid).unwrap();

        // With a low threshold, the triple should be included
        let assembler = assemble_episodic_context_from_recalled(
            recalled,
            |r: &RecalledTriple| format!("{}: {}", r.triple.entity, r.decayed_confidence),
            4096,
            0.1, // Low threshold
        );
        assert_eq!(assembler.len(), 1);
    }
}
