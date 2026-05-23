//! Context assembly with semantic deduplication
//!
//! The `ContextAssembler` builds prompts from multiple context sources
//! (system instructions, user messages, memory triples, session history)
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
//! 3. Memory context (semantic + episodic triples)
//! 4. Session history (most recent first)

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
    SemanticMemory,
    EpisodicMemory,
    SessionHistory,
    TemplateOutput,
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
        *blake3::hash(self.content.as_bytes()).as_bytes()
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
                FragmentSource::SemanticMemory => "[MEMORY]",
                FragmentSource::EpisodicMemory => "[EXPERIENCE]",
                FragmentSource::SessionHistory => "[HISTORY]",
                FragmentSource::TemplateOutput => "[TEMPLATE]",
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
    // Conservative estimate: 1 token per 4 characters
    // This overestimates for English, underestimates for CJK
    (text.len() + 3) / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assembler_accepts_novel_fragments() {
        let mut assembler = ContextAssembler::new(4096);

        let f1 = ContextFragment::new("Hello world".into(), FragmentSource::User);
        let f2 = ContextFragment::new("How are you?".into(), FragmentSource::User);

        assert_eq!(assembler.add(f1), AddResult::Accepted);
        assert_eq!(assembler.add(f2), AddResult::Accepted);
        assert_eq!(assembler.len(), 2);
    }

    #[test]
    fn test_assembler_rejects_exact_duplicates() {
        let mut assembler = ContextAssembler::new(4096);

        let f1 = ContextFragment::new("Hello world".into(), FragmentSource::User);
        let f2 = ContextFragment::new("Hello world".into(), FragmentSource::User);

        assert_eq!(assembler.add(f1), AddResult::Accepted);
        assert_eq!(assembler.add(f2), AddResult::DuplicateExact);
        assert_eq!(assembler.len(), 1);
    }

    #[test]
    fn test_assembler_enforces_token_budget() {
        let mut assembler = ContextAssembler::new(10); // 10 tokens = ~40 chars

        let f1 = ContextFragment::new("Short".into(), FragmentSource::User);
        let f2 = ContextFragment::new(
            "This is a very long fragment that will exceed the token budget".into(),
            FragmentSource::User,
        );

        assert_eq!(assembler.add(f1), AddResult::Accepted);
        assert_eq!(assembler.add(f2), AddResult::BudgetExceeded);
        assert_eq!(assembler.len(), 1);
    }

    #[test]
    fn test_assembler_render_with_prefixes() {
        let mut assembler = ContextAssembler::new(4096);

        assembler.add(ContextFragment::new(
            "You are helpful.".into(),
            FragmentSource::System,
        ));
        assembler.add(ContextFragment::new(
            "What is AI?".into(),
            FragmentSource::User,
        ));

        let output = assembler.render();
        assert!(output.contains("[SYSTEM] You are helpful."));
        assert!(output.contains("[USER] What is AI?"));
    }

    #[test]
    fn test_assembler_render_plain() {
        let mut assembler = ContextAssembler::new(4096);

        assembler.add(ContextFragment::new(
            "Line 1".into(),
            FragmentSource::System,
        ));
        assembler.add(ContextFragment::new("Line 2".into(), FragmentSource::User));

        let output = assembler.render_plain();
        assert_eq!(output, "Line 1\nLine 2");
    }

    #[test]
    fn test_assembler_stats() {
        let mut assembler = ContextAssembler::new(10);

        assembler.add(ContextFragment::new("Hello".into(), FragmentSource::User));
        assembler.add(ContextFragment::new("Hello".into(), FragmentSource::User)); // dup
        assembler.add(ContextFragment::new(
            "This is way too long for the budget".into(),
            FragmentSource::User,
        )); // budget

        let stats = assembler.stats();
        assert_eq!(stats.fragments_offered, 3);
        assert_eq!(stats.fragments_accepted, 1);
        assert_eq!(stats.duplicates_exact, 1);
        assert_eq!(stats.budget_rejected, 1);
    }

    #[test]
    fn test_assembler_clear() {
        let mut assembler = ContextAssembler::new(4096);
        assembler.add(ContextFragment::new("Hello".into(), FragmentSource::User));

        assert_eq!(assembler.len(), 1);
        assembler.clear();
        assert_eq!(assembler.len(), 0);
        assert!(assembler.is_empty());
    }

    #[test]
    fn test_assembler_tokens_remaining() {
        let mut assembler = ContextAssembler::new(100);
        assembler.add(ContextFragment::new("Hello".into(), FragmentSource::User)); // ~2 tokens

        let remaining = assembler.tokens_remaining();
        assert!(remaining > 90);
        assert!(remaining < 100);
    }

    #[test]
    fn test_assembler_add_many() {
        let mut assembler = ContextAssembler::new(4096);

        let fragments = vec![
            ContextFragment::new("One".into(), FragmentSource::User),
            ContextFragment::new("Two".into(), FragmentSource::User),
            ContextFragment::new("One".into(), FragmentSource::User), // dup
        ];

        let results = assembler.add_many(fragments);
        assert_eq!(
            results,
            vec![
                AddResult::Accepted,
                AddResult::Accepted,
                AddResult::DuplicateExact
            ]
        );
        assert_eq!(assembler.len(), 2);
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("a"), 1);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
        assert_eq!(estimate_tokens("Hello world"), 3); // 11 chars / 4 = 2.75 → 3
    }

    #[test]
    fn test_fragment_source_equality() {
        assert_eq!(FragmentSource::System, FragmentSource::System);
        assert_ne!(FragmentSource::System, FragmentSource::User);
    }

    #[test]
    fn test_budget_rollback_on_reject() {
        let mut assembler = ContextAssembler::new(10);

        // Add a fragment that fits
        assembler.add(ContextFragment::new("Hi".into(), FragmentSource::User));

        // Try to add one that doesn't fit
        assembler.add(ContextFragment::new(
            "This is too long for the remaining budget".into(),
            FragmentSource::User,
        ));

        // The hash should have been rolled back, so adding it again
        // should still be rejected for budget, not for duplicate
        let result = assembler.add(ContextFragment::new(
            "This is too long for the remaining budget".into(),
            FragmentSource::User,
        ));
        assert_eq!(result, AddResult::BudgetExceeded);
    }
}
