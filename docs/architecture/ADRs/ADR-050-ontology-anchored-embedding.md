# ADR-050: Ontology-Anchored Embedding (Tag → Embed)

**Date:** 2026-07-15
**Status:** Accepted
**Decider:** User (mdz-axolotl)
**Supersedes:** None
**Superseded by:** None

## Context

The corpus pipeline processes 128 source files (PDF + HTML + MD) through a
sequence of MCP tools: convert → chunk → tag → embed → extract_triples →
dedup → consolidate → build_prompts → generate_qa → ingest_qa → train.

The original pipeline order ran **embed before tag** (v7.0). This created
raw-text embeddings with no ontology awareness. The question arose: should
embeddings inform tagging (embed → tag with KNN context) or should tagging
inform embeddings (tag → embed with ontology annotations)?

## Research Findings

Academic research consistently supports **tag → embed** (ontology informs
embedding, not the reverse):

### 1. Instruction-tuned embeddings (INSTRUCTOR, Su et al., NeurIPS 2023)

INSTRUCTOR prepends task-specific instructions to text before embedding,
producing task-conditioned vectors that outperform generic embeddings across
70 evaluation tasks. Our analog: prepend `[golem: metaphor, narrative]` to
chunk text before embedding, so the vector encodes both content and ontology.

### 2. Ontology-enhanced KG embeddings (Wang et al., ICCPR 2023)

Incorporating ontology information (entity types, class hierarchies) into
embeddings consistently outperforms baseline embeddings. The ontology acts as
a structured prior that shapes the embedding space.

### 3. Ontology-driven text classification (iieta.org, 2024)

Ontology-based classification outperforms keyword-based. The ontology provides
a semantic framework that guides classification — the LLM classifies against
GOLEM, FIBO, ESO, PKO rather than free-form.

### 4. Ontology-guided KG construction (OntoKG, arXiv:2604.02618)

Ontology schemas guide LLM-based extraction, producing higher quality KGs
than unguided extraction. One-time classification cost, multiple downstream
benefits.

### 5. RAG with ontology-guided KGs (arXiv:2511.05991)

Ontology-guided KGs built from relational databases perform competitively to
text-extracted ones, with the benefit of a one-time-only ontology learning
process that substantially reduces LLM usage costs.

## Decision

**Adopt tag → embed ordering.** The pipeline now runs:

```
chunk → tag → embed → extract_triples → dedup → consolidate → build_prompts → ...
```

### Changes

1. **`docproc_tag_chunks`** — runs BEFORE embed. Classifies each chunk
   against GOLEM, FIBO, ESO, PKO, OMC, Dublin Core ontologies. No KNN context
   needed — the passage text alone is sufficient for ontology classification.

2. **`docproc_embed`** — runs AFTER tag. Accepts optional `tagged_jsonl`
   parameter. When provided, reads ontology tags and prepends them as
   annotations to chunk text before embedding:
   ```
   [golem: metaphor, narrative structure | pko: analysis] <chunk text>
   ```
   This produces ontology-anchored embeddings per INSTRUCTOR (Su et al. 2023).

3. **Pipeline YAML** — reordered: `tag_chunks` before `embed_chunks`.
   `embed_chunks` now receives `tagged_jsonl: "corpus/chunks/tagged_ontology.jsonl"`.

### Benefits

| Benefit | Mechanism |
|---------|-----------|
| Better KNN retrieval in build_prompts | Ontology-anchored embeddings cluster narrative with narrative, financial with financial |
| Better dedup (cosine similarity) | Near-identical passages with same ontology tags are more similar than those without |
| Better consolidate (clustering) | Clustering is ontology-aware — related passages in the same domain cluster together |
| One-time classification cost | Tag once, all downstream steps benefit from the classification |
| Consistent with research | Supported by INSTRUCTOR, ontology-enhanced KG embeddings, and instruction-tuned embedding literature |

### What we rejected and why

**Embed → tag with KNN (Option A)** was rejected because:
- The embedding is "dumb" (raw text only) and the tagging does all the semantic work
- KNN context adds noise to the tagging task (classification doesn't need context)
- Bad embeddings → bad KNN → misleading context → wrong tags (contamination cascade)
- Tagging is a classification task, not a generation task — RAG paradigm doesn't apply

**Iterative/multi-pass (Option C)** was rejected because:
- Doubles LLM cost for tagging
- No guarantee of convergence
- Diminishing returns after first iteration
- The first pass (tag → embed) already captures most of the benefit

## Consequences

- **Pipeline order is strictly sequential** — tag must complete before embed
  starts. They can no longer run in parallel.
- **Embedding cost is unchanged** — the annotation prefix adds ~20-30 tokens
  per chunk, which is negligible for the embedding API.
- **Tagging cost is unchanged** — the tagger works from raw text, same as before.
- **Downstream KNN quality should improve** — embeddings now encode ontology,
  so similar passages in the same domain cluster together.
- **The extract_triples step** also reads `tagged_jsonl` for ontology injection
  into the extraction prompt. This was already implemented and is unchanged.