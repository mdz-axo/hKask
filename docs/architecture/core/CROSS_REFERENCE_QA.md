---
title: "Cross-Reference QA Generation — Design Note"
audience: [developers, researchers]
last_updated: 2026-07-12
version: "0.31.0"
status: "Active"
domain: "QA"
mds_categories: [domain, curation]
---

# Cross-Reference QA Generation — Design Note

**Added:** 2026-07-09 | **Flag:** `corpus-ingest build-prompts --cross-reference`  
**Research Basis:** RA-DIT (Lin et al., 2024), Self-RAG (Asai et al., 2023)

## Problem

Standard QA generation from individual text chunks produces shallow QAs that test recall of single passages. Investment reasoning requires synthesis across sources — comparing Damodaran's DCF to Fabozzi's RIM, diagnosing competitive position from Greenwald's framework applied to Porter's five forces. Single-chunk QAs cannot capture this.

## Solution

`build-prompts --cross-reference` groups tagged chunks by shared investment concepts, selects the top-N most salient chunks per concept group, and generates prompts that require the LLM to synthesize across multiple passages with explicit source citation.

### Algorithm

```
1. Group all qualifying chunks by shared concepts (HashMap<concept, Vec<chunk>>)
2. Filter to groups with 2+ chunks
3. Sort groups by: (a) chunk count descending, (b) max salience descending
4. For each group:
   a. Sort chunks by salience, take top-N (default: 3)
   b. Assign QA type by rotation: comparative → diagnostic → causal → applied
   c. Generate prompt with all passages + citation requirement
   d. Append to prompts.jsonl with `"cross_reference": true` marker
```

### Prompt Structure

The system prompt explicitly requires:
- Synthesis across multiple passages
- Source citation per claim ("Per Passage 1, ... while Passage 3 notes ...")
- Grounding in provided text, not invented facts
- QA types that inherently require multi-source reasoning: comparative (contrast perspectives), diagnostic (identify cross-source patterns/tensions), causal (trace idea connections), applied (multi-source diagnosis of a scenario)

### Traceability

Each cross-reference prompt records:
- `chunk_refs: ["corpus:book:damodaran:12", "corpus:book:fabozzi:45", ...]` — all source chunks
- `concept: "valuation_methods"` — the shared concept anchoring the synthesis
- `cross_reference: true` — flag distinguishing from single-chunk prompts

## Research Basis

**RA-DIT** (Lin et al., 2024, "RA-DIT: Retrieval-Augmented Dual Instruction Tuning") demonstrates that retrieval-augmented generation quality improves when the LLM is explicitly trained/fine-tuned to attend across multiple retrieved passages rather than treating them as independent context blocks. Cross-reference prompting implements this at the generation level — requiring the LLM to compare, contrast, and synthesize before generating.

**Self-RAG** (Asai et al., 2023, "Self-RAG: Learning to Retrieve, Generate, and Critique through Self-Reflection") shows that models trained to cite sources produce more factually grounded outputs. The mandatory citation requirement ("cite which passages") reduces hallucination.

**GraphRAG** (Microsoft Research, 2024) demonstrates that knowledge-graph-structured retrieval (our concept-grouped chunks) outperforms flat KNN for multi-hop reasoning tasks. Our concept grouping via `EntityTags` provides lightweight graph structure without a full knowledge graph.

## Usage

```bash
corpus-ingest build-prompts \
  --cross-reference \
  --cross-ref-max-chunks 3 \
  --cross-ref-max-prompts 500 \
  --min-salience 0.05 \
  --min-concepts 2
```

Expected output: ~500 cross-reference prompts appended to `corpus/qa_pairs/prompts.jsonl`, interleaved with standard single-chunk prompts. The LLM consuming these prompts generates QAs that test synthesis, not recall.
