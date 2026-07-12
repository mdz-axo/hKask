---
title: "Embedding Architecture — QA Pipeline"
audience: [developers, researchers]
last_updated: 2026-07-12
version: "0.31.0"
status: "Active"
domain: "QA"
mds_categories: [domain, curation]
---

# Embedding Architecture — QA Pipeline

**Date:** 2026-07-10 | **Model:** Qwen3-Embedding-0.6B (1024-dim)

## Current State

| Component | Uses Embeddings? | How |
|-----------|-----------------|-----|
| `corpus_embed` | ✅ Produces | Embeds 20K+ chunks → EmbeddingStore |
| `corpus_salience` | ❌ No | Graph-centrality only |
| `build-prompts` | ❌ No | Salience + concepts only |
| `generate-qa` | ❌ No | Raw text → LLM |
| `ingest-qa` | ✅ Produces | Embeds QAs AFTER generation |
| `replica_build` | ✅ Produces | Embeds corpus for persona centroids |

## Gap: Embeddings Not Used in QA Generation

| Opportunity | Impact | Difficulty |
|-------------|--------|------------|
| **Chunk dedup** (cosine >0.95) | −15% inference cost | Medium — needs DB access in build-prompts |
| **MMR selection** (salience + diversity) | Fewer redundant QAs | Medium — needs vec0 KNN queries |
| **Semantic cross-ref** (embedding groups) | Better synthesis QAs | Hard — O(n²) without index |
| **QA-chunk alignment** (cosine validation) | Catch hallucinations | Easy — already have both embeddings |

## Why Keep Qwen3-Embedding-0.6B

| Factor | Analysis |
|--------|----------|
| MTEB retrieval | ~60% (adequate for dedup at 0.95 threshold) |
| Upgrade cost | Re-embed 20K chunks + 7K QAs → 4+ hours |
| Dim compatibility | 1024 matches EMBEDDING_DIM hardcoded everywhere |
| Academic consensus | Simple embeddings beat complex methods for data selection (Large-Scale Data Selection, 2025) |
| DEITA threshold | 0.9-0.95 for Repr Filter on OpenHermes/Tulu3 pools |

## Upgrade Path (when justified)

1. **Phase 1** (now): Use existing embeddings for concept coverage validation ✅
2. **Phase 2** (next): Add `--use-embeddings` flag to build-prompts for MMR selection
3. **Phase 3** (future): Evaluate bge-large-en-v1.5 vs Qwen3-Embedding-0.6B on investment lit
4. **Phase 4** (future): QA-chunk alignment validation in ingest-qa

## Ontological Anchoring

The `concepts` field in tagged_chunks.jsonl serves as a lightweight investment ontology:
- Competitive positioning: `competitive advantage`, `moat`, `barriers to entry`
- Valuation: `discounted cash flow`, `DCF`, `multiples`, `intrinsic value`
- Return analysis: `return on capital`, `ROIC`, `ROE`, `economic profit`
- Risk: `margin of safety`, `cost of capital`, `beta`, `uncertainty`
- Strategy: `capital allocation`, `reinvestment`, `growth`, `management quality`

Concept coverage is validated in the pipeline selection step — flags if critical concepts are missing from training QAs.
