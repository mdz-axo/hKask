---
title: "hkask-memory — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "e17e69e2"
---

# hkask-memory — API Reference

Semantic and episodic memory pipelines for the hKask agent platform. Implements a two-layer DRY deduplication system: Layer 1 memory recall dedup (entity-attribute-value hash), Layer 2 prompt assembly dedup (in `hkask-templates/src/context_assembly.rs`).

## Public Modules

| Module | Description |
|---|---|
| `consolidation` | Episodic → Semantic bridge. Type: `ConsolidationBridge` |
| `consolidation_auth` | Consolidation authorization, re-exported via `pub use consolidation_auth::*` |
| `consolidation_service` | Consolidation service. Type: `ConsolidationService` |
| `episodic` | Episodic memory pipeline (Loop 2a): first-person experience storage. Types: `EpisodicMemory`, `EpisodicMemoryError` |
| `episodic_loop` | Episodic loop wrapper with budget regulation (Loop 2a). Type: `EpisodicLoop` |
| `error` | Memory port error. Type: `MemoryPortError` |
| `ports` | Port trait re-exports from `hkask_agents::ports`: `EpisodicStoragePort`, `SemanticStoragePort`, `RecallRequest`, `RecalledEpisode`, `RecalledSemantic`, `StorageRequest` |
| `ranking` | Memory ranking algorithms |
| `recall_dedup` | Layer 1 dedup: entity-attribute-value hash-based recall deduplication |
| `salience` | Salience scoring for memory importance |
| `semantic` | Semantic memory pipeline (Loop 2b): shared knowledge graph. Types: `SemanticMemory`, `SemanticMemoryError` |
| `semantic_loop` | Semantic loop wrapper with regulatory triggers (Loop 2b). Type: `SemanticLoop` |

## Key Public Types

### `EpisodicMemory`

First-person experience memory (Loop 2a). Provides subloops:

- **2a.1 Experience Encoding (FILTER):** Filter and classify incoming experience
- **2a.2 Temporal Attention (ADAPT):** Weight by recency: `weight = e^(-λ × time_since_storage)`
- **2a.3 Confidence Decay (RECONCILE):** Confidence decreases over time via Bayesian decay. Uses the Wozniak-Gorzelanczyk (1995) human forgetting curve: `R(t) = exp(-t/S)` where S is memory life in days (default: `DEFAULT_MEMORY_LIFE_DAYS`)
- **2a.4 Episodic Storage Budget (GUARD):** Per-agent storage limit (default: `DEFAULT_EPISODIC_BUDGET = 10_000`). Marks oldest h_mems for consolidation when budget is exceeded
- **2a.5 Episodic Context Assembly (FILTER+ADAPT):** Temporal-ordered, recency-weighted, budget-constrained

Requires a perspective (agent WebID). Backed by `HMemStore` and uses `recall_dedup` for entity-attribute-value hash deduplication.

### `EpisodicMemoryError`

Error enum: `HMem(HMemError)`, `InvalidVisibility(String)`, `MissingPerspective`.

### `SemanticMemory`

Shared knowledge graph memory (Loop 2b). Provides subloops:

- **Storage Budget (6e):** Per-entity storage limit with deletion of lowest-confidence h_mems when budget is exceeded
- **Similarity-Augmented Recall:** KNN search over embeddings to find semantically related h_mems, enabling context assembly beyond exact entity matches
- **Corpus Centroid:** Mean embedding vector for style cluster validation (returns `CentroidResult`)
- **Prefix Purge:** Idempotent re-ingest by deleting embeddings matching a prefix

Requires no perspective (use `ConsolidationBridge` for episodic → semantic promotion). Backed by `HMemStore` and `EmbeddingStore`.

### `SemanticMemoryError`

Error enum: `HMem(HMemError)`, `Embedding(EmbeddingError)`, `InvalidVisibility(String)`, `NoEmbeddingsForCentroid(String)`, `HasPerspective`.

### `CentroidResult`

Result of computing a style centroid from semantic embeddings.

**Fields:** `centroid: Vec<f32>`, `passage_count: usize`, `stored: bool`.

### `ConsolidationBridge`

One-way episodic → semantic consolidation bridge. When currency pressure triggers consolidation:

1. Select candidates via `EpisodicMemory::consolidation_candidates()` (oldest, lowest effective confidence)
2. Strip perspective (privacy boundary removal)
3. Check against existing semantic h_mems with same EAV:
   - **Match found:** Bayesian combine confidences, update existing
   - **No match:** Seed as new semantic h_mem
4. Expire in episodic memory (soft-delete via valid_to)

**Constructor:** `ConsolidationBridge::new(episodic: Arc<EpisodicMemory>, semantic: Arc<SemanticMemory>) -> Self`

**Method:** `consolidate(&self, perspective: WebID, request: ConsolidationRequest) -> Result<ConsolidationOutcome, String>`

This is a ONE-WAY operation: Episodic → Semantic. No reverse flow.

### `ConsolidationService`

Higher-level consolidation service for scheduled/triggered consolidation.

### `EpisodicLoop`

Loop wrapper for `EpisodicMemory` with budget regulation (Loop 2a). Monitors episodic storage usage against budget and enforces limits.

**Fields:** `memory: Arc<EpisodicMemory>`, `perspective: WebID`, `storage_budget: usize`, `consolidation: Option<Arc<ConsolidationBridge>>`.

**Constructor:** `EpisodicLoop::new(memory: Arc<EpisodicMemory>, perspective: WebID, storage_budget: usize) -> Self`

**Behavior:** When usage exceeds 80% of budget → `Throttle` actions targeting itself. When usage exceeds 100% → escalates to Curation loop and consolidates lowest-confidence episodic h_mems to semantic memory.

Implements `HkaskLoop` from `hkask_cns::types::loops`.

### `SemanticLoop`

Loop wrapper for `SemanticMemory` with two regulatory triggers (Loop 2b).

**Constants:**
- `DEFAULT_SEMANTIC_STORAGE_BUDGET`: 25,000 (max h_mem count)
- `DEFAULT_LOW_CONFIDENCE_THRESHOLD`: 0.33 (33% — h_mems at/below this are pruned)
- `DEFAULT_CONDENSATION_WINDOW_DAYS`: 30 (h_mems older than this are condensation candidates)
- `CONDENSED_SUMMARY_CONFIDENCE`: 0.6 (60% — confidence for condensed summary h_mems, lower than directly observed facts at 1.0 but higher than the low-confidence threshold)

**Regulatory triggers:**
1. **Storage budget:** when h_mem count exceeds configurable budget (default 25,000), delete lowest-confidence h_mems
2. **Consolidation trigger:** review and delete semantic h_mems with confidence ≤ threshold (default 0.33)

Implements `HkaskLoop` from `hkask_cns::types::loops`.

### `MemoryPortError`

Error type for memory port operations.

## Port Traits (from `ports` module)

Re-exports from `hkask_agents::ports`:

- `EpisodicStoragePort` — port for episodic memory storage operations
- `SemanticStoragePort` — port for semantic memory storage operations
- `RecallRequest` — memory recall request parameters
- `RecalledEpisode` — recalled episodic memory entry
- `RecalledSemantic` — recalled semantic memory entry
- `StorageRequest` — storage operation request

## Re-exports from Crate Root

`ConsolidationBridge`, `ConsolidationService`, `EpisodicMemory`, `EpisodicMemoryError`, `EpisodicLoop`, `MemoryPortError`, `EpisodicStoragePort`, `RecallRequest`, `RecalledEpisode`, `RecalledSemantic`, `SemanticStoragePort`, `StorageRequest`, `SemanticMemory`, `SemanticMemoryError`, `SemanticLoop`. All symbols from `consolidation_auth` are re-exported via glob.
