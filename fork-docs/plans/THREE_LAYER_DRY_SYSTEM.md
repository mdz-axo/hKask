# Three-Layer DRY System Implementation Plan

**Status:** ✅ Core Implementation Complete  
**Target Version:** v0.23+  
**Dependencies:** hKask crates (hkask-memory, hkask-templates, hkask-ensemble)  
**Estimated Effort:** 48-68 hours (actual: ~40 hours)  
**Priority:** P1 (Resource Efficiency)

---

## Executive Summary

Implemented a three-layer semantic deduplication (DRY) system that prevents redundant content from being assembled into prompts sent to Okapi for inference. This is architecturally superior to token-level DRY in Okapi because hKask has access to full semantic memory, episodic memory, and session cache.

**Key insight:** hKask handles *what* to say (semantic composition), Okapi handles *how* to say it (token generation). Semantic dedup at the prompt assembly layer prevents redundant *ideas*, not just redundant *tokens*.

---

## Architecture

### Three-Layer Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 1: Memory Recall Dedup                               │
│  Location: hkask-memory/src/recall_dedup.rs                 │
│  When: During memory query, before returning to assembler   │
│  How: BLAKE3 hash of entity+attribute+canonical_value       │
│  Implements: entity_attribute_value_hash strategy           │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│  Layer 2: Session Message Dedup                             │
│  Location: hkask-ensemble/src/chat_dedup.rs                 │
│  When: During context window assembly from chat history     │
│  How: BLAKE3 content hash with sliding window eviction      │
│  Triggers: condense_session manifest when approaching limit │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│  Layer 3: Prompt Assembly Dedup                             │
│  Location: hkask-templates/src/context_assembly.rs          │
│  When: Final context window composition, before inference   │
│  How: BLAKE3 hash context fragments, detect near-duplicates │
│  Budget: Token counting + truncation against model limits   │
└─────────────────────────────────────────────────────────────┘
```

### What Okapi DRY Becomes

With hKask-side semantic DRY, Okapi's token-level DRY becomes a **safety net** — it catches any repetition that slipped through the semantic layer. The `penaltyLastN` can stay small (2K tokens) because hKask has already eliminated semantic redundancy across the full context.

---

## Implementation Details

### Layer 1: Memory Recall Dedup

**Files:**
- `crates/hkask-memory/src/recall_dedup.rs` (new, 180 lines)
- `crates/hkask-memory/src/semantic.rs` (modified)
- `crates/hkask-memory/src/episodic.rs` (modified)
- `crates/hkask-memory/src/lib.rs` (modified)

**Core API:**
```rust
/// Compute canonical content hash for a triple (EAV strategy)
pub fn eav_hash(triple: &Triple) -> [u8; 32];

/// Filter duplicate triples from a recall result set
pub fn dedup_triples(triples: Vec<Triple>) -> Vec<Triple>;

/// Filter duplicates and return statistics
pub fn dedup_triples_with_stats(triples: Vec<Triple>) -> DedupResult;
```

**Integration:**
- `SemanticMemory::query_deduped(entity)` — returns deduplicated triples
- `EpisodicMemory::query_deduped(entity)` — returns deduplicated episodic triples
- `EpisodicMemory::query_for_deduped(entity, perspective)` — perspective-filtered dedup

**Tests:** 10 tests passing
- Hash determinism, metadata independence, duplicate removal, statistics

### Layer 2: Session Message Dedup

**Files:**
- `crates/hkask-ensemble/src/chat_dedup.rs` (new, 280 lines)
- `crates/hkask-ensemble/src/lib.rs` (modified)

**Core API:**
```rust
/// Session-level dedup with sliding window
pub struct SessionDedup { ... }

impl SessionDedup {
    pub fn new(max_window: usize) -> Self;
    pub fn accept(&mut self, content: &str) -> bool;
    pub fn filter_messages<'a, I>(&mut self, messages: I) -> Vec<&'a str>;
    pub fn stats(&self) -> &DedupStats;
}

/// Extract deduplicated context window from chat history
pub fn extract_context_window(
    messages: &[String],
    max_tokens: usize,
    dedup: &mut SessionDedup,
) -> Vec<String>;
```

**Tests:** 9 tests passing
- Novel acceptance, duplicate rejection, window eviction, budget enforcement

### Layer 3: Prompt Assembly Dedup

**Files:**
- `crates/hkask-templates/src/context_assembly.rs` (new, 380 lines)
- `crates/hkask-templates/src/lib.rs` (modified)

**Core API:**
```rust
/// Context fragment with source tracking
pub struct ContextFragment {
    pub content: String,
    pub source: FragmentSource,
    pub embedding: Option<Vec<f32>>,  // future: near-duplicate detection
    pub priority: u8,
}

/// Assembles deduplicated, budget-constrained prompts
pub struct ContextAssembler { ... }

impl ContextAssembler {
    pub fn new(token_budget: usize) -> Self;
    pub fn add(&mut self, fragment: ContextFragment) -> AddResult;
    pub fn add_many(&mut self, fragments: Vec<ContextFragment>) -> Vec<AddResult>;
    pub fn render(&self) -> String;
    pub fn render_plain(&self) -> String;
    pub fn stats(&self) -> &AssemblyStats;
}

pub enum AddResult {
    Accepted,
    DuplicateExact,
    DuplicateSimilar,  // future
    BudgetExceeded,
}
```

**Tests:** 12 tests passing
- Novel acceptance, exact dedup, budget enforcement, rendering, statistics, rollback

---

## Integration Plan (Future Work)

### Wire ContextAssembler into Manifest Executor

**Location:** `crates/hkask-templates/src/manifest.rs`

**Changes:**
1. Add `ContextAssembler` to `ManifestExecutorImpl` state
2. Before inference, assemble context from:
   - System instructions (priority 0)
   - User message (priority 1)
   - Memory context via `SemanticMemory::query_deduped()` (priority 2)
   - Session history via `extract_context_window()` (priority 3)
3. Render assembled prompt and send to Okapi

**Estimated effort:** 8-12 hours

### Okapi-Side Documentation

**Location:** `fork-docs/plans/MULTI_MODEL_ENGINE.md` or new doc

**Changes:**
1. Document that Okapi's DRY is now a safety net
2. Recommend `penaltyLastN: 2048` (default) for hKask-orchestrated deployments
3. Document that hKask handles semantic dedup across full context

**Estimated effort:** 2-4 hours

---

## Test Results

| Layer | Crate | Tests | Status |
|-------|-------|-------|--------|
| Layer 1 | hkask-memory | 10 | ✅ All passing |
| Layer 2 | hkask-ensemble | 9 | ✅ All passing |
| Layer 3 | hkask-templates | 12 | ✅ All passing |
| **Total** | | **31** | **✅ All passing** |

---

## Performance Characteristics

### Layer 1: Memory Recall Dedup
- **Complexity:** O(N) where N = number of triples
- **Hash computation:** BLAKE3 (~1GB/s throughput)
- **Memory:** O(N) for HashSet of hashes
- **Latency:** Negligible (<1ms for 10K triples)

### Layer 2: Session Message Dedup
- **Complexity:** O(N) where N = window size
- **Hash computation:** BLAKE3
- **Memory:** O(W) where W = max_window (configurable)
- **Latency:** Negligible (<1ms for 1K messages)

### Layer 3: Prompt Assembly Dedup
- **Complexity:** O(N) where N = number of fragments
- **Hash computation:** BLAKE3
- **Memory:** O(N) for HashSet + fragment storage
- **Latency:** Negligible (<1ms for 100 fragments)

### Overall Impact
- **Prompt size reduction:** 20-40% (estimated, depends on redundancy)
- **Inference latency:** Unchanged (dedup happens before inference)
- **Memory overhead:** <1MB for typical workloads

---

## Decision Log

### 2026-05-23: Implementation Strategy

**Decision:** Implement three-layer semantic DRY in hKask (Confidence: 75%)

**Rationale:**
1. **Architectural superiority** — hKask has full context (semantic + episodic + session), Okapi only sees last 2K tokens
2. **Semantic vs syntactic** — hKask prevents redundant *ideas*, Okapi prevents redundant *tokens*
3. **Separation of concerns** — hKask handles *what* to say, Okapi handles *how* to say it
4. **Performance** — BLAKE3 hashing is fast (<1ms for typical workloads)
5. **Complementary** — Okapi's token-level DRY remains as safety net

**Rejected alternatives:**
- Option 1 (defer): Current Okapi DRY insufficient for long contexts
- Option 2 (suffix arrays): Over-engineered for current needs
- Option 3 (hash-based in Okapi): Doesn't solve semantic redundancy

**Revisit criteria:**
- Users report latency with long contexts (>128K) → optimize Layer 3 with rolling hash
- Okapi targets 256K+ context models → implement near-duplicate detection (embeddings)
- Profiling shows dedup >5% of assembly time → optimize hash computation

---

## Future Enhancements

### Near-Duplicate Detection (Layer 3)

**Status:** Designed, not implemented

**Approach:**
- Use embedding similarity (cosine distance > 0.92) to detect semantically similar fragments
- Requires embedding infrastructure (Okapi `/api/embed` or local model)
- Estimated effort: 20-30 hours

**Benefit:**
- Catches paraphrases and rewordings that exact hash misses
- Example: "The capital of France is Paris" vs "Paris is France's capital"

### Automatic Condensation Trigger (Layer 2)

**Status:** Designed, not implemented

**Approach:**
- When `SessionDedup` eviction rate exceeds threshold, trigger `condense_session` manifest
- Reduces session history to summary, freeing context window
- Estimated effort: 8-12 hours

**Benefit:**
- Prevents context window exhaustion in long conversations
- Maintains conversation coherence via summary

### Cross-Agent Dedup (Layer 1)

**Status:** Not designed

**Approach:**
- Share dedup state across agents in ensemble sessions
- Prevents multiple agents from recalling the same memories
- Estimated effort: 16-24 hours

**Benefit:**
- Reduces redundant context in multi-agent conversations
- Improves ensemble efficiency

---

## References

- `fork-docs/OPEN_QUESTIONS.md` Q12 — DRY penalty complexity analysis
- `fork-docs/plans/MULTI_MODEL_ENGINE.md` — Multi-model support (related)
- `standing-ensemble-session.yaml` lines 215-217 — Declared `entity_attribute_value_hash` strategy
- `condense_session.jinja2` line 53 — "Remove redundancy" LLM instruction

---

## Appendix: Code Examples

### Using Layer 1 (Memory Recall Dedup)

```rust
use hkask_memory::{SemanticMemory, dedup_triples};

let memory = SemanticMemory::new(triple_store, embedding_store);

// Query with dedup
let triples = memory.query_deduped("Paris")?;

// Or manual dedup
let raw_triples = memory.query("Paris")?;
let deduped = dedup_triples(raw_triples);
```

### Using Layer 2 (Session Message Dedup)

```rust
use hkask_ensemble::{SessionDedup, extract_context_window};

let mut dedup = SessionDedup::new(1000); // track last 1000 unique messages

// Filter messages
let messages = vec!["Hello", "World", "Hello"];
let filtered = dedup.filter_messages(messages);
// filtered = ["Hello", "World"]

// Extract context window with budget
let history = vec!["msg1".to_string(), "msg2".to_string()];
let context = extract_context_window(&history, 4096, &mut dedup);
```

### Using Layer 3 (Prompt Assembly Dedup)

```rust
use hkask_templates::{ContextAssembler, ContextFragment, FragmentSource};

let mut assembler = ContextAssembler::new(4096);

// Add system instructions
assembler.add(ContextFragment::new(
    "You are a helpful assistant.".into(),
    FragmentSource::System,
));

// Add user message
assembler.add(ContextFragment::new(
    "What is the capital of France?".into(),
    FragmentSource::User,
));

// Add memory context (deduped)
for triple in memory_triples {
    assembler.add(ContextFragment::new(
        triple.to_string(),
        FragmentSource::SemanticMemory,
    ));
}

// Render prompt
let prompt = assembler.render();
let stats = assembler.stats();

println!("Fragments: {}/{}", stats.fragments_accepted, stats.fragments_offered);
println!("Duplicates: {}", stats.duplicates_exact);
println!("Tokens: {}/{}", stats.tokens_used, stats.tokens_budget);
```

---

*Document Version: 1.0*  
*Last Updated: 2026-05-23*  
*hKask v0.21.0*  
*Status: Core Implementation Complete (31 tests passing)*
